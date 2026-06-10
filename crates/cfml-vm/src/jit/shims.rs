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

/// v0.99.5 — JIT inline-cache lookup for `obj.prop` on a `CfmlValue::Struct`.
///
/// `ic_slot` points at a `[u64; 2]` owned by the [`super::translate::Backend`]
/// (allocated as a `Box<UnsafeCell<[u64; 2]>>` so the address is stable for
/// the Backend's lifetime). Layout: `[cached_shape, cached_idx]`. The
/// sentinel `cached_shape == 0` is "never populated" — `CfmlStruct` shape
/// IDs start at 1.
///
/// Hot path (cached_shape == current_shape): hit `map.get_index(cached_idx)`,
/// box the value into the arena, return the tagged pointer. Cold path:
/// case-insensitive lookup via `get_ci_indexed`, repopulate the IC, return.
/// Missing key returns `Null` (matches the interpreter's `GetProperty`
/// branch on `Struct`).
///
/// **Bails when the receiver is not a `Struct`.** The IC only specialises
/// for plain structs in this first cut; Components / Queries / Closures /
/// Native objects all have more elaborate dispatch (accessors, column
/// proxies, etc.) and fall back to the interpreter via `*bail = 1`.
#[no_mangle]
pub extern "C" fn cfml_jit_member_get_boxed(
    obj_tagged: i64,
    name_ptr: *const u8,
    name_len: i64,
    ic_slot: *mut u64,
    bail: *mut i64,
) -> i64 {
    // v0.99.6 — IC slot is now `[cached_shape, cached_idx, cached_kind]`
    // where `cached_kind`:
    //   0 = never populated (matches initial zero-fill sentinel together
    //       with `cached_shape == 0`),
    //   1 = `CfmlValue::Int` → SMI fast-encode on hit,
    //   2 = `CfmlValue::Double` → return heap-boxed Double (Float SMI deferred
    //       to v0.99.7+),
    //   3 = other (String/Struct/Array/...) → arena-box clone.
    //
    // The kind ONLY hints the return encoding. The actual value at
    // `cached_idx` is still read each hit (shape_id guarantees that index
    // didn't shift; value-only mutations don't bump shape).
    let obj_utag = obj_tagged as usize;
    if obj_utag & boxed::TAG_MASK != boxed::TAG_PTR {
        // SMI-tagged Ints aren't structs.
        unsafe {
            *bail = 1;
        }
        return 0;
    }
    let v = unsafe { boxed::borrow_tagged(obj_utag) };
    let s = match v {
        CfmlValue::Struct(s) => s,
        _ => {
            unsafe {
                *bail = 1;
            }
            return 0;
        }
    };
    let shape = s.shape_id();
    // SAFETY: ic_slot points to a `[u64; 3]` owned by Backend.
    let cached_shape = unsafe { *ic_slot };
    let cached_idx = unsafe { *ic_slot.add(1) };
    let cached_kind = unsafe { *ic_slot.add(2) };
    if cached_shape == shape {
        if let Some(val) = s.get_at_index(cached_idx as usize) {
            return encode_member_result(val, cached_kind, ic_slot);
        }
        // Fall through to slow path on out-of-range — defensive.
    }
    // SAFETY: name_ptr is an interned `&'static str` pointer from
    // Backend's `member_names`.
    let name = unsafe {
        std::str::from_utf8_unchecked(std::slice::from_raw_parts(name_ptr, name_len as usize))
    };
    if let Some((idx, val)) = s.get_ci_indexed(name) {
        let kind_code = kind_code_for(&val);
        unsafe {
            *ic_slot = shape;
            *ic_slot.add(1) = idx as u64;
            *ic_slot.add(2) = kind_code;
        }
        return encode_member_result(val, kind_code, ic_slot);
    }
    // Missing key — interp returns `Null`. Cache as "other" so next hit
    // still finds it consistently if shape stays unchanged.
    arena::box_into_active(CfmlValue::Null) as i64
}

#[inline]
fn kind_code_for(v: &CfmlValue) -> u64 {
    match v {
        CfmlValue::Int(_) => 1,
        CfmlValue::Double(_) => 2,
        _ => 3,
    }
}

/// Encode a member-read result honouring the cached kind hint. When the
/// observed value's actual kind no longer matches the hint (e.g. someone
/// reassigned `obj.x = "foo"` after the IC saw `Int`), update the slot
/// and fall through to the new encoding.
#[inline]
fn encode_member_result(val: CfmlValue, cached_kind: u64, ic_slot: *mut u64) -> i64 {
    let actual = kind_code_for(&val);
    if actual != cached_kind {
        // Type drift: invalidate hint to the new kind. Shape didn't
        // change (we only get here under shape match), so the index
        // is still valid.
        unsafe {
            *ic_slot.add(2) = actual;
        }
    }
    match actual {
        1 => {
            // SMI fast path: encode inline if it fits, else heap-box.
            if let CfmlValue::Int(i) = val {
                if let Some(smi) = boxed::try_tag_smi_int(i) {
                    return smi;
                }
                return arena::box_into_active(CfmlValue::Int(i)) as i64;
            }
            unreachable!("kind_code 1 ⇒ Int")
        }
        _ => arena::box_into_active(val) as i64,
    }
}

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

/// Box an `i64` as a polymorphic JIT value. v0.99.6 — return an inline
/// SMI Int tag when the value fits in i61; otherwise heap-allocate as
/// `CfmlValue::Int(i)` and track it in the active arena.
#[no_mangle]
pub extern "C" fn cfml_jit_box_int(i: i64) -> i64 {
    if let Some(smi) = boxed::try_tag_smi_int(i) {
        return smi;
    }
    arena::box_into_active(CfmlValue::Int(i)) as i64
}

/// Box an `f64` (passed as its `to_bits()` reinterpretation, since the
/// caller may be holding a `Float` value in an `i64` slot — but in
/// practice we ABI it as a real `f64`). Returns the tagged pointer.
#[no_mangle]
pub extern "C" fn cfml_jit_box_float(d: f64) -> i64 {
    arena::box_into_active(CfmlValue::Double(d)) as i64
}

/// Materialise a tagged-pointer arg as an owned `CfmlValue`.
///
/// v0.99.6 — handles both heap pointers (clone the pointee) and inline SMI
/// Int tags (synthesise `CfmlValue::Int`). Heap clones for Arc-backed
/// variants (String/Array/Struct) are refcount bumps; for primitive
/// variants the clone is a few-byte copy. All callers operate on the
/// slow path, so the clone cost is acceptable.
unsafe fn materialize(tag: i64) -> CfmlValue {
    boxed::materialize_tagged(tag as usize)
}

/// Encode an owned `CfmlValue` as a tagged JIT value. Tries SMI Int first;
/// falls back to arena-tracked heap box.
#[inline]
fn encode_into_arena(v: CfmlValue) -> i64 {
    if let CfmlValue::Int(i) = &v {
        if let Some(smi) = boxed::try_tag_smi_int(*i) {
            return smi;
        }
    }
    arena::box_into_active(v) as i64
}

/// `a & b` where one or both operands cross as Boxed. Always produces a
/// String box (per CFML `&` semantics — both sides are stringified).
///
/// Args are passed as tagged `i64` pointers; both `a` and `b` must already
/// be boxed. The lowering of `Kind::Int + Kind::Boxed` Concat boxes the
/// Int side first via [`cfml_jit_box_int`] and then calls this shim.
#[no_mangle]
pub extern "C" fn cfml_jit_concat_boxed(a: i64, b: i64) -> i64 {
    let (sa, sb) = unsafe { (materialize(a), materialize(b)) };
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
    // v0.99.6 — SMI fast path: both inputs are inline Ints. Decode and
    // try to keep the result as SMI; only allocate on overflow out of i61.
    if boxed::is_smi_int(a) && boxed::is_smi_int(b) {
        let r = boxed::untag_smi_int(a).wrapping_add(boxed::untag_smi_int(b));
        if let Some(smi) = boxed::try_tag_smi_int(r) {
            return smi;
        }
        return arena::box_into_active(CfmlValue::Int(r)) as i64;
    }
    let (va, vb) = unsafe { (materialize(a), materialize(b)) };
    let result: CfmlValue = match (&va, &vb) {
        (CfmlValue::Int(i), CfmlValue::Int(j)) => CfmlValue::Int(i.wrapping_add(*j)),
        (CfmlValue::Double(x), CfmlValue::Double(y)) => CfmlValue::Double(x + y),
        (CfmlValue::Int(i), CfmlValue::Double(d)) => CfmlValue::Double(*i as f64 + d),
        (CfmlValue::Double(d), CfmlValue::Int(i)) => CfmlValue::Double(d + *i as f64),
        (CfmlValue::String(s), CfmlValue::String(t)) => CfmlValue::string(format!("{s}{t}")),
        _ => {
            let x = to_number(&va);
            let y = to_number(&vb);
            match (x, y) {
                (Some(x), Some(y)) => CfmlValue::Double(x + y),
                _ => CfmlValue::string(format!("{}{}", va.as_string(), vb.as_string())),
            }
        }
    };
    encode_into_arena(result)
}

/// v0.99.7 — `a - b` where one or both operands cross as Boxed. Mirrors the
/// interpreter's `BytecodeOp::Sub` (`numeric_op(.. |x,y| x-y)`): coerce both
/// sides via `to_number` (falling back to 0.0 on non-coercible) and produce
/// a `Double`. Int×Int stays in i64 via wrapping_sub (matches the
/// interpreter's Int×Int branch when the result still fits i64; CFML int
/// overflow wraps per gotcha #4).
///
/// `bail` is currently unused (Sub never throws — non-numeric coerces).
#[no_mangle]
pub extern "C" fn cfml_jit_sub_boxed(a: i64, b: i64, _bail: *mut i64) -> i64 {
    if boxed::is_smi_int(a) && boxed::is_smi_int(b) {
        let r = boxed::untag_smi_int(a).wrapping_sub(boxed::untag_smi_int(b));
        if let Some(smi) = boxed::try_tag_smi_int(r) {
            return smi;
        }
        return arena::box_into_active(CfmlValue::Int(r)) as i64;
    }
    let (va, vb) = unsafe { (materialize(a), materialize(b)) };
    let result: CfmlValue = match (&va, &vb) {
        (CfmlValue::Int(i), CfmlValue::Int(j)) => CfmlValue::Int(i.wrapping_sub(*j)),
        _ => {
            let x = to_number(&va).unwrap_or(0.0);
            let y = to_number(&vb).unwrap_or(0.0);
            CfmlValue::Double(x - y)
        }
    };
    encode_into_arena(result)
}

/// v0.99.7 — `a * b` where one or both operands cross as Boxed. Mirrors the
/// interpreter's `BytecodeOp::Mul`. Int×Int wraps via `wrapping_mul`; mixed
/// or non-numeric coerces to `Double` via `to_number().unwrap_or(0.0)`.
///
/// `bail` is currently unused (Mul never throws).
#[no_mangle]
pub extern "C" fn cfml_jit_mul_boxed(a: i64, b: i64, _bail: *mut i64) -> i64 {
    if boxed::is_smi_int(a) && boxed::is_smi_int(b) {
        let r = boxed::untag_smi_int(a).wrapping_mul(boxed::untag_smi_int(b));
        if let Some(smi) = boxed::try_tag_smi_int(r) {
            return smi;
        }
        return arena::box_into_active(CfmlValue::Int(r)) as i64;
    }
    let (va, vb) = unsafe { (materialize(a), materialize(b)) };
    let result: CfmlValue = match (&va, &vb) {
        (CfmlValue::Int(i), CfmlValue::Int(j)) => CfmlValue::Int(i.wrapping_mul(*j)),
        _ => {
            let x = to_number(&va).unwrap_or(0.0);
            let y = to_number(&vb).unwrap_or(0.0);
            CfmlValue::Double(x * y)
        }
    };
    encode_into_arena(result)
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
        // v0.99.6: box_int(42) returns an SMI tag (no heap alloc).
        assert!(boxed::is_smi_int(tag));
        let v = unsafe { materialize(tag) };
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
        let v = unsafe { materialize(r) };
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
        let v = unsafe { materialize(r) };
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
        let v = unsafe { materialize(r) };
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
        let v = unsafe { materialize(r) };
        assert!(matches!(v, CfmlValue::String(t) if t.as_str() == "34"));
        drop(_g);
        drop(unsafe { boxed::reclaim_tagged(a as usize) });
        drop(unsafe { boxed::reclaim_tagged(b as usize) });
        arena.drain_except(None);
    }

    #[test]
    fn sub_smi_int_int() {
        let mut arena = Arena::new();
        let _g = ArenaGuard::install(&mut arena);
        let a = boxed::try_tag_smi_int(10).unwrap();
        let b = boxed::try_tag_smi_int(3).unwrap();
        let mut bail = 0i64;
        let r = cfml_jit_sub_boxed(a, b, &mut bail);
        assert!(boxed::is_smi_int(r));
        let v = unsafe { materialize(r) };
        assert!(matches!(v, CfmlValue::Int(7)));
        drop(_g);
        arena.drain_except(None);
    }

    #[test]
    fn mul_smi_int_int() {
        let mut arena = Arena::new();
        let _g = ArenaGuard::install(&mut arena);
        let a = boxed::try_tag_smi_int(6).unwrap();
        let b = boxed::try_tag_smi_int(7).unwrap();
        let mut bail = 0i64;
        let r = cfml_jit_mul_boxed(a, b, &mut bail);
        assert!(boxed::is_smi_int(r));
        let v = unsafe { materialize(r) };
        assert!(matches!(v, CfmlValue::Int(42)));
        drop(_g);
        arena.drain_except(None);
    }

    #[test]
    fn sub_string_string_coerces_to_double() {
        let mut arena = Arena::new();
        let _g = ArenaGuard::install(&mut arena);
        let a = boxed::box_value(CfmlValue::string("10")) as i64;
        let b = boxed::box_value(CfmlValue::string("3")) as i64;
        let mut bail = 0i64;
        let r = cfml_jit_sub_boxed(a, b, &mut bail);
        let v = unsafe { materialize(r) };
        assert!(matches!(v, CfmlValue::Double(d) if (d - 7.0).abs() < 1e-9));
        drop(_g);
        drop(unsafe { boxed::reclaim_tagged(a as usize) });
        drop(unsafe { boxed::reclaim_tagged(b as usize) });
        arena.drain_except(None);
    }

    #[test]
    fn mul_int_plus_boxed_string_number_is_double() {
        let mut arena = Arena::new();
        let _g = ArenaGuard::install(&mut arena);
        let a = boxed::box_value(CfmlValue::Int(3)) as i64;
        let b = boxed::box_value(CfmlValue::string("4")) as i64;
        let mut bail = 0i64;
        let r = cfml_jit_mul_boxed(a, b, &mut bail);
        let v = unsafe { materialize(r) };
        assert!(matches!(v, CfmlValue::Double(d) if (d - 12.0).abs() < 1e-9));
        drop(_g);
        drop(unsafe { boxed::reclaim_tagged(a as usize) });
        drop(unsafe { boxed::reclaim_tagged(b as usize) });
        arena.drain_except(None);
    }

    #[test]
    fn sub_non_numeric_coerces_to_zero() {
        // Sub on two non-numeric strings: to_number returns None, falls back
        // to 0.0 - 0.0 = 0.0 (Double). Matches interpreter `numeric_op`.
        let mut arena = Arena::new();
        let _g = ArenaGuard::install(&mut arena);
        let a = boxed::box_value(CfmlValue::string("hello")) as i64;
        let b = boxed::box_value(CfmlValue::string("world")) as i64;
        let mut bail = 0i64;
        let r = cfml_jit_sub_boxed(a, b, &mut bail);
        let v = unsafe { materialize(r) };
        assert!(matches!(v, CfmlValue::Double(d) if d.abs() < 1e-9));
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
        let v = unsafe { materialize(r) };
        assert!(matches!(v, CfmlValue::String(t) if t.as_str() == "hello world"));
        drop(_g);
        drop(unsafe { boxed::reclaim_tagged(a as usize) });
        drop(unsafe { boxed::reclaim_tagged(b as usize) });
        arena.drain_except(None);
    }
}
