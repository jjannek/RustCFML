//! Dynamic value types for CFML runtime

use crate::vm::CfmlResult;
use indexmap::IndexMap;
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, RwLock};

/// Trait implemented by Rust types that want to be addressable as CFML
/// objects (`new rust:MyClass()` / member-call dispatch).
///
/// Implementers must be `Send + Sync` because instances can be shared across
/// cfthread boundaries via the surrounding `Arc<RwLock<…>>`. `Debug` is
/// required so the runtime can stringify native objects in dump output
/// without an extra trait.
///
/// `call_method` is the single dispatch entry point: the runtime looks up
/// `name` on the object and forwards `args`. Method names are matched
/// case-insensitively at the call site, so implementers can choose either
/// style — the convention is camelCase to match the rest of the CFML
/// surface.
pub trait CfmlNative: Send + Sync + fmt::Debug {
    /// Logical class name (e.g. "Counter"). Used for `type_name`,
    /// `getMetadata`, and dump output.
    fn class_name(&self) -> &str;

    /// Invoke a method on the underlying Rust value. Return
    /// `Err(CfmlError::…)` for unknown methods or argument mismatches.
    fn call_method(&mut self, name: &str, args: Vec<CfmlValue>) -> CfmlResult;

    /// Optional property read. Used when a CFC declares
    /// `extends="rust:Name"` and host code reads `this.X` (or `inst.X`)
    /// for a key the CFC struct doesn't define. Default returns `None` —
    /// the runtime falls back to the standard CFC property lookup.
    /// Implementers expose Rust-side state to the CFC half by returning
    /// `Some(value)` for the names they recognise.
    fn get_property(&self, _name: &str) -> Option<CfmlValue> {
        None
    }

    /// Optional property write. Mirrors `get_property`: return `None` to
    /// let the CFC struct take the assignment, or `Some(Ok(()))` /
    /// `Some(Err(…))` to indicate the native side handled (or rejected)
    /// the write. Default returns `None`.
    fn set_property(&mut self, _name: &str, _value: CfmlValue) -> Option<Result<(), crate::vm::CfmlError>> {
        None
    }
}

#[derive(Clone)]
pub enum CfmlValue {
    Null,
    Bool(bool),
    Int(i64),
    Double(f64),
    String(String),
    Array(Arc<Vec<CfmlValue>>),
    /// Lucee-style query column proxy: behaves as Array for iteration/indexing/length,
    /// but stringifies to the first row's value (so `q.col & "x"` works) and reports
    /// `type_name()` as "Array" so `isArray()` is true. Produced by `query.colname`
    /// member-access on a Query. Payload is the column's row values.
    QueryColumn(Arc<Vec<CfmlValue>>),
    Struct(Arc<IndexMap<String, CfmlValue>>),
    Closure(Box<CfmlClosure>),
    Component(Box<CfmlComponent>),
    Function(CfmlFunction),
    Query(CfmlQuery),
    Binary(Vec<u8>),
    /// Instance of a Rust-backed class registered via
    /// `CfmlVirtualMachine::register_native_class`. Method dispatch goes
    /// through the `CfmlNative` trait.
    NativeObject(Arc<RwLock<dyn CfmlNative>>),
}

/// Hand-rolled Debug elides the Arc<_> wrapper on Array/Struct so log diffs
/// and test output remain byte-identical to the pre-Arc-flip representation.
impl fmt::Debug for CfmlValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CfmlValue::Null => f.write_str("Null"),
            CfmlValue::Bool(b) => f.debug_tuple("Bool").field(b).finish(),
            CfmlValue::Int(i) => f.debug_tuple("Int").field(i).finish(),
            CfmlValue::Double(d) => f.debug_tuple("Double").field(d).finish(),
            CfmlValue::String(s) => f.debug_tuple("String").field(s).finish(),
            CfmlValue::Array(a) => f.debug_tuple("Array").field(&**a).finish(),
            CfmlValue::QueryColumn(a) => f.debug_tuple("QueryColumn").field(&**a).finish(),
            CfmlValue::Struct(s) => f.debug_tuple("Struct").field(&**s).finish(),
            CfmlValue::Closure(c) => f.debug_tuple("Closure").field(c).finish(),
            CfmlValue::Component(c) => f.debug_tuple("Component").field(c).finish(),
            CfmlValue::Function(fun) => f.debug_tuple("Function").field(fun).finish(),
            CfmlValue::Query(q) => f.debug_tuple("Query").field(q).finish(),
            CfmlValue::Binary(b) => f.debug_tuple("Binary").field(b).finish(),
            CfmlValue::NativeObject(obj) => match obj.read() {
                Ok(g) => f
                    .debug_tuple("NativeObject")
                    .field(&g.class_name().to_string())
                    .finish(),
                Err(_) => f.debug_tuple("NativeObject").field(&"<poisoned>").finish(),
            },
        }
    }
}

impl CfmlValue {
    pub fn type_name(&self) -> &'static str {
        match self {
            CfmlValue::Null => "Null",
            CfmlValue::Bool(_) => "Boolean",
            CfmlValue::Int(_) => "Integer",
            CfmlValue::Double(_) => "Double",
            CfmlValue::String(_) => "String",
            CfmlValue::Array(_) => "Array",
            // Lucee@7: `isArray(q.col)` is false — QueryColumn is a string proxy
            // with bracket-indexing for rows, not an array. Distinct type_name
            // means isArray/isStruct/etc. all report false.
            CfmlValue::QueryColumn(_) => "QueryColumn",
            CfmlValue::Struct(_) => "Struct",
            CfmlValue::Closure(_) => "Closure",
            CfmlValue::Component(_) => "Component",
            CfmlValue::Function(_) => "Function",
            CfmlValue::Query(_) => "Query",
            CfmlValue::Binary(_) => "Binary",
            CfmlValue::NativeObject(_) => "NativeObject",
        }
    }

    pub fn is_true(&self) -> bool {
        match self {
            CfmlValue::Null => false,
            CfmlValue::Bool(b) => *b,
            CfmlValue::Int(i) => *i != 0,
            CfmlValue::Double(d) => *d != 0.0,
            CfmlValue::String(s) => {
                let trimmed = s.trim();
                if trimmed.is_empty() {
                    return false;
                }
                match trimmed.to_lowercase().as_str() {
                    "false" | "no" | "0" => false,
                    _ => true,
                }
            }
            CfmlValue::Array(a) => !a.is_empty(),
            // QueryColumn truthiness: first row's truthiness (Lucee proxies to first row).
            CfmlValue::QueryColumn(a) => a.first().map(|v| v.is_true()).unwrap_or(false),
            CfmlValue::Struct(s) => !s.is_empty(),
            CfmlValue::Closure(_) => true,
            CfmlValue::Component(_) => true,
            CfmlValue::Function(_) => true,
            CfmlValue::Query(q) => !q.rows.is_empty(),
            CfmlValue::Binary(b) => !b.is_empty(),
            CfmlValue::NativeObject(_) => true,
        }
    }

    pub fn as_string(&self) -> String {
        match self {
            CfmlValue::Null => String::new(),
            CfmlValue::Bool(b) => b.to_string(),
            CfmlValue::Int(i) => i.to_string(),
            CfmlValue::Double(d) => d.to_string(),
            CfmlValue::String(s) => s.clone(),
            CfmlValue::Array(a) => {
                let items: Vec<String> = a.iter().map(|v| v.as_string()).collect();
                format!("[{}]", items.join(", "))
            }
            // QueryColumn stringifies to the first row's value, matching Lucee's
            // proxy behavior so `q.col & "x"` concatenates the first row.
            CfmlValue::QueryColumn(a) => a.first().map(|v| v.as_string()).unwrap_or_default(),
            CfmlValue::Struct(s) => {
                let items: Vec<String> = s
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v.as_string()))
                    .collect();
                format!("{{{}}}", items.join(", "))
            }
            CfmlValue::Closure(_) => "<Closure>".to_string(),
            CfmlValue::Component(_) => "<Component>".to_string(),
            CfmlValue::Function(f) => f.name.clone(),
            CfmlValue::Query(_) => "<Query>".to_string(),
            CfmlValue::Binary(_) => "<Binary>".to_string(),
            CfmlValue::NativeObject(obj) => match obj.read() {
                Ok(g) => format!("<NativeObject:{}>", g.class_name()),
                Err(_) => "<NativeObject:poisoned>".to_string(),
            },
        }
    }

    pub fn get(&self, key: &str) -> Option<CfmlValue> {
        match self {
            CfmlValue::Struct(s) => s.get(key).cloned(),
            CfmlValue::Array(a) | CfmlValue::QueryColumn(a) => {
                if let Ok(idx) = key.parse::<usize>() {
                    a.get(idx).cloned()
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn set(&mut self, key: String, value: CfmlValue) {
        match self {
            CfmlValue::Struct(s) => {
                Arc::make_mut(s).insert(key, value);
            }
            CfmlValue::Array(a) => {
                if let Ok(idx) = key.parse::<usize>() {
                    if idx < a.len() {
                        Arc::make_mut(a)[idx] = value;
                    }
                }
            }
            _ => {}
        }
    }

    /// Construct a `CfmlValue::Array` from an owned `Vec`, wrapping in the
    /// shared Arc layer. `#[inline]` because this is called from every
    /// Array-producing builtin across crate boundaries.
    #[inline]
    pub fn array(v: Vec<CfmlValue>) -> Self {
        CfmlValue::Array(Arc::new(v))
    }

    /// Construct a `CfmlValue::Struct` from an owned `IndexMap`, wrapping in
    /// the shared Arc layer. Named `strukt` because `struct` is a keyword.
    #[inline]
    pub fn strukt(m: IndexMap<String, CfmlValue>) -> Self {
        CfmlValue::Struct(Arc::new(m))
    }

    pub fn as_array(&self) -> Option<&Vec<CfmlValue>> {
        match self {
            CfmlValue::Array(a) => Some(a),
            _ => None,
        }
    }

    /// Like `as_array` but also returns the row view when called on a
    /// `QueryColumn`. Use for narrow opt-in cases (e.g. `valueList(q.col)`
    /// which canonically iterates rows on Lucee). Most array consumers
    /// should stay on `as_array` so that `arrayLen(q.col)` etc. cleanly
    /// reject the value, matching Lucee@7.
    pub fn as_array_or_query_column(&self) -> Option<&Vec<CfmlValue>> {
        match self {
            CfmlValue::Array(a) | CfmlValue::QueryColumn(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_struct(&self) -> Option<&IndexMap<String, CfmlValue>> {
        match self {
            CfmlValue::Struct(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_array_mut(&mut self) -> Option<&mut Vec<CfmlValue>> {
        match self {
            CfmlValue::Array(a) => Some(Arc::make_mut(a)),
            _ => None,
        }
    }

    pub fn as_struct_mut(&mut self) -> Option<&mut IndexMap<String, CfmlValue>> {
        match self {
            CfmlValue::Struct(s) => Some(Arc::make_mut(s)),
            _ => None,
        }
    }

    pub fn eq(&self, other: &CfmlValue) -> bool {
        match (self, other) {
            (CfmlValue::Null, CfmlValue::Null) => true,
            // NativeObjects compare by identity: two CFML references that
            // point at the same underlying Rust object are equal. A second
            // `createObject("rust", "Name")` returns a fresh Arc and so is
            // NOT equal even if the Rust state matches.
            (CfmlValue::NativeObject(a), CfmlValue::NativeObject(b)) => Arc::ptr_eq(a, b),
            (CfmlValue::Bool(a), CfmlValue::Bool(b)) => a == b,
            (CfmlValue::Int(a), CfmlValue::Int(b)) => a == b,
            (CfmlValue::Double(a), CfmlValue::Double(b)) => a == b,
            (CfmlValue::String(a), CfmlValue::String(b)) => a.to_lowercase() == b.to_lowercase(),
            (CfmlValue::Int(a), CfmlValue::Double(b)) => *a as f64 == *b,
            (CfmlValue::Double(a), CfmlValue::Int(b)) => *a == *b as f64,
            (
                CfmlValue::Array(a) | CfmlValue::QueryColumn(a),
                CfmlValue::Array(b) | CfmlValue::QueryColumn(b),
            ) => {
                if a.len() != b.len() {
                    return false;
                }
                a.iter().zip(b.iter()).all(|(x, y)| x.eq(y))
            }
            (CfmlValue::Struct(a), CfmlValue::Struct(b)) => {
                if a.len() != b.len() {
                    return false;
                }
                a.iter()
                    .all(|(k, v)| b.get(k).map(|bv| v.eq(bv)).unwrap_or(false))
            }
            _ => false,
        }
    }
}

impl Default for CfmlValue {
    fn default() -> Self {
        CfmlValue::Null
    }
}

impl fmt::Display for CfmlValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_string())
    }
}

#[derive(Debug, Clone)]
pub struct CfmlClosure {
    pub params: Vec<String>,
    pub body: Box<CfmlClosureBody>,
    pub captured_vars: IndexMap<String, CfmlValue>,
}

#[derive(Debug, Clone)]
pub enum CfmlClosureBody {
    Expression(Box<CfmlValue>),
    Statements(Vec<CfmlStatement>),
}

#[derive(Debug, Clone)]
pub enum CfmlStatement {
    Expression(CfmlValue),
    Return(Option<CfmlValue>),
    Assignment(String, CfmlValue),
}

#[derive(Debug, Clone)]
pub struct CfmlComponent {
    pub name: String,
    pub properties: IndexMap<String, CfmlValue>,
    pub methods: HashMap<String, CfmlFunction>,
    pub extends: Option<String>,
    pub implements: Vec<String>,
}

impl CfmlComponent {
    pub fn new(name: String) -> Self {
        Self {
            name,
            properties: IndexMap::new(),
            methods: HashMap::new(),
            extends: None,
            implements: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CfmlFunction {
    pub name: String,
    pub params: Vec<CfmlParam>,
    pub body: CfmlClosureBody,
    pub return_type: Option<String>,
    pub access: CfmlAccess,
    /// Captured scope for closures — shared mutable environment so multiple
    /// invocations (and sibling closures) see each other's mutations.
    pub captured_scope: Option<Arc<RwLock<IndexMap<String, CfmlValue>>>>,
}

#[derive(Debug, Clone)]
pub struct CfmlParam {
    pub name: String,
    pub param_type: Option<String>,
    pub default: Option<CfmlValue>,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CfmlAccess {
    Public,
    Private,
    Package,
    Remote,
}

#[derive(Debug, Clone)]
pub struct CfmlQuery {
    pub columns: Vec<String>,
    pub rows: Vec<IndexMap<String, CfmlValue>>,
    pub sql: Option<String>,
}

impl CfmlQuery {
    pub fn new(columns: Vec<String>) -> Self {
        Self {
            columns,
            rows: Vec::new(),
            sql: None,
        }
    }
}

// ─────────────────────────────────────────────
// CfmlValue serde support (for session serialization)
// ─────────────────────────────────────────────

impl serde::Serialize for CfmlValue {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::{SerializeMap, SerializeSeq};
        match self {
            CfmlValue::Null => s.serialize_none(),
            CfmlValue::Bool(b) => s.serialize_bool(*b),
            CfmlValue::Int(i) => s.serialize_i64(*i),
            CfmlValue::Double(d) => s.serialize_f64(*d),
            CfmlValue::String(st) => s.serialize_str(st),
            CfmlValue::Array(a) | CfmlValue::QueryColumn(a) => {
                let mut seq = s.serialize_seq(Some(a.len()))?;
                for v in a.iter() {
                    seq.serialize_element(v)?;
                }
                seq.end()
            }
            CfmlValue::Struct(m) => {
                let mut map = s.serialize_map(Some(m.len()))?;
                for (k, v) in m.iter() {
                    map.serialize_entry(k, v)?;
                }
                map.end()
            }
            CfmlValue::Binary(b) => {
                let hex: String = b.iter().map(|byte| format!("{:02x}", byte)).collect();
                let mut map = s.serialize_map(Some(2))?;
                map.serialize_entry("_cftype", "binary")?;
                map.serialize_entry("data", &hex)?;
                map.end()
            }
            CfmlValue::Query(q) => {
                let mut map = s.serialize_map(Some(3))?;
                map.serialize_entry("_cftype", "query")?;
                map.serialize_entry("columns", &q.columns)?;
                let rows: Vec<std::collections::HashMap<&str, &CfmlValue>> = q
                    .rows
                    .iter()
                    .map(|row| row.iter().map(|(k, v)| (k.as_str(), v)).collect())
                    .collect();
                map.serialize_entry("rows", &rows)?;
                map.end()
            }
            CfmlValue::Closure(_) | CfmlValue::Function(_) | CfmlValue::Component(_) | CfmlValue::NativeObject(_) => {
                log::debug!("serializing non-serializable CfmlValue variant as null");
                s.serialize_none()
            }
        }
    }
}

impl<'de> serde::Deserialize<'de> for CfmlValue {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_any(CfmlValueVisitor)
    }
}

struct CfmlValueVisitor;

impl<'de> serde::de::Visitor<'de> for CfmlValueVisitor {
    type Value = CfmlValue;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a CFML value (null, bool, number, string, array, or object)")
    }

    fn visit_unit<E: serde::de::Error>(self) -> Result<CfmlValue, E> {
        Ok(CfmlValue::Null)
    }
    fn visit_none<E: serde::de::Error>(self) -> Result<CfmlValue, E> {
        Ok(CfmlValue::Null)
    }
    fn visit_some<D: serde::Deserializer<'de>>(self, d: D) -> Result<CfmlValue, D::Error> {
        serde::Deserialize::deserialize(d)
    }
    fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<CfmlValue, E> {
        Ok(CfmlValue::Bool(v))
    }
    fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<CfmlValue, E> {
        Ok(CfmlValue::Int(v))
    }
    fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<CfmlValue, E> {
        Ok(CfmlValue::Int(v as i64))
    }
    fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<CfmlValue, E> {
        if v.fract() == 0.0 && v >= i64::MIN as f64 && v <= i64::MAX as f64 {
            Ok(CfmlValue::Int(v as i64))
        } else {
            Ok(CfmlValue::Double(v))
        }
    }
    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<CfmlValue, E> {
        Ok(CfmlValue::String(v.to_string()))
    }
    fn visit_string<E: serde::de::Error>(self, v: String) -> Result<CfmlValue, E> {
        Ok(CfmlValue::String(v))
    }
    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut a: A) -> Result<CfmlValue, A::Error> {
        let mut vec = Vec::new();
        while let Some(v) = a.next_element::<CfmlValue>()? {
            vec.push(v);
        }
        Ok(CfmlValue::array(vec))
    }
    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut a: A) -> Result<CfmlValue, A::Error> {
        let mut map: IndexMap<String, CfmlValue> = IndexMap::new();
        while let Some((k, v)) = a.next_entry::<String, CfmlValue>()? {
            map.insert(k, v);
        }
        // Detect tagged special types
        if let Some(CfmlValue::String(t)) = map.get("_cftype") {
            match t.as_str() {
                "binary" => {
                    if let Some(CfmlValue::String(hex)) = map.get("data") {
                        let bytes: Vec<u8> = (0..hex.len())
                            .step_by(2)
                            .filter_map(|i| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok())
                            .collect();
                        return Ok(CfmlValue::Binary(bytes));
                    }
                }
                "query" => {
                    if let Some(CfmlValue::Array(cols)) = map.get("columns") {
                        let columns: Vec<String> =
                            cols.iter().map(|v| v.as_string()).collect();
                        let mut query = CfmlQuery::new(columns.clone());
                        if let Some(CfmlValue::Array(rows)) = map.get("rows") {
                            for row_val in rows.iter() {
                                if let CfmlValue::Struct(row_map) = row_val {
                                    query.rows.push((**row_map).clone());
                                }
                            }
                        }
                        return Ok(CfmlValue::Query(query));
                    }
                }
                _ => {}
            }
        }
        Ok(CfmlValue::strukt(map))
    }
}

#[cfg(test)]
mod size_probe {
    //! PR-0 size probes (RustCFML performance plan). These print the live size
    //! of the core value/runtime types and assert a non-regression *ceiling*.
    //!
    //! Run with: `cargo test -p cfml-common size_probe -- --nocapture`
    //!
    //! When an intentional shrink lands (e.g. boxing `Function`/`Query`,
    //! `String(Arc<str>)`), tighten the ceiling here so the win is recorded and
    //! protected against future regressions.
    use super::*;
    use std::mem::size_of;

    #[test]
    fn report_sizes() {
        let cfml_value = size_of::<CfmlValue>();
        eprintln!("size_of::<CfmlValue>()      = {cfml_value} B");
        eprintln!("size_of::<CfmlFunction>()   = {} B", size_of::<CfmlFunction>());
        eprintln!("size_of::<CfmlQuery>()      = {} B", size_of::<CfmlQuery>());
        eprintln!("size_of::<CfmlComponent>()  = {} B", size_of::<CfmlComponent>());
        eprintln!("size_of::<CfmlClosure>()    = {} B", size_of::<CfmlClosure>());

        // Ceiling, not an exact match: catches accidental growth, tolerates
        // shrinks. Lower this number whenever a planned shrink lands.
        //
        // Baseline as of PR-0 (2026-05-30): 112 B. The dominant driver is the
        // inline `Function(CfmlFunction)` variant (112 B on its own) — boxing
        // it (and `Query`, 72 B) is the T1.1 win and should drop this toward
        // ~24 B. Note: the original plan said 88 B; the enum has since grown
        // (the `QueryColumn` variant + a fatter `CfmlFunction`).
        assert!(
            cfml_value <= 112,
            "CfmlValue grew to {cfml_value} B (ceiling 112 B) — a perf regression. \
             If intentional, justify and raise the ceiling."
        );
    }
}
