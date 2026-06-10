//! Option-γ tag-pointer encoding for polymorphic JIT values.
//!
//! v0.89.0 shipped only the outer-ABI crossing (heap pointer only, tag 0b000).
//! v0.90.0 added mid-body Boxed productions through the arena. v0.99.6 lights
//! up **TAG_INT** (0b001): Int values that fit in i61 are encoded inline in
//! the tagged word — no heap allocation, no arena tracking. This is the
//! classic V8 "SMI tagging" trick adapted to our 3-bit tag layout.
//!
//! Low-bit tagging is safe on every supported target including PAC-enabled
//! Apple Silicon (PAC modifies high bits 39–55, never the low bits of an
//! 8-aligned pointer). Float SMI / NaN-pun (high-bit) is deferred to a
//! future v0.99.7+ Phase B and will be gated on `cfg(target_arch)`.
//!
//! | tag   | meaning                            |
//! |-------|------------------------------------|
//! | 0b000 | `*const CfmlValue` (heap box)      |
//! | 0b001 | Int immediate (i61 signed)         |
//! | 0b010 | reserved (Bool/Null immediate)     |
//! | 0b011..0b111 | reserved                    |
//!
//! SMI range: encoding shifts the i64 left by 3, so values must satisfy
//! `(i << 3) >> 3 == i` (i.e. `i ∈ [-2^60, 2^60 − 1]`). Out-of-range Ints
//! fall back to heap-boxing via the arena.

use cfml_common::dynamic::CfmlValue;

pub const TAG_PTR: usize = 0b000;
pub const TAG_INT: usize = 0b001;
pub const TAG_MASK: usize = 0b111;

/// Build a tagged `usize` from an owned [`CfmlValue`], leaking the box. The
/// caller is responsible for reclaiming it via [`reclaim_tagged`].
pub fn box_value(v: CfmlValue) -> usize {
    let raw = Box::into_raw(Box::new(v));
    debug_assert!(raw as usize & TAG_MASK == 0, "Box<CfmlValue> must be 8-aligned");
    (raw as usize) | TAG_PTR
}

/// True if `tag` carries an inline SMI Int (tag 0b001).
#[inline]
pub fn is_smi_int(tag: i64) -> bool {
    (tag as usize) & TAG_MASK == TAG_INT
}

/// True if `tag` is a heap `Box<CfmlValue>` pointer (tag 0b000).
#[inline]
#[allow(dead_code)] // public API; reserved for future Phase-B Float SMI checks
pub fn is_heap_ptr(tag: i64) -> bool {
    (tag as usize) & TAG_MASK == TAG_PTR
}

/// Encode an `i64` as an inline SMI Int if it fits in the i61 range.
/// Returns `None` otherwise; caller must heap-box.
#[inline]
pub fn try_tag_smi_int(i: i64) -> Option<i64> {
    let shifted = (i as i64).wrapping_shl(3);
    if (shifted >> 3) == i {
        Some(shifted | TAG_INT as i64)
    } else {
        None
    }
}

/// Decode an SMI Int tagged word (must have `tag & 0b111 == TAG_INT`).
#[inline]
pub fn untag_smi_int(tag: i64) -> i64 {
    debug_assert!(is_smi_int(tag), "untag_smi_int on non-SMI tag");
    tag >> 3
}

/// Reclaim a tagged value. For `TAG_PTR`, takes ownership of the underlying
/// `Box<CfmlValue>` and returns the owned value. For `TAG_INT`, synthesises
/// `CfmlValue::Int(untagged)` (no heap memory to free).
///
/// # Safety
/// For `TAG_PTR`: `tagged` must have come from [`box_value`] in this process
/// and not yet have been reclaimed. SMI tags carry no aliasing constraint.
pub unsafe fn reclaim_tagged(tagged: usize) -> CfmlValue {
    let tag = tagged & TAG_MASK;
    match tag {
        TAG_INT => CfmlValue::Int(untag_smi_int(tagged as i64)),
        TAG_PTR => {
            let raw = (tagged & !TAG_MASK) as *mut CfmlValue;
            *Box::from_raw(raw)
        }
        _ => panic!("boxed.rs: unknown tag 0b{tag:03b}"),
    }
}

/// Materialise a tagged value as a fresh owned `CfmlValue` without consuming
/// the underlying box. For SMI Int, synthesises `CfmlValue::Int(untagged)`.
/// For heap pointers, clones the pointee (cheap for Arc-backed variants —
/// String/Array/Struct — and acceptable on the slow path for the rest).
///
/// # Safety
/// For heap pointers: `tagged` must point to a live box.
pub unsafe fn materialize_tagged(tagged: usize) -> CfmlValue {
    let tag = tagged & TAG_MASK;
    match tag {
        TAG_INT => CfmlValue::Int(untag_smi_int(tagged as i64)),
        TAG_PTR => {
            let raw = (tagged & !TAG_MASK) as *const CfmlValue;
            (*raw).clone()
        }
        _ => panic!("boxed.rs: unknown tag 0b{tag:03b}"),
    }
}

/// Borrow the underlying `CfmlValue` from a tagged HEAP pointer without
/// taking ownership. **Panics on SMI tags** — callers that need to handle
/// the polymorphic case must use [`materialize_tagged`] instead.
///
/// # Safety
/// `tagged` must point to a live box produced by [`box_value`].
#[allow(dead_code)]
pub unsafe fn borrow_tagged(tagged: usize) -> &'static CfmlValue {
    let tag = tagged & TAG_MASK;
    assert_eq!(tag, TAG_PTR, "boxed.rs: borrow_tagged on non-TAG_PTR (0b{tag:03b}) — use materialize_tagged");
    let raw = (tagged & !TAG_MASK) as *const CfmlValue;
    &*raw
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_string() {
        let s = CfmlValue::String(std::sync::Arc::new("hello".to_string()));
        let tagged = box_value(s.clone());
        assert_eq!(tagged & TAG_MASK, TAG_PTR);
        let r = unsafe { reclaim_tagged(tagged) };
        assert!(matches!(r, CfmlValue::String(ref a) if a.as_str() == "hello"));
    }

    #[test]
    fn smi_round_trip_zero() {
        let t = try_tag_smi_int(0).unwrap();
        assert!(is_smi_int(t));
        assert_eq!(untag_smi_int(t), 0);
    }

    #[test]
    fn smi_round_trip_positive_and_negative() {
        for &i in &[1i64, -1, 42, -42, 1_000_000, -1_000_000, i64::MIN >> 3, (i64::MAX >> 3)] {
            let t = try_tag_smi_int(i).unwrap();
            assert!(is_smi_int(t));
            assert_eq!(untag_smi_int(t), i);
        }
    }

    #[test]
    fn smi_overflow_returns_none() {
        assert!(try_tag_smi_int(i64::MAX).is_none());
        assert!(try_tag_smi_int(i64::MIN).is_none());
        assert!(try_tag_smi_int(1i64 << 60).is_none()); // just above i61 max
        assert!(try_tag_smi_int(-(1i64 << 60) - 1).is_none()); // just below i61 min
    }

    #[test]
    fn smi_reclaim_returns_int_no_panic() {
        let t = try_tag_smi_int(7).unwrap();
        let v = unsafe { reclaim_tagged(t as usize) };
        assert!(matches!(v, CfmlValue::Int(7)));
    }

    #[test]
    fn materialize_clones_heap_ptr() {
        let tagged = box_value(CfmlValue::Int(99));
        let m = unsafe { materialize_tagged(tagged) };
        assert!(matches!(m, CfmlValue::Int(99)));
        // box still owned by us; reclaim
        let r = unsafe { reclaim_tagged(tagged) };
        assert!(matches!(r, CfmlValue::Int(99)));
    }

    #[test]
    fn materialize_smi() {
        let t = try_tag_smi_int(-17).unwrap();
        let m = unsafe { materialize_tagged(t as usize) };
        assert!(matches!(m, CfmlValue::Int(-17)));
    }

    #[test]
    fn tag_predicates() {
        let smi = try_tag_smi_int(5).unwrap();
        assert!(is_smi_int(smi));
        assert!(!is_heap_ptr(smi));
        let heap = box_value(CfmlValue::Int(5)) as i64;
        assert!(is_heap_ptr(heap));
        assert!(!is_smi_int(heap));
        drop(unsafe { reclaim_tagged(heap as usize) });
    }
}
