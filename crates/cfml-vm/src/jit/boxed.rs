//! Option-γ tag-pointer encoding for polymorphic JIT values.
//!
//! v0.89.0 ships only the **outer-ABI** crossing: a compiled function can
//! accept Boxed arguments and return a Boxed value, but the body itself does
//! not yet tag/untag or operate on Boxed slots — every Boxed slot is loaded,
//! stored, and returned as an opaque `i64` tagged-`usize`. v0.90.0 adds the
//! mid-body operations (`+`, concat, member access) that need IR-level
//! tag/untag.
//!
//! Tag bits live in the **bottom 3 bits**, giving 8 tags with 8-byte-aligned
//! heap pointers underneath (which `Box<CfmlValue>` already guarantees on
//! every supported target — no PAC interaction).
//!
//! | tag   | meaning (v0.89.0)                  | v0.90.0+              |
//! |-------|------------------------------------|-----------------------|
//! | 0b000 | `*const CfmlValue` (heap box)      | unchanged             |
//! | 0b001 | reserved                           | Int immediate (i61)   |
//! | 0b010 | reserved                           | Bool / Null immediate |
//! | 0b011..0b111 | reserved                    | future tag growth     |
//!
//! In v0.89.0 only tag 0b000 is produced — every Boxed value crossing the
//! ABI is a leaked `Box<CfmlValue>`. The caller's marshaller owns the
//! lifetime: it builds the box, hands the raw pointer across, and reclaims
//! it after the compiled body returns. See [`super::run_compiled`].

use cfml_common::dynamic::CfmlValue;

/// Tag for a pointer-to-`CfmlValue` (the only encoding produced in v0.89.0).
pub const TAG_PTR: usize = 0b000;
pub const TAG_MASK: usize = 0b111;

/// Build a tagged `usize` from an owned [`CfmlValue`], leaking the box. The
/// caller is responsible for reclaiming it via [`reclaim_tagged`] — typically
/// after the compiled body has returned, so all in-flight slot references
/// have been consumed.
pub fn box_value(v: CfmlValue) -> usize {
    let raw = Box::into_raw(Box::new(v));
    debug_assert!(raw as usize & TAG_MASK == 0, "Box<CfmlValue> must be 8-aligned");
    (raw as usize) | TAG_PTR
}

/// Reclaim a tagged pointer produced by [`box_value`], returning the owned
/// value. Panics if the tag is not [`TAG_PTR`] — v0.89.0 never produces any
/// other tag, so a mismatch is a real bug.
///
/// # Safety
/// `tagged` must have come from [`box_value`] in this process and not yet
/// have been reclaimed.
pub unsafe fn reclaim_tagged(tagged: usize) -> CfmlValue {
    let tag = tagged & TAG_MASK;
    assert_eq!(tag, TAG_PTR, "boxed.rs: only TAG_PTR is produced in v0.89.0 (got 0b{tag:03b})");
    let raw = (tagged & !TAG_MASK) as *mut CfmlValue;
    *Box::from_raw(raw)
}

/// Borrow the underlying `CfmlValue` from a tagged pointer without taking
/// ownership. Used by the JIT engine to read a Boxed return value before
/// reclaiming the box. Panics on a non-`TAG_PTR` tag.
///
/// # Safety
/// `tagged` must point to a live box produced by [`box_value`].
#[allow(dead_code)] // v0.90.0+
pub unsafe fn borrow_tagged(tagged: usize) -> &'static CfmlValue {
    let tag = tagged & TAG_MASK;
    assert_eq!(tag, TAG_PTR, "boxed.rs: only TAG_PTR is produced in v0.89.0 (got 0b{tag:03b})");
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
    fn borrow_does_not_consume() {
        let tagged = box_value(CfmlValue::Int(7));
        let borrowed = unsafe { borrow_tagged(tagged) };
        assert!(matches!(borrowed, CfmlValue::Int(7)));
        // still own the box; reclaim cleans up
        let r = unsafe { reclaim_tagged(tagged) };
        assert!(matches!(r, CfmlValue::Int(7)));
    }
}
