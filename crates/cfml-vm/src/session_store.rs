//! Pluggable session storage backend.

use cfml_common::dynamic::CfmlValue;
use indexmap::IndexMap;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::SessionData;

/// Trait for session storage backends.
///
/// All methods receive a session ID string. Implementations must be
/// `Send + Sync` so they can be shared across Tokio worker threads.
pub trait SessionStore: Send + Sync + 'static {
    /// Retrieve a session by ID. Returns `None` if the session does not exist.
    fn get(&self, id: &str) -> Option<SessionData>;
    /// Insert or overwrite a session.
    fn set(&self, id: &str, data: SessionData);
    /// Remove a session.
    fn remove(&self, id: &str);
    /// Atomically move session data from `old_id` to `new_id`.
    fn rotate(&self, old_id: &str, new_id: &str);
    /// Returns `true` if the session exists.
    fn contains(&self, id: &str) -> bool {
        self.get(id).is_some()
    }
    /// Drain all sessions whose `last_accessed_secs` age exceeds their
    /// `timeout_secs`, returning `(app_name, id, variables)` per drained
    /// session so callers can route `onSessionEnd` to the owning application.
    /// `app_name` is `""` for stores that do not record it (memcached / KV),
    /// which also never deliver `onSessionEnd`.
    fn take_expired(&self, now_secs: u64) -> Vec<(String, String, IndexMap<String, CfmlValue>)>;

    /// Earliest absolute expiry instant (unix epoch seconds) across all live
    /// sessions, if the store can compute it cheaply. Drives adaptive reaper
    /// scheduling: when `Some(t)`, the reaper may sleep until `t` (capped at
    /// the configured tick) instead of waking on a fixed interval. The default
    /// returns `None`, so a store opts out simply by not overriding it and the
    /// reaper falls back to its fixed tick.
    fn next_expiry(&self, _now_secs: u64) -> Option<u64> {
        None
    }
}

// ─────────────────────────────────────────────
// MemoryStore — in-process default
// ─────────────────────────────────────────────

/// In-process session store backed by a `Mutex<HashMap>`.
/// Identical behaviour to the original `ServerState.sessions` field.
#[derive(Clone)]
pub struct MemoryStore {
    inner: Arc<Mutex<HashMap<String, SessionData>>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionStore for MemoryStore {
    fn get(&self, id: &str) -> Option<SessionData> {
        // Read-path exactness (G1): a session is invisible the instant it is
        // past `last_accessed + timeout`, independent of any sweep. Remove the
        // dead record opportunistically while we hold the lock.
        let now = crate::now_epoch_secs();
        let mut m = self.inner.lock().ok()?;
        match m.get(id) {
            Some(s) if now.saturating_sub(s.last_accessed_secs) > s.timeout_secs => {
                m.remove(id);
                None
            }
            Some(s) => Some(s.clone()),
            None => None,
        }
    }

    fn set(&self, id: &str, data: SessionData) {
        if let Ok(mut m) = self.inner.lock() {
            m.insert(id.to_string(), data);
        }
    }

    fn remove(&self, id: &str) {
        if let Ok(mut m) = self.inner.lock() {
            m.remove(id);
        }
    }

    fn rotate(&self, old_id: &str, new_id: &str) {
        if let Ok(mut m) = self.inner.lock() {
            if let Some(data) = m.remove(old_id) {
                m.insert(new_id.to_string(), data);
            }
        }
    }

    fn contains(&self, id: &str) -> bool {
        // Route through the expiry-aware get so an expired record reads absent.
        self.get(id).is_some()
    }

    fn take_expired(&self, now_secs: u64) -> Vec<(String, String, IndexMap<String, CfmlValue>)> {
        if let Ok(mut m) = self.inner.lock() {
            let expired: Vec<String> = m
                .iter()
                .filter(|(_, s)| now_secs.saturating_sub(s.last_accessed_secs) > s.timeout_secs)
                .map(|(k, _)| k.clone())
                .collect();
            expired
                .into_iter()
                .filter_map(|id| m.remove(&id).map(|s| (s.app_name.clone(), id, s.variables)))
                .collect()
        } else {
            Vec::new()
        }
    }

    fn next_expiry(&self, _now_secs: u64) -> Option<u64> {
        let m = self.inner.lock().ok()?;
        m.values()
            .map(|s| s.last_accessed_secs.saturating_add(s.timeout_secs))
            .min()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::now_epoch_secs;

    fn session(app: &str, last_accessed: u64, timeout: u64) -> SessionData {
        SessionData {
            variables: IndexMap::new(),
            created_secs: last_accessed,
            last_accessed_secs: last_accessed,
            auth_user: None,
            auth_roles: Vec::new(),
            timeout_secs: timeout,
            app_name: app.to_string(),
        }
    }

    #[test]
    fn get_hides_and_removes_expired_record() {
        let store = MemoryStore::new();
        let now = now_epoch_secs();
        // last accessed 10_000s ago, 5s timeout → already expired.
        store.set("dead", session("appA", now - 10_000, 5));
        assert!(store.get("dead").is_none(), "expired session must read as absent");
        assert!(!store.contains("dead"), "contains must agree with get");
        // It was opportunistically removed, so take_expired finds nothing left.
        assert!(store.take_expired(now).is_empty(), "get should have evicted it");
    }

    #[test]
    fn get_returns_live_record() {
        let store = MemoryStore::new();
        let now = now_epoch_secs();
        store.set("live", session("appA", now, 1800));
        assert!(store.get("live").is_some());
        assert!(store.contains("live"));
    }

    #[test]
    fn take_expired_reports_owning_app_name() {
        let store = MemoryStore::new();
        let now = now_epoch_secs();
        store.set("a", session("shop", now - 10_000, 5));
        store.set("b", session("blog", now, 1800));
        let mut drained = store.take_expired(now);
        assert_eq!(drained.len(), 1, "only the timed-out session is drained");
        let (app, id, _vars) = drained.pop().unwrap();
        assert_eq!(app, "shop");
        assert_eq!(id, "a");
        // The live session survives and its app is untouched.
        assert!(store.get("b").is_some());
    }

    #[test]
    fn next_expiry_is_earliest_absolute_instant() {
        let store = MemoryStore::new();
        let base = 1_000_000;
        store.set("a", session("x", base, 100)); // expires at base+100
        store.set("b", session("x", base, 30)); // expires at base+30  (earliest)
        store.set("c", session("x", base, 900)); // expires at base+900
        assert_eq!(store.next_expiry(base), Some(base + 30));
    }
}
