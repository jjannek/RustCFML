//! QoQ function registry: built-in (native Rust) and user-registered (CFML UDF)
//! functions usable inside QoQ SQL.

use cfml_common::dynamic::CfmlValue;
use cfml_common::vm::CfmlResult;
use std::collections::HashMap;

/// Tells the engine whether a function is called per-row (scalar) or
/// per-partition (aggregate).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QoQFnKind {
    /// Called once per row with the per-row argument values.
    /// Signature: `fn([arg1, arg2, …]) -> CfmlResult`.
    Scalar,
    /// Called once per partition. Each SQL argument is delivered as a
    /// `CfmlValue::Array` of that argument's value across every row in the
    /// partition. E.g. `SUM(salary)` receives `[Array([100, 200, 300])]`.
    Aggregate,
}

/// A native QoQ function pointer — same shape as a stdlib `BuiltinFunction`.
pub type QoQFn = fn(Vec<CfmlValue>) -> CfmlResult;

/// Holds the functions available inside QoQ SQL: native scalar/aggregate
/// functions (registered from Rust) and CFML UDFs/closures (registered at
/// runtime via `queryRegisterFunction`).
#[derive(Debug, Default)]
pub struct QoQFunctionRegistry {
    /// Native scalar functions, keyed by lowercase name.
    pub scalars: HashMap<String, QoQFn>,
    /// Native aggregate functions, keyed by lowercase name.
    pub aggregates: HashMap<String, QoQFn>,
    /// CFML UDFs/closures, keyed by lowercase name, with their kind. The stored
    /// `CfmlValue` is a `Function`/`Closure` invoked through the VM callback.
    pub customs: HashMap<String, (CfmlValue, QoQFnKind)>,
}

impl QoQFunctionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a native scalar or aggregate function.
    pub fn register_native(&mut self, name: &str, func: QoQFn, kind: QoQFnKind) {
        let key = name.to_lowercase();
        match kind {
            QoQFnKind::Scalar => {
                self.scalars.insert(key, func);
            }
            QoQFnKind::Aggregate => {
                self.aggregates.insert(key, func);
            }
        }
    }

    /// Register a CFML UDF/closure under `name`. `kind` defaults to scalar at
    /// the call site if the caller doesn't know better.
    pub fn register_custom(&mut self, name: &str, func: CfmlValue, kind: QoQFnKind) {
        self.customs.insert(name.to_lowercase(), (func, kind));
    }

    /// Look up a native function, returning its kind and pointer.
    pub fn get_native(&self, name: &str) -> Option<(QoQFnKind, QoQFn)> {
        let key = name.to_lowercase();
        if let Some(&f) = self.scalars.get(&key) {
            return Some((QoQFnKind::Scalar, f));
        }
        if let Some(&f) = self.aggregates.get(&key) {
            return Some((QoQFnKind::Aggregate, f));
        }
        None
    }

    /// Look up a custom CFML function (the value + its kind).
    pub fn get_custom(&self, name: &str) -> Option<&(CfmlValue, QoQFnKind)> {
        self.customs.get(&name.to_lowercase())
    }

    /// Is `name` an aggregate (native or custom)?
    pub fn is_aggregate(&self, name: &str) -> bool {
        let key = name.to_lowercase();
        self.aggregates.contains_key(&key)
            || matches!(self.customs.get(&key), Some((_, QoQFnKind::Aggregate)))
    }

    /// Is `name` registered at all (native or custom)?
    pub fn contains(&self, name: &str) -> bool {
        let key = name.to_lowercase();
        self.scalars.contains_key(&key)
            || self.aggregates.contains_key(&key)
            || self.customs.contains_key(&key)
    }
}
