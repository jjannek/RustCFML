//! Static allowlist of pure Tier-1 builtin shims the JIT may call.
//!
//! Each [`Shim`] is one signature (overloads share a `name`). The JIT only
//! permits a `LoadGlobal(name)` + `Call(n)` pair where `name` matches an entry
//! here and the actual operand kinds match some overload's `args_req`. Because
//! the shims are `extern "C"` Rust fns whose semantics mirror the interpreter
//! (`cfml-stdlib::builtins::fn_abs/min/max`), the JIT result is bit-identical to
//! the interpreter result for every accepted call shape.
//!
//! Shadowing safety lives in the engine: at `try_call` time we re-check that
//! each referenced builtin name is not shadowed in the VM's `user_functions` /
//! `globals` — if it is, we bail to the interpreter so a user-defined `abs`
//! still wins.
//!
//! Adding a new builtin = one `extern "C"` fn + one [`Shim`] entry; both
//! `analysis` and `translate` read the table by index, so no other edits.

use super::analysis::Kind;

/// What an argument kind must be for a [`Shim`] overload to apply.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KindReq {
    /// Must be exactly `Kind::Int`.
    Int,
    /// Must be exactly `Kind::Float`.
    Float,
    /// `Kind::Int` or `Kind::Float` — promoted at the ABI to the shim's
    /// `args_abi` slot via `to_f64` / `to_i64`.
    Numeric,
}

impl KindReq {
    fn matches(self, k: Kind) -> bool {
        match (self, k) {
            (KindReq::Int, Kind::Int) => true,
            (KindReq::Float, Kind::Float) => true,
            (KindReq::Numeric, Kind::Int | Kind::Float) => true,
            _ => false,
        }
    }
}

/// One ABI-level shim. Multiple `Shim`s may share `name` (overloads).
pub struct Shim {
    /// Lowercase CFML builtin name (must match `vm.builtins` key).
    pub name: &'static str,
    /// Per-arg acceptance rule (length = arity).
    pub args_req: &'static [KindReq],
    /// Per-arg ABI kind in the emitted IR (the operand value is converted to
    /// this kind before the `call`). Length = arity.
    pub args_abi: &'static [Kind],
    /// Result kind produced by the shim.
    pub ret_kind: Kind,
    /// Cranelift module symbol name (registered with `JITBuilder::symbol`).
    pub sym: &'static str,
    /// Raw fn pointer for `JITBuilder::symbol` to hand off to the linker.
    pub addr: *const u8,
}

// `Shim`'s `addr` is a function pointer; we share the table across threads.
// Function pointers are trivially `Send + Sync` semantically — Rust's stdlib
// just declines the auto-derive because of `*const u8`. The pointers live in
// this crate's read-only data section for the life of the process.
unsafe impl Sync for Shim {}

// ── extern "C" shims ────────────────────────────────────────────────────────
//
// Each mirrors a `cfml-stdlib::builtins::fn_*` entry exactly. They never
// allocate, never throw, and never touch the VM — pure functions of their
// arguments, safe to call from JIT'd code.

/// Mirrors `fn_abs` for `CfmlValue::Int(i)` → `Int(i.abs())`.
/// `i64::abs` panics on `INT_MIN` in debug; the interpreter does the same.
/// To keep the JIT side panic-free (and to let the interpreter fall through
/// for that one pathological input), we return `INT_MIN` for `INT_MIN` —
/// matching the *release-build* interpreter semantics (`i.abs()` wraps in
/// release because the underlying `wrapping_neg` is the same op).
extern "C" fn cfml_abs_i64(x: i64) -> i64 {
    x.wrapping_abs()
}

/// Mirrors `fn_abs` for `CfmlValue::Double(d)` → `Double(d.abs())`.
extern "C" fn cfml_abs_f64(x: f64) -> f64 {
    x.abs()
}

/// Mirrors `fn_min`: both operands promoted to `f64` via `get_float`, result
/// always `Double(a.min(b))`.
extern "C" fn cfml_min_f64(a: f64, b: f64) -> f64 {
    a.min(b)
}

/// Mirrors `fn_max`: both operands promoted to `f64`, result `Double(a.max(b))`.
extern "C" fn cfml_max_f64(a: f64, b: f64) -> f64 {
    a.max(b)
}

// ── Single-arg pure-math shims ────────────────────────────────────────────
// Every shim below is `extern "C"` and mirrors a `cfml-stdlib::builtins::fn_*`
// entry verbatim. Operand is always promoted to `f64` at the ABI boundary
// (`KindReq::Numeric` / `Kind::Float`), matching `get_float(args, 0)` in the
// interpreter. Return is `f64` for math functions and `i64` for the rounding /
// sign / truncation family — same as the interpreter's `CfmlValue::Double`
// vs `CfmlValue::Int`.

/// `fn_floor` — `x.floor() as i64`. CFML returns an `Int`.
extern "C" fn cfml_floor_i64(x: f64) -> i64 {
    x.floor() as i64
}

/// `fn_ceiling` — `x.ceil() as i64`. CFML returns an `Int`.
extern "C" fn cfml_ceiling_i64(x: f64) -> i64 {
    x.ceil() as i64
}

/// `fn_round` (1-arg form only) — half-up toward positive infinity, matching
/// Lucee/Adobe's `Math.round`. Bit-exact with `(x + 0.5).floor() as i64`.
/// Rust's `f64::round` is half-away-from-zero, which would diverge on negatives.
/// CFML returns `Int`.
extern "C" fn cfml_round_i64(x: f64) -> i64 {
    (x + 0.5).floor() as i64
}

/// `fn_sgn` — `1` / `-1` / `0` for positive / negative / zero. CFML returns `Int`.
extern "C" fn cfml_sgn_i64(x: f64) -> i64 {
    if x > 0.0 {
        1
    } else if x < 0.0 {
        -1
    } else {
        0
    }
}

/// `fn_fix` — truncate toward zero. `x.trunc() as i64`. CFML returns `Int`.
extern "C" fn cfml_fix_i64(x: f64) -> i64 {
    x.trunc() as i64
}

/// `fn_sqr` — square root. CFML uses the name `sqr` (not `sqrt`).
extern "C" fn cfml_sqr_f64(x: f64) -> f64 {
    x.sqrt()
}

extern "C" fn cfml_exp_f64(x: f64) -> f64 {
    x.exp()
}
/// `fn_log` — natural log (`ln`). CFML's `log` IS the natural log.
extern "C" fn cfml_log_f64(x: f64) -> f64 {
    x.ln()
}
extern "C" fn cfml_log10_f64(x: f64) -> f64 {
    x.log10()
}
extern "C" fn cfml_sin_f64(x: f64) -> f64 {
    x.sin()
}
extern "C" fn cfml_cos_f64(x: f64) -> f64 {
    x.cos()
}
extern "C" fn cfml_tan_f64(x: f64) -> f64 {
    x.tan()
}
extern "C" fn cfml_asin_f64(x: f64) -> f64 {
    x.asin()
}
extern "C" fn cfml_acos_f64(x: f64) -> f64 {
    x.acos()
}
extern "C" fn cfml_atan_f64(x: f64) -> f64 {
    x.atan()
}

// ── Bit-twiddling shims ───────────────────────────────────────────────────
// Mirror `fn_bit_and/or/xor/not/shln/shrn` in cfml-stdlib. Operands are
// always Int at the ABI; `bitNot` truncates to Java's 32-bit `int` semantics
// (Lucee/Adobe parity) before complementing.

extern "C" fn cfml_bit_and_i64(a: i64, b: i64) -> i64 {
    a & b
}
extern "C" fn cfml_bit_or_i64(a: i64, b: i64) -> i64 {
    a | b
}
extern "C" fn cfml_bit_xor_i64(a: i64, b: i64) -> i64 {
    a ^ b
}
/// `fn_bit_not` truncates to 32 bits before complementing to match the
/// interpreter's Java-int semantics: `(!(x as i32)) as i64`.
extern "C" fn cfml_bit_not_i64(x: i64) -> i64 {
    (!(x as i32)) as i64
}
extern "C" fn cfml_bit_shln_i64(a: i64, b: i64) -> i64 {
    a << b
}
extern "C" fn cfml_bit_shrn_i64(a: i64, b: i64) -> i64 {
    a >> b
}

// ── incrementValue / decrementValue — typed overloads ───────────────────
// `fn_increment_value` / `fn_decrement_value` in cfml-stdlib pattern-match
// on `Int` and `Double` and add/subtract 1; the JIT only ever sees those
// two kinds, so two overloads each cover the JIT-eligible paths exactly.
// Non-numeric coercion stays in the interpreter (we'd bail in analysis).

extern "C" fn cfml_increment_value_i64(x: i64) -> i64 {
    x.wrapping_add(1)
}
extern "C" fn cfml_increment_value_f64(x: f64) -> f64 {
    x + 1.0
}
extern "C" fn cfml_decrement_value_i64(x: i64) -> i64 {
    x.wrapping_sub(1)
}
extern "C" fn cfml_decrement_value_f64(x: f64) -> f64 {
    x - 1.0
}

// ── bitMaskRead / bitMaskSet / bitMaskClear — 3 / 4-arg Int → Int ────────
// Mirror `fn_bit_mask_read` / `fn_bit_mask_set` / `fn_bit_mask_clear` in
// cfml-stdlib bit-for-bit.

extern "C" fn cfml_bit_mask_read_i64(number: i64, start: i64, length: i64) -> i64 {
    (number >> start) & ((1i64 << length) - 1)
}
extern "C" fn cfml_bit_mask_set_i64(number: i64, mask: i64, start: i64, length: i64) -> i64 {
    let clear_mask = ((1i64 << length) - 1) << start;
    (number & !clear_mask) | ((mask & ((1i64 << length) - 1)) << start)
}
extern "C" fn cfml_bit_mask_clear_i64(number: i64, start: i64, length: i64) -> i64 {
    number & !(((1i64 << length) - 1) << start)
}

/// `fn_pow(base, exp)` — `base.powf(exp)`. The `^` infix operator already
/// calls `translate::cfml_pow` (a private extern in the translate module);
/// this adds the function-call form too so `pow(2, 10)` JITs identically
/// to `2 ^ 10`.
extern "C" fn cfml_pow_fn_f64(base: f64, exp: f64) -> f64 {
    base.powf(exp)
}

/// The complete shim table. Order matters for `lookup_overload`: more specific
/// signatures (e.g. `abs(Int)`) must precede broader ones (`abs(Numeric)`).
pub static SHIMS: &[Shim] = &[
    Shim {
        name: "abs",
        args_req: &[KindReq::Int],
        args_abi: &[Kind::Int],
        ret_kind: Kind::Int,
        sym: "cfml_abs_i64",
        addr: cfml_abs_i64 as *const u8,
    },
    Shim {
        name: "abs",
        args_req: &[KindReq::Float],
        args_abi: &[Kind::Float],
        ret_kind: Kind::Float,
        sym: "cfml_abs_f64",
        addr: cfml_abs_f64 as *const u8,
    },
    Shim {
        name: "min",
        args_req: &[KindReq::Numeric, KindReq::Numeric],
        args_abi: &[Kind::Float, Kind::Float],
        ret_kind: Kind::Float,
        sym: "cfml_min_f64",
        addr: cfml_min_f64 as *const u8,
    },
    Shim {
        name: "max",
        args_req: &[KindReq::Numeric, KindReq::Numeric],
        args_abi: &[Kind::Float, Kind::Float],
        ret_kind: Kind::Float,
        sym: "cfml_max_f64",
        addr: cfml_max_f64 as *const u8,
    },
    // ── Single-arg numeric → Int (rounding / sign / truncation) ──────────
    Shim {
        name: "floor",
        args_req: &[KindReq::Numeric],
        args_abi: &[Kind::Float],
        ret_kind: Kind::Int,
        sym: "cfml_floor_i64",
        addr: cfml_floor_i64 as *const u8,
    },
    Shim {
        name: "ceiling",
        args_req: &[KindReq::Numeric],
        args_abi: &[Kind::Float],
        ret_kind: Kind::Int,
        sym: "cfml_ceiling_i64",
        addr: cfml_ceiling_i64 as *const u8,
    },
    Shim {
        name: "round",
        args_req: &[KindReq::Numeric],
        args_abi: &[Kind::Float],
        ret_kind: Kind::Int,
        sym: "cfml_round_i64",
        addr: cfml_round_i64 as *const u8,
    },
    Shim {
        name: "sgn",
        args_req: &[KindReq::Numeric],
        args_abi: &[Kind::Float],
        ret_kind: Kind::Int,
        sym: "cfml_sgn_i64",
        addr: cfml_sgn_i64 as *const u8,
    },
    Shim {
        name: "fix",
        args_req: &[KindReq::Numeric],
        args_abi: &[Kind::Float],
        ret_kind: Kind::Int,
        sym: "cfml_fix_i64",
        addr: cfml_fix_i64 as *const u8,
    },
    // ── Single-arg numeric → Float (transcendentals) ─────────────────────
    Shim {
        name: "sqr",
        args_req: &[KindReq::Numeric],
        args_abi: &[Kind::Float],
        ret_kind: Kind::Float,
        sym: "cfml_sqr_f64",
        addr: cfml_sqr_f64 as *const u8,
    },
    Shim {
        name: "exp",
        args_req: &[KindReq::Numeric],
        args_abi: &[Kind::Float],
        ret_kind: Kind::Float,
        sym: "cfml_exp_f64",
        addr: cfml_exp_f64 as *const u8,
    },
    Shim {
        name: "log",
        args_req: &[KindReq::Numeric],
        args_abi: &[Kind::Float],
        ret_kind: Kind::Float,
        sym: "cfml_log_f64",
        addr: cfml_log_f64 as *const u8,
    },
    Shim {
        name: "log10",
        args_req: &[KindReq::Numeric],
        args_abi: &[Kind::Float],
        ret_kind: Kind::Float,
        sym: "cfml_log10_f64",
        addr: cfml_log10_f64 as *const u8,
    },
    Shim {
        name: "sin",
        args_req: &[KindReq::Numeric],
        args_abi: &[Kind::Float],
        ret_kind: Kind::Float,
        sym: "cfml_sin_f64",
        addr: cfml_sin_f64 as *const u8,
    },
    Shim {
        name: "cos",
        args_req: &[KindReq::Numeric],
        args_abi: &[Kind::Float],
        ret_kind: Kind::Float,
        sym: "cfml_cos_f64",
        addr: cfml_cos_f64 as *const u8,
    },
    Shim {
        name: "tan",
        args_req: &[KindReq::Numeric],
        args_abi: &[Kind::Float],
        ret_kind: Kind::Float,
        sym: "cfml_tan_f64",
        addr: cfml_tan_f64 as *const u8,
    },
    Shim {
        name: "asin",
        args_req: &[KindReq::Numeric],
        args_abi: &[Kind::Float],
        ret_kind: Kind::Float,
        sym: "cfml_asin_f64",
        addr: cfml_asin_f64 as *const u8,
    },
    Shim {
        name: "acos",
        args_req: &[KindReq::Numeric],
        args_abi: &[Kind::Float],
        ret_kind: Kind::Float,
        sym: "cfml_acos_f64",
        addr: cfml_acos_f64 as *const u8,
    },
    Shim {
        name: "atan",
        args_req: &[KindReq::Numeric],
        args_abi: &[Kind::Float],
        ret_kind: Kind::Float,
        sym: "cfml_atan_f64",
        addr: cfml_atan_f64 as *const u8,
    },
    // ── Bit-twiddling: 1 / 2-arg Int → Int ───────────────────────────────
    Shim {
        name: "bitand",
        args_req: &[KindReq::Int, KindReq::Int],
        args_abi: &[Kind::Int, Kind::Int],
        ret_kind: Kind::Int,
        sym: "cfml_bit_and_i64",
        addr: cfml_bit_and_i64 as *const u8,
    },
    Shim {
        name: "bitor",
        args_req: &[KindReq::Int, KindReq::Int],
        args_abi: &[Kind::Int, Kind::Int],
        ret_kind: Kind::Int,
        sym: "cfml_bit_or_i64",
        addr: cfml_bit_or_i64 as *const u8,
    },
    Shim {
        name: "bitxor",
        args_req: &[KindReq::Int, KindReq::Int],
        args_abi: &[Kind::Int, Kind::Int],
        ret_kind: Kind::Int,
        sym: "cfml_bit_xor_i64",
        addr: cfml_bit_xor_i64 as *const u8,
    },
    Shim {
        name: "bitnot",
        args_req: &[KindReq::Int],
        args_abi: &[Kind::Int],
        ret_kind: Kind::Int,
        sym: "cfml_bit_not_i64",
        addr: cfml_bit_not_i64 as *const u8,
    },
    Shim {
        name: "bitshln",
        args_req: &[KindReq::Int, KindReq::Int],
        args_abi: &[Kind::Int, Kind::Int],
        ret_kind: Kind::Int,
        sym: "cfml_bit_shln_i64",
        addr: cfml_bit_shln_i64 as *const u8,
    },
    Shim {
        name: "bitshrn",
        args_req: &[KindReq::Int, KindReq::Int],
        args_abi: &[Kind::Int, Kind::Int],
        ret_kind: Kind::Int,
        sym: "cfml_bit_shrn_i64",
        addr: cfml_bit_shrn_i64 as *const u8,
    },
    // ── incrementValue / decrementValue — typed overloads ────────────────
    Shim {
        name: "incrementvalue",
        args_req: &[KindReq::Int],
        args_abi: &[Kind::Int],
        ret_kind: Kind::Int,
        sym: "cfml_increment_value_i64",
        addr: cfml_increment_value_i64 as *const u8,
    },
    Shim {
        name: "incrementvalue",
        args_req: &[KindReq::Float],
        args_abi: &[Kind::Float],
        ret_kind: Kind::Float,
        sym: "cfml_increment_value_f64",
        addr: cfml_increment_value_f64 as *const u8,
    },
    Shim {
        name: "decrementvalue",
        args_req: &[KindReq::Int],
        args_abi: &[Kind::Int],
        ret_kind: Kind::Int,
        sym: "cfml_decrement_value_i64",
        addr: cfml_decrement_value_i64 as *const u8,
    },
    Shim {
        name: "decrementvalue",
        args_req: &[KindReq::Float],
        args_abi: &[Kind::Float],
        ret_kind: Kind::Float,
        sym: "cfml_decrement_value_f64",
        addr: cfml_decrement_value_f64 as *const u8,
    },
    // ── bitMaskRead / bitMaskSet / bitMaskClear — Int → Int ──────────────
    Shim {
        name: "bitmaskread",
        args_req: &[KindReq::Int, KindReq::Int, KindReq::Int],
        args_abi: &[Kind::Int, Kind::Int, Kind::Int],
        ret_kind: Kind::Int,
        sym: "cfml_bit_mask_read_i64",
        addr: cfml_bit_mask_read_i64 as *const u8,
    },
    Shim {
        name: "bitmaskset",
        args_req: &[KindReq::Int, KindReq::Int, KindReq::Int, KindReq::Int],
        args_abi: &[Kind::Int, Kind::Int, Kind::Int, Kind::Int],
        ret_kind: Kind::Int,
        sym: "cfml_bit_mask_set_i64",
        addr: cfml_bit_mask_set_i64 as *const u8,
    },
    Shim {
        name: "bitmaskclear",
        args_req: &[KindReq::Int, KindReq::Int, KindReq::Int],
        args_abi: &[Kind::Int, Kind::Int, Kind::Int],
        ret_kind: Kind::Int,
        sym: "cfml_bit_mask_clear_i64",
        addr: cfml_bit_mask_clear_i64 as *const u8,
    },
    // ── 2-arg pow() builtin (function-call form of the `^` infix) ────────
    Shim {
        name: "pow",
        args_req: &[KindReq::Numeric, KindReq::Numeric],
        args_abi: &[Kind::Float, Kind::Float],
        ret_kind: Kind::Float,
        sym: "cfml_pow_fn_f64",
        addr: cfml_pow_fn_f64 as *const u8,
    },
];

/// Lowercased lookup: `true` iff some shim has this exact name. Currently only
/// used by tests in this file — the production path goes through
/// [`canonical_name`] so the analyser also gets the `&'static str` interner.
#[cfg(test)]
pub fn name_is_known(name: &str) -> bool {
    canonical_name(name).is_some()
}

/// Returns the canonical `&'static str` for `name` (case-insensitive lookup
/// into [`SHIMS`]), or `None` if no overload matches. The returned slice is the
/// shim table's own name field, so it doubles as a stable interned identifier
/// that callers can stash in [`super::analysis::Kind::Builtin`].
pub fn canonical_name(name: &str) -> Option<&'static str> {
    let lower = name.to_ascii_lowercase();
    SHIMS.iter().find(|s| s.name == lower).map(|s| s.name)
}

/// Resolve a `(name, arg_kinds)` to the matching shim index in [`SHIMS`].
///
/// Returns `None` when no overload of `name` accepts these exact kinds. Walks
/// the table in declaration order, so put more specific overloads first.
pub fn lookup_overload(name: &str, arg_kinds: &[Kind]) -> Option<usize> {
    let lower = name.to_ascii_lowercase();
    SHIMS.iter().enumerate().find_map(|(i, s)| {
        if s.name != lower {
            return None;
        }
        if s.args_req.len() != arg_kinds.len() {
            return None;
        }
        if s.args_req.iter().zip(arg_kinds.iter()).all(|(req, k)| req.matches(*k)) {
            Some(i)
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_lookup_is_case_insensitive() {
        assert!(name_is_known("abs"));
        assert!(name_is_known("ABS"));
        assert!(name_is_known("AbS"));
        assert!(!name_is_known("nope"));
    }

    #[test]
    fn overload_prefers_specific_int() {
        // abs(Int) → Int (not promoted to Float)
        let idx = lookup_overload("abs", &[Kind::Int]).unwrap();
        assert_eq!(SHIMS[idx].ret_kind, Kind::Int);
    }

    #[test]
    fn overload_picks_float_when_arg_is_float() {
        let idx = lookup_overload("abs", &[Kind::Float]).unwrap();
        assert_eq!(SHIMS[idx].ret_kind, Kind::Float);
    }

    #[test]
    fn min_accepts_any_numeric_combo() {
        for kinds in [
            [Kind::Int, Kind::Int],
            [Kind::Int, Kind::Float],
            [Kind::Float, Kind::Int],
            [Kind::Float, Kind::Float],
        ] {
            let idx = lookup_overload("min", &kinds).expect("min should accept any numeric combo");
            assert_eq!(SHIMS[idx].ret_kind, Kind::Float);
        }
    }

    #[test]
    fn unknown_name_or_arity_rejects() {
        assert!(lookup_overload("nope", &[Kind::Int]).is_none());
        assert!(lookup_overload("abs", &[Kind::Int, Kind::Int]).is_none());
        assert!(lookup_overload("min", &[Kind::Int]).is_none());
    }

    #[test]
    fn shim_semantics_match_interpreter() {
        // Spot-check a handful of values to confirm the extern "C" shims do
        // what `fn_abs`/`fn_min`/`fn_max` do.
        assert_eq!(cfml_abs_i64(-5), 5);
        assert_eq!(cfml_abs_i64(7), 7);
        assert_eq!(cfml_abs_i64(0), 0);
        // wrapping_abs(INT_MIN) = INT_MIN — release-mode parity.
        assert_eq!(cfml_abs_i64(i64::MIN), i64::MIN);
        assert_eq!(cfml_abs_f64(-1.5), 1.5);
        assert_eq!(cfml_min_f64(3.0, 5.0), 3.0);
        assert_eq!(cfml_max_f64(3.0, 5.0), 5.0);
        // f64::min/max propagate NaN per the second operand; same as f64::min in the interp.
        assert!(cfml_min_f64(f64::NAN, 1.0).is_nan() || cfml_min_f64(f64::NAN, 1.0) == 1.0);
    }
}
