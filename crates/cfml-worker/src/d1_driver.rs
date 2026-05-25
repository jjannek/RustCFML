//! Cloudflare D1 driver for the cfml-stdlib dynamic-driver registry.
//!
//! End-to-end path:
//! 1. CFML calls `<cfquery datasource="main">...</cfquery>` or
//!    `queryExecute("...", [...], {datasource: "main"})`.
//! 2. `cfml-stdlib::fn_query_execute` looks up the dynamic registry by
//!    name, finds a `D1Driver` registered by `cfml_worker::handle_fetch`.
//! 3. We serialize `(name, sql, params)` to JSON and call the JSPI extern
//!    [`crate::jspi::d1_query_sync`].
//! 4. JSPI suspends wasm, the JS shim awaits `db.prepare(sql).bind(...).all()`
//!    against the D1 binding bound to the same name, packs the response
//!    JSON back into wasm memory, resumes us.
//! 5. We parse the response and shape it into the CFML query value the rest
//!    of the language expects (records IndexMap + columnList + recordCount
//!    + metadata).
//!
//! Failure modes:
//! - Shim missing → user-friendly error mentioning the wiring step.
//! - SQL error from D1 → propagated as `CfmlError` with the original message.
//! - Type mismatch in row data → preserved as best-effort string.

#![cfg(target_arch = "wasm32")]

use cfml_common::dynamic::CfmlValue;
use cfml_common::vm::{CfmlError, CfmlResult};
use cfml_stdlib::DynamicDbDriver;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use worker::d1::D1Database;

pub struct D1Driver {
    name: String,
    /// Kept on the struct so `cfml-worker`'s handler can keep the binding
    /// alive across the duration of a request; the JS shim itself doesn't
    /// receive this handle — it looks up the binding by `name` against the
    /// `env` it captured at instantiation.
    #[allow(dead_code)]
    db: Arc<D1Database>,
}

impl D1Driver {
    pub fn new(name: impl Into<String>, db: Arc<D1Database>) -> Self {
        Self {
            name: name.into(),
            db,
        }
    }
}

#[derive(Serialize)]
struct WireRequest<'a> {
    datasource: &'a str,
    sql: &'a str,
    params: Vec<WireParam>,
}

/// Wire-format param values. We deliberately keep this narrow so the JS
/// shim has unambiguous behaviour — D1's param binding accepts numbers,
/// strings, null, ArrayBuffer (BLOB), and booleans-as-integers.
#[derive(Serialize)]
#[serde(untagged)]
enum WireParam {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
}

impl WireParam {
    fn from_cfml(v: &CfmlValue) -> Self {
        match v {
            CfmlValue::Null => WireParam::Null,
            CfmlValue::Bool(b) => WireParam::Bool(*b),
            CfmlValue::Int(i) => WireParam::Int(*i),
            CfmlValue::Double(d) => WireParam::Float(*d),
            CfmlValue::String(s) => WireParam::Str(s.clone()),
            // Anything else stringifies (date/time, etc.). D1 is SQLite-flavoured
            // and accepts ISO-8601 strings for date/time columns.
            other => WireParam::Str(other.as_string()),
        }
    }
}

#[derive(Deserialize)]
struct WireResponse {
    success: bool,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    results: Option<Vec<serde_json::Map<String, serde_json::Value>>>,
    #[serde(default)]
    meta: Option<WireMeta>,
}

#[derive(Deserialize, Default)]
struct WireMeta {
    #[serde(default)]
    duration: f64,
    #[serde(default)]
    rows_read: i64,
    #[serde(default)]
    rows_written: i64,
    #[serde(default)]
    changes: i64,
    #[serde(default)]
    last_row_id: i64,
}

impl DynamicDbDriver for D1Driver {
    fn execute(&self, sql: &str, params_arg: &CfmlValue, return_type: &str) -> CfmlResult {
        let params = match params_arg {
            CfmlValue::Array(arr) => arr.iter().map(WireParam::from_cfml).collect(),
            CfmlValue::Null => Vec::new(),
            single => vec![WireParam::from_cfml(single)],
        };

        let req = WireRequest {
            datasource: &self.name,
            sql,
            params,
        };
        let request_json = serde_json::to_string(&req).map_err(|e| {
            CfmlError::runtime(format!(
                "cfquery (D1 '{}'): could not serialize request: {}",
                self.name, e
            ))
        })?;

        let response_json = crate::jspi::d1_query_sync(&request_json)?;
        let response: WireResponse = serde_json::from_str(&response_json).map_err(|e| {
            CfmlError::runtime(format!(
                "cfquery (D1 '{}'): malformed shim response: {}",
                self.name, e
            ))
        })?;

        if !response.success {
            return Err(CfmlError::runtime(format!(
                "cfquery (D1 '{}'): {}",
                self.name,
                response.error.unwrap_or_else(|| "unknown error".into())
            )));
        }

        let rows = response.results.unwrap_or_default();
        let meta = response.meta.unwrap_or_default();

        match return_type {
            "array" => Ok(rows_to_array(rows)),
            rt if rt.starts_with("struct:") => {
                let key = &rt["struct:".len()..];
                Ok(rows_to_struct_by_key(rows, key))
            }
            _ => Ok(rows_to_query(rows, &meta)),
        }
    }
}

fn json_to_cfml(v: serde_json::Value) -> CfmlValue {
    match v {
        serde_json::Value::Null => CfmlValue::Null,
        serde_json::Value::Bool(b) => CfmlValue::Bool(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                CfmlValue::Int(i)
            } else if let Some(f) = n.as_f64() {
                CfmlValue::Double(f)
            } else {
                CfmlValue::String(n.to_string())
            }
        }
        serde_json::Value::String(s) => CfmlValue::String(s),
        serde_json::Value::Array(arr) => {
            CfmlValue::array(arr.into_iter().map(json_to_cfml).collect())
        }
        serde_json::Value::Object(obj) => {
            let mut m = IndexMap::new();
            for (k, val) in obj {
                m.insert(k, json_to_cfml(val));
            }
            CfmlValue::strukt(m)
        }
    }
}

/// Build a CFML query value the rest of the engine recognises:
/// `{recordCount, columnList, columns, data, _meta?}`. The existing
/// per-driver code in cfml-stdlib uses the same shape, so downstream code
/// (queryGetRow, query column access, cfdump, etc.) all just works.
fn rows_to_query(
    rows: Vec<serde_json::Map<String, serde_json::Value>>,
    meta: &WireMeta,
) -> CfmlValue {
    // Column order: union of keys, in the order they first appear. D1
    // returns row objects with consistent key ordering, but be defensive.
    let mut columns: Vec<String> = Vec::new();
    for row in &rows {
        for k in row.keys() {
            if !columns.iter().any(|c| c.eq_ignore_ascii_case(k)) {
                columns.push(k.clone());
            }
        }
    }

    let record_count = rows.len();

    // Convert to records: Vec of IndexMap<col, value>
    let mut records: Vec<CfmlValue> = Vec::with_capacity(record_count);
    for row in rows {
        let mut rec = IndexMap::new();
        for col in &columns {
            // case-insensitive lookup back into the json map
            let val = row
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(col))
                .map(|(_, v)| v.clone())
                .unwrap_or(serde_json::Value::Null);
            rec.insert(col.clone(), json_to_cfml(val));
        }
        records.push(CfmlValue::strukt(rec));
    }

    let mut q = IndexMap::new();
    q.insert(
        "recordCount".into(),
        CfmlValue::Int(record_count as i64),
    );
    q.insert(
        "columnList".into(),
        CfmlValue::String(columns.join(",")),
    );
    q.insert("columns".into(), CfmlValue::array(
        columns.iter().map(|c| CfmlValue::String(c.clone())).collect(),
    ));
    q.insert("data".into(), CfmlValue::array(records));

    // Driver metadata — useful for INSERTs / UPDATEs where you want
    // generatedKey or RECORDCOUNT for changed rows.
    let mut driver_meta = IndexMap::new();
    driver_meta.insert("duration_ms".into(), CfmlValue::Double(meta.duration));
    driver_meta.insert("rows_read".into(), CfmlValue::Int(meta.rows_read));
    driver_meta.insert("rows_written".into(), CfmlValue::Int(meta.rows_written));
    driver_meta.insert("changes".into(), CfmlValue::Int(meta.changes));
    driver_meta.insert("last_row_id".into(), CfmlValue::Int(meta.last_row_id));
    q.insert("_meta".into(), CfmlValue::strukt(driver_meta));

    CfmlValue::strukt(q)
}

fn rows_to_array(rows: Vec<serde_json::Map<String, serde_json::Value>>) -> CfmlValue {
    CfmlValue::array(
        rows.into_iter()
            .map(|row| {
                let mut rec = IndexMap::new();
                for (k, v) in row {
                    rec.insert(k, json_to_cfml(v));
                }
                CfmlValue::strukt(rec)
            })
            .collect(),
    )
}

fn rows_to_struct_by_key(
    rows: Vec<serde_json::Map<String, serde_json::Value>>,
    key: &str,
) -> CfmlValue {
    let mut out = IndexMap::new();
    for row in rows {
        let key_val = row
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.clone())
            .unwrap_or(serde_json::Value::Null);
        let key_str = match key_val {
            serde_json::Value::String(s) => s,
            other => other.to_string(),
        };
        let mut rec = IndexMap::new();
        for (k, v) in row {
            rec.insert(k, json_to_cfml(v));
        }
        out.insert(key_str, CfmlValue::strukt(rec));
    }
    CfmlValue::strukt(out)
}
