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
    /// Must be exactly `Kind::Boxed` (Option-γ tag-pointer). v0.92.0+ —
    /// crosses to the shim as a raw `i64` holding the tagged pointer; the
    /// shim borrows the underlying `CfmlValue` via `boxed::borrow_tagged`.
    Boxed,
}

impl KindReq {
    fn matches(self, k: Kind) -> bool {
        match (self, k) {
            (KindReq::Int, Kind::Int) => true,
            (KindReq::Float, Kind::Float) => true,
            (KindReq::Numeric, Kind::Int | Kind::Float) => true,
            (KindReq::Boxed, Kind::Boxed) => true,
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

// ── v0.92.0 Boxed-argument shims ────────────────────────────────────────────
// Each one takes a tagged `i64` (TAG_PTR-encoded `*const CfmlValue` from
// `boxed.rs`), borrows the underlying value via `boxed::borrow_tagged`, and
// either returns an `i64` (numeric ret_kind) or constructs a fresh
// `CfmlValue` and pushes it into the active arena (Boxed ret_kind).
//
// All six mirror their `cfml-stdlib::builtins::fn_*` counterparts bit-for-bit
// and are infallible by design: `len` is defensive (returns 0 for unknown
// types in interp), and the case/trim shims use `as_string()` which works on
// every `CfmlValue` variant. No bail path needed.

/// Mirrors `fn_len`: `String → s.len()` (bytes), `Bool/Int/Double → chars of
/// the stringified form`, `Array/Struct/Binary → element count`,
/// `QueryColumn → chars of stringified first row`, anything else → `0`.
extern "C" fn cfml_len_boxed_i64(tagged: i64) -> i64 {
    use cfml_common::dynamic::CfmlValue;
    let v = unsafe { super::boxed::borrow_tagged(tagged as usize) };
    match v {
        CfmlValue::String(s) => s.len() as i64,
        CfmlValue::Bool(_) | CfmlValue::Int(_) | CfmlValue::Double(_) => {
            v.as_string().chars().count() as i64
        }
        CfmlValue::Array(a) => a.len() as i64,
        CfmlValue::Struct(s) => s.len() as i64,
        CfmlValue::Binary(b) => b.len() as i64,
        CfmlValue::QueryColumn(_) => v.as_string().chars().count() as i64,
        _ => 0,
    }
}

/// Mirrors `fn_ucase`: `CfmlValue::string(v.as_string().to_uppercase())`,
/// boxed into the active arena. Result kind: `Boxed`.
extern "C" fn cfml_ucase_boxed(tagged: i64) -> i64 {
    use cfml_common::dynamic::CfmlValue;
    let v = unsafe { super::boxed::borrow_tagged(tagged as usize) };
    super::arena::box_into_active(CfmlValue::string(v.as_string().to_uppercase())) as i64
}

/// Mirrors `fn_lcase`.
extern "C" fn cfml_lcase_boxed(tagged: i64) -> i64 {
    use cfml_common::dynamic::CfmlValue;
    let v = unsafe { super::boxed::borrow_tagged(tagged as usize) };
    super::arena::box_into_active(CfmlValue::string(v.as_string().to_lowercase())) as i64
}

/// Mirrors `fn_trim`.
extern "C" fn cfml_trim_boxed(tagged: i64) -> i64 {
    use cfml_common::dynamic::CfmlValue;
    let v = unsafe { super::boxed::borrow_tagged(tagged as usize) };
    super::arena::box_into_active(CfmlValue::string(v.as_string().trim().to_string())) as i64
}

/// Mirrors `fn_ltrim`.
extern "C" fn cfml_ltrim_boxed(tagged: i64) -> i64 {
    use cfml_common::dynamic::CfmlValue;
    let v = unsafe { super::boxed::borrow_tagged(tagged as usize) };
    super::arena::box_into_active(CfmlValue::string(v.as_string().trim_start().to_string())) as i64
}

/// Mirrors `fn_rtrim`.
extern "C" fn cfml_rtrim_boxed(tagged: i64) -> i64 {
    use cfml_common::dynamic::CfmlValue;
    let v = unsafe { super::boxed::borrow_tagged(tagged as usize) };
    super::arena::box_into_active(CfmlValue::string(v.as_string().trim_end().to_string())) as i64
}

/// Mirrors `fn_reverse`: `chars().rev().collect()` of the stringified value.
/// Lucee/RustCFML `reverse()` is string-only (arrays go through `arrayReverse`).
extern "C" fn cfml_reverse_boxed(tagged: i64) -> i64 {
    use cfml_common::dynamic::CfmlValue;
    let v = unsafe { super::boxed::borrow_tagged(tagged as usize) };
    let s: String = v.as_string().chars().rev().collect();
    super::arena::box_into_active(CfmlValue::string(s)) as i64
}

/// Mirrors `fn_asc`: first char of stringified arg as its `i64` code point,
/// or `0` for empty. CFML returns `Int`.
extern "C" fn cfml_asc_boxed_i64(tagged: i64) -> i64 {
    let v = unsafe { super::boxed::borrow_tagged(tagged as usize) };
    v.as_string().chars().next().map_or(0, |c| c as i64)
}

/// Mirrors `fn_strip_cr`: drops every `\r` from the stringified arg.
extern "C" fn cfml_strip_cr_boxed(tagged: i64) -> i64 {
    use cfml_common::dynamic::CfmlValue;
    let v = unsafe { super::boxed::borrow_tagged(tagged as usize) };
    super::arena::box_into_active(CfmlValue::string(v.as_string().replace('\r', ""))) as i64
}

/// Mirrors `fn_html_edit_format`: `& < > "` → entity refs.
extern "C" fn cfml_html_edit_format_boxed(tagged: i64) -> i64 {
    use cfml_common::dynamic::CfmlValue;
    let v = unsafe { super::boxed::borrow_tagged(tagged as usize) };
    let s = v
        .as_string()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;");
    super::arena::box_into_active(CfmlValue::string(s)) as i64
}

/// Mirrors `fn_html_code_format`: htmlEditFormat then `<pre>…</pre>` wrap.
extern "C" fn cfml_html_code_format_boxed(tagged: i64) -> i64 {
    use cfml_common::dynamic::CfmlValue;
    let v = unsafe { super::boxed::borrow_tagged(tagged as usize) };
    let inner = v
        .as_string()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;");
    super::arena::box_into_active(CfmlValue::string(format!("<pre>{inner}</pre>"))) as i64
}

/// Mirrors `fn_encode_for_html`: same as htmlEditFormat plus `'` and `/` escapes.
extern "C" fn cfml_encode_for_html_boxed(tagged: i64) -> i64 {
    use cfml_common::dynamic::CfmlValue;
    let v = unsafe { super::boxed::borrow_tagged(tagged as usize) };
    let s = v
        .as_string()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
        .replace('/', "&#x2f;");
    super::arena::box_into_active(CfmlValue::string(s)) as i64
}

// ── v0.99.2 single-arg Boxed→Boxed string shims ─────────────────────────────
// urlEncodedFormat / urlDecode / jsStringFormat — all bit-exact mirrors of
// the cfml-stdlib::builtins::fn_url_encode / fn_url_decode / fn_js_string_format
// implementations. Infallible.

/// Mirrors `fn_url_encode` (registered as `urlEncodedFormat`).
extern "C" fn cfml_url_encoded_format_boxed(tagged: i64) -> i64 {
    use cfml_common::dynamic::CfmlValue;
    use std::fmt::Write;
    let v = unsafe { super::boxed::borrow_tagged(tagged as usize) };
    let s = v.as_string();
    let mut result = String::new();
    for c in s.chars() {
        match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '*' => result.push(c),
            ' ' => result.push('+'),
            _ => {
                for b in c.to_string().as_bytes() {
                    let _ = write!(result, "%{:02X}", b);
                }
            }
        }
    }
    super::arena::box_into_active(CfmlValue::string(result)) as i64
}

/// Mirrors `fn_url_decode`.
extern "C" fn cfml_url_decode_boxed(tagged: i64) -> i64 {
    use cfml_common::dynamic::CfmlValue;
    let v = unsafe { super::boxed::borrow_tagged(tagged as usize) };
    let s = v.as_string();
    let mut result = String::new();
    let mut bytes: Vec<u8> = Vec::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '%' => {
                let hex: String = chars.by_ref().take(2).collect();
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    bytes.push(byte);
                }
                if chars.peek() != Some(&'%') {
                    if let Ok(decoded) = String::from_utf8(bytes.clone()) {
                        result.push_str(&decoded);
                    } else {
                        for b in &bytes {
                            result.push(*b as char);
                        }
                    }
                    bytes.clear();
                }
            }
            '+' => {
                if !bytes.is_empty() {
                    if let Ok(decoded) = String::from_utf8(bytes.clone()) {
                        result.push_str(&decoded);
                    }
                    bytes.clear();
                }
                result.push(' ');
            }
            _ => {
                if !bytes.is_empty() {
                    if let Ok(decoded) = String::from_utf8(bytes.clone()) {
                        result.push_str(&decoded);
                    }
                    bytes.clear();
                }
                result.push(c);
            }
        }
    }
    if !bytes.is_empty() {
        if let Ok(decoded) = String::from_utf8(bytes.clone()) {
            result.push_str(&decoded);
        }
    }
    super::arena::box_into_active(CfmlValue::string(result)) as i64
}

/// Mirrors `fn_js_string_format`: escapes `\`, `'`, `"`, `\n`, `\r`, `\t`.
extern "C" fn cfml_js_string_format_boxed(tagged: i64) -> i64 {
    use cfml_common::dynamic::CfmlValue;
    let v = unsafe { super::boxed::borrow_tagged(tagged as usize) };
    let escaped = v
        .as_string()
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t");
    super::arena::box_into_active(CfmlValue::string(escaped)) as i64
}

// ── v0.99.2 multi-arg Boxed shims ────────────────────────────────────────────
// left / right / mid / repeatString — accept a leading Boxed string and one
// or two Int counts. find / findNoCase return Int from two Boxed strings.
// replace / replaceNoCase 3-arg forms (scope defaults to "one"). All
// infallible — argument coercion via `.max(0)` / `.saturating_sub`.

/// Mirrors `fn_left(string, count)`.
extern "C" fn cfml_left_boxed_int(tagged: i64, count: i64) -> i64 {
    use cfml_common::dynamic::CfmlValue;
    let v = unsafe { super::boxed::borrow_tagged(tagged as usize) };
    let s = v.as_string();
    let n = count.max(0) as usize;
    let chars: Vec<char> = s.chars().collect();
    let out: String = chars[..n.min(chars.len())].iter().collect();
    super::arena::box_into_active(CfmlValue::string(out)) as i64
}

/// Mirrors `fn_right(string, count)`.
extern "C" fn cfml_right_boxed_int(tagged: i64, count: i64) -> i64 {
    use cfml_common::dynamic::CfmlValue;
    let v = unsafe { super::boxed::borrow_tagged(tagged as usize) };
    let s = v.as_string();
    let n = count.max(0) as usize;
    let chars: Vec<char> = s.chars().collect();
    let start = chars.len().saturating_sub(n);
    let out: String = chars[start..].iter().collect();
    super::arena::box_into_active(CfmlValue::string(out)) as i64
}

/// Mirrors `fn_mid(string, start, length)` (3-arg form).
extern "C" fn cfml_mid_boxed_int_int(tagged: i64, start: i64, length: i64) -> i64 {
    use cfml_common::dynamic::CfmlValue;
    let v = unsafe { super::boxed::borrow_tagged(tagged as usize) };
    let s = v.as_string();
    let start = (start.max(1) as usize).saturating_sub(1);
    let length = length.max(0) as usize;
    let chars: Vec<char> = s.chars().collect();
    let out: String = if start >= chars.len() {
        String::new()
    } else {
        let end = (start + length).min(chars.len());
        chars[start..end].iter().collect()
    };
    super::arena::box_into_active(CfmlValue::string(out)) as i64
}

/// Mirrors `fn_repeat_string(string, count)`.
extern "C" fn cfml_repeat_string_boxed_int(tagged: i64, count: i64) -> i64 {
    use cfml_common::dynamic::CfmlValue;
    let v = unsafe { super::boxed::borrow_tagged(tagged as usize) };
    let s = v.as_string();
    let n = count.max(0) as usize;
    super::arena::box_into_active(CfmlValue::string(s.repeat(n))) as i64
}

/// Mirrors `fn_find(substring, string)` (2-arg form). 1-based index of the
/// first occurrence, or 0 if not found.
extern "C" fn cfml_find_boxed_boxed_i64(needle: i64, hay: i64) -> i64 {
    let n = unsafe { super::boxed::borrow_tagged(needle as usize) };
    let h = unsafe { super::boxed::borrow_tagged(hay as usize) };
    let substring = n.as_string();
    let string = h.as_string();
    if let Some(pos) = string.find(&*substring) {
        (pos + 1) as i64
    } else {
        0
    }
}

/// Mirrors `fn_find_no_case(substring, string)`.
extern "C" fn cfml_find_no_case_boxed_boxed_i64(needle: i64, hay: i64) -> i64 {
    let n = unsafe { super::boxed::borrow_tagged(needle as usize) };
    let h = unsafe { super::boxed::borrow_tagged(hay as usize) };
    let substring = n.as_string().to_lowercase();
    let string = h.as_string().to_lowercase();
    if let Some(pos) = string.find(&substring) {
        (pos + 1) as i64
    } else {
        0
    }
}

/// Mirrors `fn_replace(string, find, replaceWith)` with default scope="one"
/// (single replacement, case-sensitive).
extern "C" fn cfml_replace_3_boxed(tag_s: i64, tag_f: i64, tag_r: i64) -> i64 {
    use cfml_common::dynamic::CfmlValue;
    let vs = unsafe { super::boxed::borrow_tagged(tag_s as usize) };
    let vf = unsafe { super::boxed::borrow_tagged(tag_f as usize) };
    let vr = unsafe { super::boxed::borrow_tagged(tag_r as usize) };
    let out = vs.as_string().replacen(&*vf.as_string(), &vr.as_string(), 1);
    super::arena::box_into_active(CfmlValue::string(out)) as i64
}

/// Mirrors `fn_replace_no_case(string, find, replaceWith)` with default scope="one".
extern "C" fn cfml_replace_no_case_3_boxed(tag_s: i64, tag_f: i64, tag_r: i64) -> i64 {
    use cfml_common::dynamic::CfmlValue;
    let vs = unsafe { super::boxed::borrow_tagged(tag_s as usize) };
    let vf = unsafe { super::boxed::borrow_tagged(tag_f as usize) };
    let vr = unsafe { super::boxed::borrow_tagged(tag_r as usize) };
    let string = vs.as_string();
    let find = vf.as_string();
    let replace_with = vr.as_string();
    let find_lower = find.to_lowercase();
    let lower = string.to_lowercase();
    let out = if let Some(pos) = lower.find(&find_lower) {
        let mut result = String::new();
        result.push_str(&string[..pos]);
        result.push_str(&replace_with);
        result.push_str(&string[pos + find.len()..]);
        result
    } else {
        string.to_string()
    };
    super::arena::box_into_active(CfmlValue::string(out)) as i64
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
    // ── v0.92.0 — Boxed-argument string/array shims ──────────────────────
    Shim {
        name: "len",
        args_req: &[KindReq::Boxed],
        args_abi: &[Kind::Boxed],
        ret_kind: Kind::Int,
        sym: "cfml_len_boxed_i64",
        addr: cfml_len_boxed_i64 as *const u8,
    },
    Shim {
        name: "ucase",
        args_req: &[KindReq::Boxed],
        args_abi: &[Kind::Boxed],
        ret_kind: Kind::Boxed,
        sym: "cfml_ucase_boxed",
        addr: cfml_ucase_boxed as *const u8,
    },
    Shim {
        name: "lcase",
        args_req: &[KindReq::Boxed],
        args_abi: &[Kind::Boxed],
        ret_kind: Kind::Boxed,
        sym: "cfml_lcase_boxed",
        addr: cfml_lcase_boxed as *const u8,
    },
    Shim {
        name: "trim",
        args_req: &[KindReq::Boxed],
        args_abi: &[Kind::Boxed],
        ret_kind: Kind::Boxed,
        sym: "cfml_trim_boxed",
        addr: cfml_trim_boxed as *const u8,
    },
    Shim {
        name: "ltrim",
        args_req: &[KindReq::Boxed],
        args_abi: &[Kind::Boxed],
        ret_kind: Kind::Boxed,
        sym: "cfml_ltrim_boxed",
        addr: cfml_ltrim_boxed as *const u8,
    },
    Shim {
        name: "rtrim",
        args_req: &[KindReq::Boxed],
        args_abi: &[Kind::Boxed],
        ret_kind: Kind::Boxed,
        sym: "cfml_rtrim_boxed",
        addr: cfml_rtrim_boxed as *const u8,
    },
    Shim {
        name: "reverse",
        args_req: &[KindReq::Boxed],
        args_abi: &[Kind::Boxed],
        ret_kind: Kind::Boxed,
        sym: "cfml_reverse_boxed",
        addr: cfml_reverse_boxed as *const u8,
    },
    Shim {
        name: "asc",
        args_req: &[KindReq::Boxed],
        args_abi: &[Kind::Boxed],
        ret_kind: Kind::Int,
        sym: "cfml_asc_boxed_i64",
        addr: cfml_asc_boxed_i64 as *const u8,
    },
    Shim {
        name: "stripcr",
        args_req: &[KindReq::Boxed],
        args_abi: &[Kind::Boxed],
        ret_kind: Kind::Boxed,
        sym: "cfml_strip_cr_boxed",
        addr: cfml_strip_cr_boxed as *const u8,
    },
    Shim {
        name: "htmleditformat",
        args_req: &[KindReq::Boxed],
        args_abi: &[Kind::Boxed],
        ret_kind: Kind::Boxed,
        sym: "cfml_html_edit_format_boxed",
        addr: cfml_html_edit_format_boxed as *const u8,
    },
    Shim {
        name: "htmlcodeformat",
        args_req: &[KindReq::Boxed],
        args_abi: &[Kind::Boxed],
        ret_kind: Kind::Boxed,
        sym: "cfml_html_code_format_boxed",
        addr: cfml_html_code_format_boxed as *const u8,
    },
    Shim {
        name: "encodeforhtml",
        args_req: &[KindReq::Boxed],
        args_abi: &[Kind::Boxed],
        ret_kind: Kind::Boxed,
        sym: "cfml_encode_for_html_boxed",
        addr: cfml_encode_for_html_boxed as *const u8,
    },
    // ── v0.99.2 — more single-arg Boxed→Boxed string shims ───────────────
    Shim {
        name: "urlencodedformat",
        args_req: &[KindReq::Boxed],
        args_abi: &[Kind::Boxed],
        ret_kind: Kind::Boxed,
        sym: "cfml_url_encoded_format_boxed",
        addr: cfml_url_encoded_format_boxed as *const u8,
    },
    Shim {
        name: "urldecode",
        args_req: &[KindReq::Boxed],
        args_abi: &[Kind::Boxed],
        ret_kind: Kind::Boxed,
        sym: "cfml_url_decode_boxed",
        addr: cfml_url_decode_boxed as *const u8,
    },
    Shim {
        name: "jsstringformat",
        args_req: &[KindReq::Boxed],
        args_abi: &[Kind::Boxed],
        ret_kind: Kind::Boxed,
        sym: "cfml_js_string_format_boxed",
        addr: cfml_js_string_format_boxed as *const u8,
    },
    // ── v0.99.2 — multi-arg Boxed shims ──────────────────────────────────
    Shim {
        name: "left",
        args_req: &[KindReq::Boxed, KindReq::Int],
        args_abi: &[Kind::Boxed, Kind::Int],
        ret_kind: Kind::Boxed,
        sym: "cfml_left_boxed_int",
        addr: cfml_left_boxed_int as *const u8,
    },
    Shim {
        name: "right",
        args_req: &[KindReq::Boxed, KindReq::Int],
        args_abi: &[Kind::Boxed, Kind::Int],
        ret_kind: Kind::Boxed,
        sym: "cfml_right_boxed_int",
        addr: cfml_right_boxed_int as *const u8,
    },
    Shim {
        name: "mid",
        args_req: &[KindReq::Boxed, KindReq::Int, KindReq::Int],
        args_abi: &[Kind::Boxed, Kind::Int, Kind::Int],
        ret_kind: Kind::Boxed,
        sym: "cfml_mid_boxed_int_int",
        addr: cfml_mid_boxed_int_int as *const u8,
    },
    Shim {
        name: "repeatstring",
        args_req: &[KindReq::Boxed, KindReq::Int],
        args_abi: &[Kind::Boxed, Kind::Int],
        ret_kind: Kind::Boxed,
        sym: "cfml_repeat_string_boxed_int",
        addr: cfml_repeat_string_boxed_int as *const u8,
    },
    Shim {
        name: "find",
        args_req: &[KindReq::Boxed, KindReq::Boxed],
        args_abi: &[Kind::Boxed, Kind::Boxed],
        ret_kind: Kind::Int,
        sym: "cfml_find_boxed_boxed_i64",
        addr: cfml_find_boxed_boxed_i64 as *const u8,
    },
    Shim {
        name: "findnocase",
        args_req: &[KindReq::Boxed, KindReq::Boxed],
        args_abi: &[Kind::Boxed, Kind::Boxed],
        ret_kind: Kind::Int,
        sym: "cfml_find_no_case_boxed_boxed_i64",
        addr: cfml_find_no_case_boxed_boxed_i64 as *const u8,
    },
    Shim {
        name: "replace",
        args_req: &[KindReq::Boxed, KindReq::Boxed, KindReq::Boxed],
        args_abi: &[Kind::Boxed, Kind::Boxed, Kind::Boxed],
        ret_kind: Kind::Boxed,
        sym: "cfml_replace_3_boxed",
        addr: cfml_replace_3_boxed as *const u8,
    },
    Shim {
        name: "replacenocase",
        args_req: &[KindReq::Boxed, KindReq::Boxed, KindReq::Boxed],
        args_abi: &[Kind::Boxed, Kind::Boxed, Kind::Boxed],
        ret_kind: Kind::Boxed,
        sym: "cfml_replace_no_case_3_boxed",
        addr: cfml_replace_no_case_3_boxed as *const u8,
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
    fn boxed_overloads_match_only_boxed_args() {
        use super::super::analysis::Kind;
        // Single-arg Boxed shims: must accept exactly Boxed; Int/Float must
        // miss. Covers the v0.99.0 / v0.99.1 / v0.99.2 surface.
        for name in [
            "len",
            "ucase",
            "lcase",
            "trim",
            "ltrim",
            "rtrim",
            "reverse",
            "asc",
            "stripcr",
            "htmleditformat",
            "htmlcodeformat",
            "encodeforhtml",
            "urlencodedformat",
            "urldecode",
            "jsstringformat",
        ] {
            assert!(
                lookup_overload(name, &[Kind::Boxed]).is_some(),
                "{name}(Boxed) must match"
            );
            assert!(
                lookup_overload(name, &[Kind::Int]).is_none(),
                "{name}(Int) must not match"
            );
            assert!(
                lookup_overload(name, &[Kind::Float]).is_none(),
                "{name}(Float) must not match"
            );
        }
        // len + asc return Int; the case/trim/format family return Boxed.
        for name in ["len", "asc"] {
            let idx = lookup_overload(name, &[Kind::Boxed]).unwrap();
            assert_eq!(SHIMS[idx].ret_kind, Kind::Int, "{name} ret must be Int");
        }
        for name in [
            "ucase",
            "lcase",
            "trim",
            "ltrim",
            "rtrim",
            "reverse",
            "stripcr",
            "htmleditformat",
            "htmlcodeformat",
            "encodeforhtml",
            "urlencodedformat",
            "urldecode",
            "jsstringformat",
        ] {
            let idx = lookup_overload(name, &[Kind::Boxed]).unwrap();
            assert_eq!(SHIMS[idx].ret_kind, Kind::Boxed, "{name} ret must be Boxed");
        }

        // Multi-arg Boxed shims (v0.99.2).
        // (Boxed, Int) → Boxed: left, right, repeatString.
        for name in ["left", "right", "repeatstring"] {
            assert!(
                lookup_overload(name, &[Kind::Boxed, Kind::Int]).is_some(),
                "{name}(Boxed, Int) must match"
            );
            assert!(
                lookup_overload(name, &[Kind::Int, Kind::Int]).is_none(),
                "{name}(Int, Int) must not match"
            );
            let idx = lookup_overload(name, &[Kind::Boxed, Kind::Int]).unwrap();
            assert_eq!(SHIMS[idx].ret_kind, Kind::Boxed);
        }
        // (Boxed, Int, Int) → Boxed: mid.
        {
            let idx = lookup_overload("mid", &[Kind::Boxed, Kind::Int, Kind::Int])
                .expect("mid(Boxed,Int,Int) must match");
            assert_eq!(SHIMS[idx].ret_kind, Kind::Boxed);
            assert!(lookup_overload("mid", &[Kind::Boxed, Kind::Int]).is_none());
        }
        // (Boxed, Boxed) → Int: find, findNoCase.
        for name in ["find", "findnocase"] {
            let idx = lookup_overload(name, &[Kind::Boxed, Kind::Boxed])
                .unwrap_or_else(|| panic!("{name}(Boxed,Boxed) must match"));
            assert_eq!(SHIMS[idx].ret_kind, Kind::Int);
            assert!(lookup_overload(name, &[Kind::Int, Kind::Int]).is_none());
        }
        // (Boxed, Boxed, Boxed) → Boxed: replace, replaceNoCase.
        for name in ["replace", "replacenocase"] {
            let idx = lookup_overload(name, &[Kind::Boxed, Kind::Boxed, Kind::Boxed])
                .unwrap_or_else(|| panic!("{name}(Boxed,Boxed,Boxed) must match"));
            assert_eq!(SHIMS[idx].ret_kind, Kind::Boxed);
            assert!(lookup_overload(name, &[Kind::Boxed, Kind::Boxed]).is_none());
        }
    }

    #[test]
    fn boxed_shims_match_interpreter() {
        use super::super::arena::{Arena, ArenaGuard};
        use super::super::boxed;
        use cfml_common::dynamic::CfmlValue;

        let mut arena = Arena::new();
        let _g = ArenaGuard::install(&mut arena);

        // len on a String returns byte length.
        let s = boxed::box_value(CfmlValue::string("héllo")) as i64;
        assert_eq!(cfml_len_boxed_i64(s), "héllo".len() as i64);
        // len on Int stringifies and counts chars: "12345" → 5.
        let i = boxed::box_value(CfmlValue::Int(12345)) as i64;
        assert_eq!(cfml_len_boxed_i64(i), 5);
        // len on Array.
        let a = boxed::box_value(CfmlValue::Array(cfml_common::dynamic::CfmlArray::new(vec![
            CfmlValue::Int(1),
            CfmlValue::Int(2),
            CfmlValue::Int(3),
        ]))) as i64;
        assert_eq!(cfml_len_boxed_i64(a), 3);

        // ucase / lcase / trim / ltrim / rtrim — produce arena-allocated boxes
        // whose underlying value matches the interpreter.
        let mixed = boxed::box_value(CfmlValue::string("  AbCd  ")) as i64;
        let upper = cfml_ucase_boxed(mixed);
        let v = unsafe { boxed::borrow_tagged(upper as usize) };
        assert!(matches!(v, CfmlValue::String(s) if s.as_str() == "  ABCD  "));

        let lower = cfml_lcase_boxed(mixed);
        let v = unsafe { boxed::borrow_tagged(lower as usize) };
        assert!(matches!(v, CfmlValue::String(s) if s.as_str() == "  abcd  "));

        let t = cfml_trim_boxed(mixed);
        let v = unsafe { boxed::borrow_tagged(t as usize) };
        assert!(matches!(v, CfmlValue::String(s) if s.as_str() == "AbCd"));

        let lt = cfml_ltrim_boxed(mixed);
        let v = unsafe { boxed::borrow_tagged(lt as usize) };
        assert!(matches!(v, CfmlValue::String(s) if s.as_str() == "AbCd  "));

        let rt = cfml_rtrim_boxed(mixed);
        let v = unsafe { boxed::borrow_tagged(rt as usize) };
        assert!(matches!(v, CfmlValue::String(s) if s.as_str() == "  AbCd"));

        // reverse — chars().rev() on the stringified value.
        let rev_in = boxed::box_value(CfmlValue::string("abcdé")) as i64;
        let rev = cfml_reverse_boxed(rev_in);
        let v = unsafe { boxed::borrow_tagged(rev as usize) };
        assert!(matches!(v, CfmlValue::String(s) if s.as_str() == "édcba"));

        // asc — first char as i64; empty string → 0.
        assert_eq!(cfml_asc_boxed_i64(rev_in), 'a' as i64);
        let empty = boxed::box_value(CfmlValue::string("")) as i64;
        assert_eq!(cfml_asc_boxed_i64(empty), 0);

        // stripCr — \r dropped, \n retained.
        let cr_in = boxed::box_value(CfmlValue::string("a\r\nb\rc")) as i64;
        let stripped = cfml_strip_cr_boxed(cr_in);
        let v = unsafe { boxed::borrow_tagged(stripped as usize) };
        assert!(matches!(v, CfmlValue::String(s) if s.as_str() == "a\nbc"));

        // htmlEditFormat — & < > " escaped, ' and / left alone.
        let html_in = boxed::box_value(CfmlValue::string("a<b>&c\"d'/e")) as i64;
        let edited = cfml_html_edit_format_boxed(html_in);
        let v = unsafe { boxed::borrow_tagged(edited as usize) };
        assert!(
            matches!(v, CfmlValue::String(s) if s.as_str() == "a&lt;b&gt;&amp;c&quot;d'/e")
        );

        // htmlCodeFormat — wraps htmlEditFormat in <pre>…</pre>.
        let coded = cfml_html_code_format_boxed(html_in);
        let v = unsafe { boxed::borrow_tagged(coded as usize) };
        assert!(
            matches!(v, CfmlValue::String(s) if s.as_str() == "<pre>a&lt;b&gt;&amp;c&quot;d'/e</pre>")
        );

        // encodeForHtml — adds ' and / escapes on top of htmlEditFormat.
        let encoded = cfml_encode_for_html_boxed(html_in);
        let v = unsafe { boxed::borrow_tagged(encoded as usize) };
        assert!(
            matches!(v, CfmlValue::String(s) if s.as_str() == "a&lt;b&gt;&amp;c&quot;d&#x27;&#x2f;e")
        );

        drop(_g);
        // Reclaim the manually-boxed inputs; arena drains the shim outputs.
        drop(unsafe { boxed::reclaim_tagged(s as usize) });
        drop(unsafe { boxed::reclaim_tagged(i as usize) });
        drop(unsafe { boxed::reclaim_tagged(a as usize) });
        drop(unsafe { boxed::reclaim_tagged(mixed as usize) });
        drop(unsafe { boxed::reclaim_tagged(rev_in as usize) });
        drop(unsafe { boxed::reclaim_tagged(empty as usize) });
        drop(unsafe { boxed::reclaim_tagged(cr_in as usize) });
        drop(unsafe { boxed::reclaim_tagged(html_in as usize) });
        arena.drain_except(None);
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
