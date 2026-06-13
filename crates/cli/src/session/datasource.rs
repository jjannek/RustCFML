//! Datasource-backed session store (SQL).
//!
//! The fourth `SessionStore` impl, alongside `memory`, `memcached` and
//! `cluster`. Sessions are JSON-serialised — the same `serde_json` shape the
//! memcached store writes, so the `data` column is portable between the two —
//! and stored in one table keyed by `(cfid, app_name)`.
//!
//! ## Portability
//!
//! All SQL goes through `cfml_stdlib::builtins::fn_query_execute`, so every
//! bundled driver (SQLite, MySQL, PostgreSQL, MSSQL) is reachable. To stay
//! dialect-neutral the store avoids vendor-specific upsert syntax: `set`
//! does an `UPDATE`, and only `INSERT`s when the update touched no row
//! (last-write-wins, the same concurrency model the memcached store ships).
//! `take_expired` claims rows with a portable `SELECT` + per-row `DELETE`
//! rather than `DELETE ... RETURNING`, so the delete is the cross-node claim
//! and `onSessionEnd` does not double-fire.
//!
//! ## Schema (auto-created on first use)
//!
//! ```sql
//! CREATE TABLE IF NOT EXISTS cf_session_data (
//!     cfid        VARCHAR(255) NOT NULL,
//!     app_name    VARCHAR(255) NOT NULL,
//!     expires_at  BIGINT       NOT NULL,
//!     data        TEXT         NOT NULL,
//!     PRIMARY KEY (cfid, app_name)
//! );
//! ```
//!
//! If DDL is denied by the datasource grants, the first operation fails with
//! a clear error telling the operator to pre-create the documented schema.

use cfml_common::dynamic::CfmlValue;
use cfml_common::vm::CfmlResult;
use cfml_vm::{SessionData, session_store::SessionStore};
use indexmap::IndexMap;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Mutex, Once};

/// Default table name. Matches Lucee's `CF_SESSION_DATA` onboarding.
pub const DEFAULT_TABLE: &str = "cf_session_data";

pub struct DatasourceStore {
    /// Datasource name (resolved through the cfquery/queryExecute registry).
    datasource: String,
    /// Table name (validated to a safe identifier at construction).
    table: String,
    /// Logical application partition. v1 uses a single partition per store;
    /// multi-app isolation is achieved with distinct datasources/tables.
    app_name: String,
    /// One-shot schema bootstrap guard.
    schema_init: Once,
    /// Per-cfid `(expires_at, data_hash)` of the last value actually written,
    /// used to throttle the sliding-expiry touch write (issue #88): a request
    /// that changes nothing skips the DB round-trip until ~25% of the timeout
    /// has elapsed.
    touch_cache: Mutex<HashMap<String, (u64, u64)>>,
}

impl DatasourceStore {
    /// Construct a datasource session store. `table` is sanitised to a safe
    /// SQL identifier; `app_name` defaults to "default" when empty.
    pub fn new(datasource: &str, table: &str, app_name: &str) -> Self {
        let table = sanitize_identifier(table, DEFAULT_TABLE);
        let app_name = if app_name.trim().is_empty() {
            "default".to_string()
        } else {
            app_name.trim().to_string()
        };
        Self {
            datasource: datasource.to_string(),
            table,
            app_name,
            schema_init: Once::new(),
            touch_cache: Mutex::new(HashMap::new()),
        }
    }

    /// Run a statement through the shared query pipeline.
    fn run(&self, sql: &str, params: Vec<CfmlValue>, return_type: &str) -> CfmlResult {
        let mut opts = IndexMap::new();
        opts.insert(
            "datasource".to_string(),
            CfmlValue::string(self.datasource.clone()),
        );
        opts.insert(
            "returntype".to_string(),
            CfmlValue::string(return_type.to_string()),
        );
        cfml_stdlib::builtins::fn_query_execute(vec![
            CfmlValue::string(sql.to_string()),
            CfmlValue::array(params),
            CfmlValue::strukt(opts),
        ])
    }

    /// Idempotently create the backing table (and a cleanup index). The table
    /// is the hard requirement; the index is best-effort because some drivers
    /// (older MySQL) reject `CREATE INDEX IF NOT EXISTS`.
    fn ensure_schema(&self) {
        self.schema_init.call_once(|| {
            let ddl = format!(
                "CREATE TABLE IF NOT EXISTS {t} (\
                 cfid VARCHAR(255) NOT NULL, \
                 app_name VARCHAR(255) NOT NULL, \
                 expires_at BIGINT NOT NULL, \
                 data TEXT NOT NULL, \
                 PRIMARY KEY (cfid, app_name))",
                t = self.table
            );
            if let Err(e) = self.run(&ddl, vec![], "query") {
                eprintln!(
                    "[session/datasource] failed to create table '{}' on datasource '{}': {} \
                     — pre-create the documented cf_session_data schema and grant access, \
                     or session storage will not work",
                    self.table, self.datasource, e.message
                );
                return;
            }
            // Index for the expiry sweep. Best-effort: ignore failures.
            let idx = format!(
                "CREATE INDEX idx_{t}_expires ON {t} (expires_at)",
                t = self.table
            );
            let _ = self.run(&idx, vec![], "query");
        });
    }

}

/// Reduce a configured table name to a safe SQL identifier (letters, digits,
/// underscore). Falls back to `default` when the result would be empty. Table
/// names cannot be parameterised, so this guards against injection.
fn sanitize_identifier(name: &str, default: &str) -> String {
    let cleaned: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    if cleaned.is_empty() {
        default.to_string()
    } else {
        cleaned
    }
}

/// Compute the absolute expiry instant for a session record.
fn expires_at(data: &SessionData) -> u64 {
    data.last_accessed_secs.saturating_add(data.timeout_secs)
}

fn hash_str(s: &str) -> u64 {
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

/// Read the single `data` cell out of an `array`-returntype query result.
fn first_data_cell(result: CfmlResult) -> Option<String> {
    let val = result.ok()?;
    if let CfmlValue::Array(arr) = val {
        if let Some(CfmlValue::Struct(row)) = arr.snapshot().into_iter().next() {
            return row
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("data"))
                .map(|(_, v)| v.as_string());
        }
    }
    None
}

/// Extract `recordCount` from a mutation result struct.
fn affected_rows(result: &CfmlResult) -> i64 {
    let Ok(CfmlValue::Struct(m)) = result else {
        return 0;
    };
    let Some(v) = m
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("recordcount"))
        .map(|(_, v)| v.clone())
    else {
        return 0;
    };
    match v {
        CfmlValue::Int(i) => i,
        CfmlValue::Double(d) => d as i64,
        other => other.as_string().parse().unwrap_or(0),
    }
}

impl SessionStore for DatasourceStore {
    fn get(&self, id: &str) -> Option<SessionData> {
        self.ensure_schema();
        let now = now_secs();
        let sql = format!(
            "SELECT data FROM {t} WHERE cfid = ? AND app_name = ? AND expires_at > ?",
            t = self.table
        );
        let json = first_data_cell(self.run(
            &sql,
            vec![
                CfmlValue::string(id.to_string()),
                CfmlValue::string(self.app_name.clone()),
                CfmlValue::Int(now as i64),
            ],
            "array",
        ))?;
        serde_json::from_str(&json).ok()
    }

    fn set(&self, id: &str, data: SessionData) {
        self.ensure_schema();
        let json = match serde_json::to_string(&data) {
            Ok(j) => j,
            Err(e) => {
                eprintln!("[session/datasource] failed to serialise session '{}': {}", id, e);
                return;
            }
        };
        size_guard(id, &json);
        let exp = expires_at(&data);
        let new_hash = hash_str(&json);

        // Throttle: if neither the data nor a meaningful slice of the timeout
        // has changed since we last wrote, skip the round-trip entirely.
        if let Ok(cache) = self.touch_cache.lock() {
            if let Some((prev_exp, prev_hash)) = cache.get(id) {
                let quarter = (data.timeout_secs / 4).max(1);
                if *prev_hash == new_hash && exp.saturating_sub(*prev_exp) < quarter {
                    return;
                }
            }
        }

        // Portable upsert: UPDATE, then INSERT only if no row matched.
        let upd = format!(
            "UPDATE {t} SET data = ?, expires_at = ? WHERE cfid = ? AND app_name = ?",
            t = self.table
        );
        let res = self.run(
            &upd,
            vec![
                CfmlValue::string(json.clone()),
                CfmlValue::Int(exp as i64),
                CfmlValue::string(id.to_string()),
                CfmlValue::string(self.app_name.clone()),
            ],
            "query",
        );
        if affected_rows(&res) == 0 {
            let ins = format!(
                "INSERT INTO {t} (cfid, app_name, expires_at, data) VALUES (?, ?, ?, ?)",
                t = self.table
            );
            if let Err(e) = self.run(
                &ins,
                vec![
                    CfmlValue::string(id.to_string()),
                    CfmlValue::string(self.app_name.clone()),
                    CfmlValue::Int(exp as i64),
                    CfmlValue::string(json),
                ],
                "query",
            ) {
                // A concurrent INSERT for the same key (PK violation) is a lost
                // race under last-write-wins — log at debug, don't panic.
                log::debug!("[session/datasource] insert for '{}' failed: {}", id, e.message);
            }
        }

        if let Ok(mut cache) = self.touch_cache.lock() {
            cache.insert(id.to_string(), (exp, new_hash));
        }
    }

    fn remove(&self, id: &str) {
        self.ensure_schema();
        let sql = format!(
            "DELETE FROM {t} WHERE cfid = ? AND app_name = ?",
            t = self.table
        );
        let _ = self.run(
            &sql,
            vec![
                CfmlValue::string(id.to_string()),
                CfmlValue::string(self.app_name.clone()),
            ],
            "query",
        );
        if let Ok(mut cache) = self.touch_cache.lock() {
            cache.remove(id);
        }
    }

    fn rotate(&self, old_id: &str, new_id: &str) {
        self.ensure_schema();
        let sql = format!(
            "UPDATE {t} SET cfid = ? WHERE cfid = ? AND app_name = ?",
            t = self.table
        );
        let _ = self.run(
            &sql,
            vec![
                CfmlValue::string(new_id.to_string()),
                CfmlValue::string(old_id.to_string()),
                CfmlValue::string(self.app_name.clone()),
            ],
            "query",
        );
        if let Ok(mut cache) = self.touch_cache.lock() {
            if let Some(entry) = cache.remove(old_id) {
                cache.insert(new_id.to_string(), entry);
            }
        }
    }

    fn contains(&self, id: &str) -> bool {
        self.get(id).is_some()
    }

    fn take_expired(&self, now_secs: u64) -> Vec<(String, String, IndexMap<String, CfmlValue>)> {
        self.ensure_schema();
        // 1. Find candidates.
        let sel = format!(
            "SELECT cfid, data FROM {t} WHERE app_name = ? AND expires_at <= ?",
            t = self.table
        );
        let rows = match self.run(
            &sel,
            vec![
                CfmlValue::string(self.app_name.clone()),
                CfmlValue::Int(now_secs as i64),
            ],
            "array",
        ) {
            Ok(CfmlValue::Array(arr)) => arr.snapshot(),
            _ => return Vec::new(),
        };

        let del = format!(
            "DELETE FROM {t} WHERE cfid = ? AND app_name = ? AND expires_at <= ?",
            t = self.table
        );
        let mut out = Vec::new();
        for row in rows {
            let CfmlValue::Struct(r) = row else { continue };
            let cfid = r
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("cfid"))
                .map(|(_, v)| v.as_string())
                .unwrap_or_default();
            if cfid.is_empty() {
                continue;
            }
            // 2. Claim the row: only fire onSessionEnd if our DELETE removed it.
            let res = self.run(
                &del,
                vec![
                    CfmlValue::string(cfid.clone()),
                    CfmlValue::string(self.app_name.clone()),
                    CfmlValue::Int(now_secs as i64),
                ],
                "query",
            );
            if affected_rows(&res) >= 1 {
                if let Ok(mut cache) = self.touch_cache.lock() {
                    cache.remove(&cfid);
                }
                let data = r
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case("data"))
                    .map(|(_, v)| v.as_string())
                    .unwrap_or_default();
                let vars = serde_json::from_str::<SessionData>(&data)
                    .map(|s| s.variables)
                    .unwrap_or_default();
                out.push((self.app_name.clone(), cfid, vars));
            }
        }
        out
    }
}

/// Warn-only size guard (issue #88): oversized sessions are usually a bug.
fn size_guard(id: &str, json: &str) {
    const WARN_BYTES: usize = 64 * 1024;
    if json.len() > WARN_BYTES {
        log::warn!(
            "[session] session '{}' serialises to {} bytes (>{} KB) — oversized sessions are usually a bug; store ids/flags and rehydrate per request",
            id,
            json.len(),
            WARN_BYTES / 1024
        );
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(vars: &[(&str, CfmlValue)], last_accessed: u64, timeout: u64) -> SessionData {
        let mut m = IndexMap::new();
        for (k, v) in vars {
            m.insert(k.to_string(), v.clone());
        }
        SessionData {
            variables: m,
            created_secs: last_accessed,
            last_accessed_secs: last_accessed,
            auth_user: None,
            auth_roles: Vec::new(),
            timeout_secs: timeout,
            app_name: "appA".to_string(),
        }
    }

    /// Each test gets its own sqlite file + datasource so they don't collide
    /// when cargo runs them in parallel.
    fn store_for(name: &str) -> DatasourceStore {
        let path = std::env::temp_dir().join(format!("rustcfml_sess_{}.db", name));
        let _ = std::fs::remove_file(&path);
        let ds_name = format!("sesstest_{}", name);
        cfml_stdlib::builtins::register_datasource(&ds_name, path.to_string_lossy().to_string());
        DatasourceStore::new(&ds_name, "cf_session_data", "appA")
    }

    #[test]
    fn set_get_remove_round_trip() {
        let store = store_for("crud");
        let now = now_secs();
        store.set("sid-1", session(&[("cart", CfmlValue::Int(3))], now, 1800));

        let got = store.get("sid-1").expect("session should exist");
        assert_eq!(got.variables.get("cart").unwrap().as_string(), "3");

        // Overwrite (upsert UPDATE path) — change data so the throttle lets it through.
        store.set(
            "sid-1",
            session(&[("cart", CfmlValue::Int(9)), ("user", CfmlValue::string("x"))], now, 1800),
        );
        let got = store.get("sid-1").unwrap();
        assert_eq!(got.variables.get("cart").unwrap().as_string(), "9");
        assert_eq!(got.variables.get("user").unwrap().as_string(), "x");

        store.remove("sid-1");
        assert!(store.get("sid-1").is_none(), "removed session must be gone");
    }

    #[test]
    fn rotate_preserves_data() {
        let store = store_for("rotate");
        let now = now_secs();
        store.set("old", session(&[("k", CfmlValue::string("v"))], now, 1800));
        store.rotate("old", "new");
        assert!(store.get("old").is_none(), "old id should be gone after rotate");
        let got = store.get("new").expect("rotated session should exist under new id");
        assert_eq!(got.variables.get("k").unwrap().as_string(), "v");
    }

    #[test]
    fn expired_session_is_not_returned_by_get() {
        let store = store_for("getexpired");
        let now = now_secs();
        // last_accessed 10000s ago, 5s timeout → already expired.
        store.set("dead", session(&[("a", CfmlValue::Int(1))], now - 10_000, 5));
        assert!(store.get("dead").is_none(), "expired session must not be returned");
    }

    #[test]
    fn take_expired_claims_and_drains() {
        let store = store_for("sweep");
        let now = now_secs();
        store.set("live", session(&[("a", CfmlValue::Int(1))], now, 1800));
        store.set("dead", session(&[("b", CfmlValue::Int(2))], now - 10_000, 5));

        let expired = store.take_expired(now);
        assert_eq!(expired.len(), 1, "only the timed-out session should be swept");
        assert_eq!(expired[0].0, "appA", "app_name should be the store's partition");
        assert_eq!(expired[0].1, "dead");
        assert_eq!(expired[0].2.get("b").unwrap().as_string(), "2");

        // The claim is the delete — a second sweep returns nothing.
        assert!(store.take_expired(now).is_empty());
        // The live session survives.
        assert!(store.get("live").is_some());
    }

    #[test]
    fn sanitize_identifier_strips_unsafe_chars() {
        assert_eq!(sanitize_identifier("cf_session_data", "x"), "cf_session_data");
        assert_eq!(sanitize_identifier("foo; DROP TABLE bar", "x"), "fooDROPTABLEbar");
        assert_eq!(sanitize_identifier("", "cf_session_data"), "cf_session_data");
    }
}
