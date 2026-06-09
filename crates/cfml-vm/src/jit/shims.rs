//! `extern "C"` runtime shims used by v0.90.0 Boxed codegen.
//!
//! Each shim is a side-effect-free or interpreter-equivalent operation that
//! the JIT calls when the analyser cannot stay in raw `i64`/`f64`. All boxes
//! the shims allocate are tracked in the active per-call [`super::arena`]
//! so the engine reclaims them on the way out of `run_compiled`.
//!
//! Semantics are bit-exact with the interpreter:
//! * [`cfml_jit_concat_boxed`] mirrors `BytecodeOp::Concat` in `lib.rs`
//!   (just `format!("{}{}", a.as_string(), b.as_string())`).
//! * [`cfml_jit_add_boxed`] mirrors `BytecodeOp::Add` on mixed-kind operands
//!   (numeric coercion if both sides `to_number()`-out, otherwise string
//!   concat — note that pure-numeric Add never reaches this shim because
//!   the analyser keeps it in the `Kind::Int`/`Kind::Float` lattice).
//! * [`cfml_jit_box_int`] / [`cfml_jit_box_float`] wrap the immediate in a
//!   fresh `CfmlValue::Int` / `Double` and tag it (so a mixed
//!   `Kind::Int + Kind::Boxed` Concat/Add can be lowered by boxing the Int
//!   side first and then calling the boxed shim).

use cfml_common::dynamic::CfmlValue;

use super::arena;
use super::boxed;

/// Local copy of the interpreter's `to_number` (private in `lib.rs`); kept
/// here so the shim is a single self-contained translation unit.
fn to_number(val: &CfmlValue) -> Option<f64> {
    match val {
        CfmlValue::Int(i) => Some(*i as f64),
        CfmlValue::Double(d) => Some(*d),
        CfmlValue::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        CfmlValue::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

/// Box an `i64` as `CfmlValue::Int(i)`, push it into the active arena,
/// return the tagged pointer.
#[no_mangle]
pub extern "C" fn cfml_jit_box_int(i: i64) -> i64 {
    arena::box_into_active(CfmlValue::Int(i)) as i64
}

/// Box an `f64` (passed as its `to_bits()` reinterpretation, since the
/// caller may be holding a `Float` value in an `i64` slot — but in
/// practice we ABI it as a real `f64`). Returns the tagged pointer.
#[no_mangle]
pub extern "C" fn cfml_jit_box_float(d: f64) -> i64 {
    arena::box_into_active(CfmlValue::Double(d)) as i64
}

/// Borrow a tagged-pointer arg without taking ownership. v0.89.0/0.90.0
/// only ever produce `TAG_PTR`, so a non-`TAG_PTR` tag is a hard bug.
unsafe fn borrow(tag: i64) -> &'static CfmlValue {
    // SAFETY: tag came from `boxed::box_value` (called by `box_into_active`
    // or by the engine while marshalling input args) and points to a live
    // `Box<CfmlValue>` owned by the active arena or the engine's
    // `boxed_args` set. Lifetime is bounded by the surrounding
    // `run_compiled` call.
    boxed::borrow_tagged(tag as usize)
}

/// `a & b` where one or both operands cross as Boxed. Always produces a
/// String box (per CFML `&` semantics — both sides are stringified).
///
/// Args are passed as tagged `i64` pointers; both `a` and `b` must already
/// be boxed. The lowering of `Kind::Int + Kind::Boxed` Concat boxes the
/// Int side first via [`cfml_jit_box_int`] and then calls this shim.
#[no_mangle]
pub extern "C" fn cfml_jit_concat_boxed(a: i64, b: i64) -> i64 {
    let (sa, sb) = unsafe { (borrow(a), borrow(b)) };
    let s = format!("{}{}", sa.as_string(), sb.as_string());
    arena::box_into_active(CfmlValue::string(s)) as i64
}

/// `a + b` where one or both operands cross as Boxed. Mirrors the
/// interpreter's `BytecodeOp::Add` mixed branch: try numeric coercion on
/// both sides; if both succeed, produce `Double(x + y)`; otherwise fall
/// back to string concat.
///
/// `bail` is currently unused (the operation never throws — fall-through
/// to string concat covers every non-numeric case). Reserved for future
/// strict-mode arithmetic.
#[no_mangle]
pub extern "C" fn cfml_jit_add_boxed(a: i64, b: i64, _bail: *mut i64) -> i64 {
    let (va, vb) = unsafe { (borrow(a), borrow(b)) };
    // Fast path: both already numeric.
    let result = match (va, vb) {
        (CfmlValue::Int(i), CfmlValue::Int(j)) => CfmlValue::Int(i.wrapping_add(*j)),
        (CfmlValue::Double(x), CfmlValue::Double(y)) => CfmlValue::Double(x + y),
        (CfmlValue::Int(i), CfmlValue::Double(d)) => CfmlValue::Double(*i as f64 + d),
        (CfmlValue::Double(d), CfmlValue::Int(i)) => CfmlValue::Double(d + *i as f64),
        (CfmlValue::String(s), CfmlValue::String(t)) => CfmlValue::string(format!("{s}{t}")),
        _ => {
            let x = to_number(va);
            let y = to_number(vb);
            match (x, y) {
                (Some(x), Some(y)) => CfmlValue::Double(x + y),
                _ => CfmlValue::string(format!("{}{}", va.as_string(), vb.as_string())),
            }
        }
    };
    arena::box_into_active(result) as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jit::arena::{Arena, ArenaGuard};

    #[test]
    fn box_int_round_trip() {
        let mut arena = Arena::new();
        let _g = ArenaGuard::install(&mut arena);
        let tag = cfml_jit_box_int(42);
        let v = unsafe { borrow(tag) };
        assert!(matches!(v, CfmlValue::Int(42)));
        drop(_g);
        arena.drain_except(None);
    }

    #[test]
    fn concat_string_string() {
        let mut arena = Arena::new();
        let _g = ArenaGuard::install(&mut arena);
        let a = boxed::box_value(CfmlValue::string("foo")) as i64;
        let b = boxed::box_value(CfmlValue::string("bar")) as i64;
        let r = cfml_jit_concat_boxed(a, b);
        let v = unsafe { borrow(r) };
        assert!(matches!(v, CfmlValue::String(s) if s.as_str() == "foobar"));
        drop(_g);
        // a + b not in arena (boxed::box_value direct) — reclaim manually.
        drop(unsafe { boxed::reclaim_tagged(a as usize) });
        drop(unsafe { boxed::reclaim_tagged(b as usize) });
        arena.drain_except(None);
    }

    #[test]
    fn concat_string_int_via_box_int() {
        let mut arena = Arena::new();
        let _g = ArenaGuard::install(&mut arena);
        let s = boxed::box_value(CfmlValue::string("row")) as i64;
        let i = cfml_jit_box_int(7); // boxed into arena
        let r = cfml_jit_concat_boxed(s, i);
        let v = unsafe { borrow(r) };
        assert!(matches!(v, CfmlValue::String(t) if t.as_str() == "row7"));
        drop(_g);
        drop(unsafe { boxed::reclaim_tagged(s as usize) });
        arena.drain_except(None);
    }

    #[test]
    fn add_int_plus_boxed_string_number_is_double() {
        // (Int, String "4") goes through the `_ => to_number / fallback`
        // arm and the strings parse so we get Double(7.0). Matches the
        // interpreter's `BytecodeOp::Add` mixed branch.
        let mut arena = Arena::new();
        let _g = ArenaGuard::install(&mut arena);
        let a = boxed::box_value(CfmlValue::Int(3)) as i64;
        let b = boxed::box_value(CfmlValue::string("4")) as i64;
        let mut bail = 0i64;
        let r = cfml_jit_add_boxed(a, b, &mut bail);
        let v = unsafe { borrow(r) };
        assert!(matches!(v, CfmlValue::Double(d) if (d - 7.0).abs() < 1e-9));
        drop(_g);
        drop(unsafe { boxed::reclaim_tagged(a as usize) });
        drop(unsafe { boxed::reclaim_tagged(b as usize) });
        arena.drain_except(None);
    }

    #[test]
    fn add_string_string_concats_per_interpreter() {
        // The interpreter's Add takes a `(String, String) => format!(..)`
        // branch *before* attempting numeric coercion; the shim mirrors
        // it exactly so the JIT can't diverge here.
        let mut arena = Arena::new();
        let _g = ArenaGuard::install(&mut arena);
        let a = boxed::box_value(CfmlValue::string("3")) as i64;
        let b = boxed::box_value(CfmlValue::string("4")) as i64;
        let mut bail = 0i64;
        let r = cfml_jit_add_boxed(a, b, &mut bail);
        let v = unsafe { borrow(r) };
        assert!(matches!(v, CfmlValue::String(t) if t.as_str() == "34"));
        drop(_g);
        drop(unsafe { boxed::reclaim_tagged(a as usize) });
        drop(unsafe { boxed::reclaim_tagged(b as usize) });
        arena.drain_except(None);
    }

    #[test]
    fn add_string_string_no_number_falls_back_to_concat() {
        let mut arena = Arena::new();
        let _g = ArenaGuard::install(&mut arena);
        let a = boxed::box_value(CfmlValue::string("hello")) as i64;
        let b = boxed::box_value(CfmlValue::string(" world")) as i64;
        let mut bail = 0i64;
        let r = cfml_jit_add_boxed(a, b, &mut bail);
        let v = unsafe { borrow(r) };
        assert!(matches!(v, CfmlValue::String(t) if t.as_str() == "hello world"));
        drop(_g);
        drop(unsafe { boxed::reclaim_tagged(a as usize) });
        drop(unsafe { boxed::reclaim_tagged(b as usize) });
        arena.drain_except(None);
    }
}
