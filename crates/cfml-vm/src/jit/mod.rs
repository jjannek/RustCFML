//! Optional multi-tier JIT for the RustCFML VM (Cranelift backend).
//!
//! This whole module only compiles under `--features jit` on a non-wasm32
//! target (the Cargo manifest gates the cranelift crates off wasm32, and the
//! `mod jit;` declaration in `lib.rs` is `#[cfg(feature = "jit")]`). The
//! stack-based interpreter in `lib.rs` is always the default execution engine
//! and the universal fallback — the JIT is a pure, opt-in optimisation.
//!
//! # Tiering
//! * **Tier 0** — the interpreter (unchanged).
//! * **Tier 1** (this module) — *integer numeric kernels*: a whole
//!   [`BytecodeFunction`] is compiled to native code iff a static scan proves
//!   every op is in a supported, side-effect-free, integer-only subset
//!   (arithmetic + counted loops). Anything else stays on the interpreter.
//!
//! # Correctness model
//! The JIT never changes observable behaviour. A Tier-1 function is pure (no
//! calls, output, globals, throws, or heap mutation), so when native execution
//! hits something it can't represent exactly (currently only divide-by-zero) it
//! sets a *bail* flag and [`JitEngine::try_call`] returns `None`; the caller then
//! runs the interpreter on the same `(func, args)` from scratch. Re-running a
//! pure function yields an identical result. Integer arithmetic uses wrapping
//! i64 ops, bit-exact with the interpreter's `CfmlValue::Int(i + j)` in release
//! builds. See `JIT_DESIGN.md` for the full rationale and gotchas.

use cfml_codegen::BytecodeFunction;
use cfml_common::dynamic::CfmlValue;
use cfml_common::vm::CfmlResult;
use rustc_hash::FxHashMap;

mod analysis;
mod builtins;
mod translate;

/// ABI of every Tier-1 compiled function.
///
/// * `args`  — pointer to `nargs` little-endian `i64`s (the unwrapped
///   `CfmlValue::Int` arguments, in declaration order).
/// * `nargs` — number of valid entries behind `args`.
/// * `bail`  — out-param; the callee stores `1` here to signal *deopt* (fall
///   back to the interpreter) and `0` (or leaves it untouched) on success.
///
/// Returns the `i64` result, valid only when `*bail == 0`.
pub type CompiledFn = unsafe extern "C" fn(args: *const i64, nargs: i64, bail: *mut i64) -> i64;

/// Per-function compilation outcome. A `global_id` absent from the cache simply
/// hasn't crossed the hotness threshold yet.
enum CacheEntry {
    /// The static analysis rejected this function; never attempt it again.
    Unjittable,
    /// Successfully compiled; call this pointer. The `bool` is `true` when the
    /// function's return kind is `Float` (the `i64` the body returns is the
    /// `f64` result's bit pattern, re-wrapped as `CfmlValue::Double`). The
    /// `Vec` lists every allowlisted builtin name the compiled body calls — at
    /// every invocation the engine re-checks the live VM and bails if any of
    /// those names has been shadowed (user-defined fn, global), so a runtime
    /// `function abs(x) { … }` still wins.
    Compiled(CompiledFn, bool, Vec<&'static str>),
}

/// Invocation counters keyed by `(global_id, signature)` — each call
/// specialization warms up independently. A `(func, kinds)` pair becomes a
/// compilation candidate once its count reaches `threshold`.
struct HotnessTracker {
    counts: FxHashMap<CacheKey, u32>,
    threshold: u32,
}

impl HotnessTracker {
    fn new(threshold: u32) -> Self {
        Self { counts: FxHashMap::default(), threshold }
    }

    /// Record one invocation of `key`; return `true` exactly once, on the call
    /// that crosses the threshold, so compilation is attempted a single time.
    /// After that the count is parked at `threshold + 1`.
    fn record_and_is_hot(&mut self, key: CacheKey) -> bool {
        let c = self.counts.entry(key).or_insert(0);
        if *c > self.threshold {
            return false;
        }
        *c += 1;
        *c == self.threshold + 1
    }
}

/// Composite cache key: `(global_id, signature)` where `signature` packs
/// `(nargs, float_bitmap)` so each `(func, param-kind-tuple)` specialization
/// caches independently. A Float bit is set when the corresponding argument
/// arrived as `CfmlValue::Double`. Limited to 32 params (anything larger is
/// not a JIT candidate today; we bail to the interpreter).
type CacheKey = (u32, u64);

const MAX_JIT_PARAMS: usize = 32;

/// Build the cache signature for a call. The low 32 bits hold `nargs`, the
/// high 32 bits a bitmap where bit `i` = 1 means `args[i]` is `Double`.
fn signature_for(args: &[CfmlValue]) -> Option<u64> {
    if args.len() > MAX_JIT_PARAMS {
        return None;
    }
    let mut mask: u64 = 0;
    for (i, a) in args.iter().enumerate() {
        match a {
            CfmlValue::Int(_) => {}
            CfmlValue::Double(_) => mask |= 1u64 << i,
            _ => return None,
        }
    }
    Some(((mask as u64) << 32) | (args.len() as u64))
}

/// The JIT engine, owned by the VM (one per VM instance; child cfthread VMs get
/// their own). Holds the Cranelift module that owns all executable memory, the
/// reusable compilation context, the per-function cache, and the profiler.
pub struct JitEngine {
    cache: FxHashMap<CacheKey, CacheEntry>,
    hot: HotnessTracker,
    backend: translate::Backend,
}

impl JitEngine {
    /// Construct the engine unless disabled. Returns `None` when
    /// `RUSTCFML_JIT=0`/`false`/`off` is set, or when the host ISA can't be
    /// initialised (in which case we silently stay on the interpreter).
    pub fn maybe_new() -> Option<Self> {
        match std::env::var("RUSTCFML_JIT") {
            Ok(v) if matches!(v.trim().to_ascii_lowercase().as_str(), "0" | "false" | "off" | "no") => {
                return None;
            }
            _ => {}
        }
        // Threshold is overridable for benchmarking/tests; default 50 trips
        // quickly without compiling genuinely cold functions.
        let threshold = std::env::var("RUSTCFML_JIT_THRESHOLD")
            .ok()
            .and_then(|s| s.trim().parse::<u32>().ok())
            .unwrap_or(50);
        let backend = translate::Backend::new().ok()?;
        Some(Self {
            cache: FxHashMap::default(),
            hot: HotnessTracker::new(threshold),
            backend,
        })
    }

    /// Number of functions currently holding a compiled native body. Used by
    /// tests and `--features jit` introspection to confirm the JIT fired.
    pub fn compiled_count(&self) -> usize {
        self.cache
            .values()
            .filter(|e| matches!(e, CacheEntry::Compiled(..)))
            .count()
    }

    /// Convenience for tests: a no-op shadowing checker (nothing is shadowed).
    #[cfg(test)]
    fn try_call_unshadowed(
        &mut self,
        func: &BytecodeFunction,
        args: &[CfmlValue],
    ) -> Option<CfmlResult> {
        self.try_call(func, args, &mut |_| false)
    }

    /// The dispatch hook, called at the top of `execute_function_with_args`.
    ///
    /// Returns `Some(result)` only when a compiled native body ran to completion
    /// for these exact arguments; in every other case (`not hot yet`, `rejected`,
    /// `args not all Int`, `arity mismatch`, or a runtime `bail`) it returns
    /// `None` and the caller proceeds with the interpreter — so this can never
    /// change behaviour, only skip interpretation when it is provably equivalent.
    pub fn try_call(
        &mut self,
        func: &BytecodeFunction,
        args: &[CfmlValue],
        is_shadowed: &mut dyn FnMut(&str) -> bool,
    ) -> Option<CfmlResult> {
        // Bail immediately if any arg isn't numeric, or if there are too many.
        let sig = signature_for(args)?;
        let key: CacheKey = (func.global_id, sig);

        // Fast path: already-known outcome for this exact signature.
        match self.cache.get(&key) {
            Some(CacheEntry::Unjittable) => return None,
            Some(CacheEntry::Compiled(f, ret_float, names)) => {
                // Shadowing guard — a user-defined `abs` (etc.) defined after
                // the JIT cached this body must still take precedence.
                for n in names {
                    if is_shadowed(n) {
                        return None;
                    }
                }
                return run_compiled(*f, func, args, *ret_float);
            }
            None => {}
        }

        // Not cached yet — only act once this (func, signature) is hot.
        if !self.hot.record_and_is_hot(key) {
            return None;
        }

        // Build the param-kind vector that drives this specialization.
        let mut kinds: Vec<analysis::Kind> = Vec::with_capacity(args.len());
        for a in args {
            kinds.push(match a {
                CfmlValue::Int(_) => analysis::Kind::Int,
                CfmlValue::Double(_) => analysis::Kind::Float,
                _ => return None, // unreachable given signature_for, but defensive
            });
        }

        // Crossed the threshold: analyse + (maybe) compile, exactly once.
        let entry = match analysis::analyze(func, &kinds) {
            Some(plan) => {
                let ret_float = matches!(plan.ret_kind, analysis::Kind::Float);
                let names = plan.referenced_builtins.clone();
                match self.backend.compile(func, &plan) {
                    Ok(ptr) => CacheEntry::Compiled(ptr, ret_float, names),
                    Err(_) => CacheEntry::Unjittable,
                }
            }
            None => CacheEntry::Unjittable,
        };
        let run = match &entry {
            CacheEntry::Compiled(f, ret_float, names) => {
                // Same shadowing guard as the fast path — applies to the very
                // first call after compilation too.
                if names.iter().any(|n| is_shadowed(n)) {
                    None
                } else {
                    run_compiled(*f, func, args, *ret_float)
                }
            }
            CacheEntry::Unjittable => None,
        };
        self.cache.insert(key, entry);
        run
    }
}

/// Marshal `args` across the ABI boundary and invoke a compiled body.
///
/// Returns `None` (→ interpret) unless every argument is a numeric
/// `CfmlValue::Int` or `CfmlValue::Double`, the argument count matches the
/// function's arity, and the callee did not set the bail flag. `Double` args
/// cross as `f64::to_bits` so the compiled prologue can `load F64` directly.
/// On success the `i64` result is re-wrapped as `CfmlValue::Int`, or as
/// `CfmlValue::Double` (via `f64::from_bits`) when `ret_float` is set.
fn run_compiled(
    f: CompiledFn,
    func: &BytecodeFunction,
    args: &[CfmlValue],
    ret_float: bool,
) -> Option<CfmlResult> {
    // Tier-1 binds exactly the declared params positionally; defaults/var-args
    // are rejected at analysis time, so arity must match precisely.
    if args.len() != func.params.len() {
        return None;
    }
    let mut raw: Vec<i64> = Vec::with_capacity(args.len());
    for a in args {
        match a {
            CfmlValue::Int(i) => raw.push(*i),
            CfmlValue::Double(d) => raw.push(d.to_bits() as i64),
            _ => return None, // non-numeric argument → let the interpreter handle it
        }
    }
    let mut bail: i64 = 0;
    let result = unsafe { f(raw.as_ptr(), raw.len() as i64, &mut bail as *mut i64) };
    if bail != 0 {
        return None; // runtime deopt (e.g. divide-by-zero) → interpret
    }
    if ret_float {
        Some(Ok(CfmlValue::Double(f64::from_bits(result as u64))))
    } else {
        Some(Ok(CfmlValue::Int(result)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cfml_codegen::compiler::CfmlCompiler;
    use cfml_compiler::parser::Parser;

    /// Compile CFML `src` and return the named function's bytecode.
    fn compile_fn(src: &str, name: &str) -> BytecodeFunction {
        let ast = Parser::new(src.to_string()).parse().expect("parse");
        let program = CfmlCompiler::new().compile(ast);
        program
            .functions
            .iter()
            .find(|f| f.name.eq_ignore_ascii_case(name))
            .unwrap_or_else(|| panic!("function {name} not found in program"))
            .as_ref()
            .clone()
    }

    /// Test-helper: default kind vector — every declared param pinned to Int.
    fn int_kinds(func: &BytecodeFunction) -> Vec<analysis::Kind> {
        func.params.iter().map(|_| analysis::Kind::Int).collect()
    }

    /// Analyse + compile `func`, then invoke the native body with `args`.
    /// Returns `(result, bailed)`.
    fn jit_run(func: &BytecodeFunction, args: &[i64]) -> (i64, bool) {
        let kinds = int_kinds(func);
        let plan = analysis::analyze(func, &kinds).expect("function should be JIT-eligible");
        let mut backend = translate::Backend::new().expect("backend init");
        let ptr = backend.compile(func, &plan).expect("compile");
        let mut bail: i64 = 0;
        let r = unsafe { ptr(args.as_ptr(), args.len() as i64, &mut bail as *mut i64) };
        (r, bail != 0)
    }

    /// Analyse + compile a `Float`-returning `func`, invoke it, and re-interpret
    /// the returned bits as `f64` (mirroring `run_compiled`'s ret-float path).
    /// Returns `(f64, bailed)`.
    fn jit_run_f64(func: &BytecodeFunction, args: &[i64]) -> (f64, bool) {
        let kinds = int_kinds(func);
        let plan = analysis::analyze(func, &kinds).expect("function should be JIT-eligible");
        assert_eq!(plan.ret_kind, analysis::Kind::Float, "expected a Float return");
        let mut backend = translate::Backend::new().expect("backend init");
        let ptr = backend.compile(func, &plan).expect("compile");
        let mut bail: i64 = 0;
        let r = unsafe { ptr(args.as_ptr(), args.len() as i64, &mut bail as *mut i64) };
        (f64::from_bits(r as u64), bail != 0)
    }

    /// Analyse + compile `func` with a caller-chosen kind tuple (each `Kind`
    /// matches one declared param). `args` carries raw 8-byte slots — Int as
    /// i64, Double as `f64::to_bits` (mirroring `run_compiled`'s marshaling).
    fn jit_run_kinds(
        func: &BytecodeFunction,
        kinds: &[analysis::Kind],
        args: &[i64],
    ) -> (f64, bool, bool) {
        let plan = analysis::analyze(func, kinds).expect("function should be JIT-eligible");
        let ret_float = plan.ret_kind == analysis::Kind::Float;
        let mut backend = translate::Backend::new().expect("backend init");
        let ptr = backend.compile(func, &plan).expect("compile");
        let mut bail: i64 = 0;
        let r = unsafe { ptr(args.as_ptr(), args.len() as i64, &mut bail as *mut i64) };
        if ret_float {
            (f64::from_bits(r as u64), ret_float, bail != 0)
        } else {
            (r as f64, ret_float, bail != 0)
        }
    }

    #[test]
    fn straight_line_arithmetic_matches() {
        let f = compile_fn("function poly(a, b) { return a * b + a - 1; }", "poly");
        // poly(6, 7) = 42 + 6 - 1 = 47
        assert_eq!(jit_run(&f, &[6, 7]), (47, false));
        // poly(-3, 10) = -30 + -3 - 1 = -34
        assert_eq!(jit_run(&f, &[-3, 10]), (-34, false));
    }

    #[test]
    fn counted_loop_sum_matches() {
        // Variable-bound loop: condition is `i <= n` (not const), exercising
        // LoadLocal/Lte/JumpIfFalse/Increment/Jump.
        let f = compile_fn(
            "function sumTo(n) { var t = 0; for (var i = 1; i <= n; i++) { t = t + i; } return t; }",
            "sumTo",
        );
        for n in [0i64, 1, 10, 100, 1000] {
            let expected = n * (n + 1) / 2;
            assert_eq!(jit_run(&f, &[n]), (expected, false), "sumTo({n})");
        }
    }

    #[test]
    fn const_bound_loop_uses_fused_ops() {
        // Const bound `i <= 100` triggers ForLoopStep + JumpIfLocalCmpConstFalse.
        let f = compile_fn(
            "function sumK() { var t = 0; for (var i = 1; i <= 100; i++) { t = t + i; } return t; }",
            "sumK",
        );
        assert_eq!(jit_run(&f, &[]), (5050, false));
    }

    #[test]
    fn factorial_loop_matches() {
        let f = compile_fn(
            "function fact(n) { var r = 1; for (var i = 2; i <= n; i++) { r = r * i; } return r; }",
            "fact",
        );
        // 10! = 3628800
        assert_eq!(jit_run(&f, &[10]), (3_628_800, false));
    }

    #[test]
    fn divide_by_zero_bails() {
        let f = compile_fn("function dz(a, b) { return a % b; }", "dz");
        assert_eq!(jit_run(&f, &[10, 3]), (1, false));
        let (_r, bailed) = jit_run(&f, &[10, 0]);
        assert!(bailed, "divide-by-zero must set the bail flag");
    }

    #[test]
    fn intdiv_matches_and_bails() {
        let f = compile_fn("function idiv(a, b) { return a \\ b; }", "idiv");
        assert_eq!(jit_run(&f, &[17, 5]), (3, false));
        assert!(jit_run(&f, &[1, 0]).1, "intdiv by zero must bail");
    }

    #[test]
    fn float_divide_returns_double() {
        // function avg(a, b) { return (a + b) / 2; }  → Double
        let f = compile_fn("function avg(a, b) { return (a + b) / 2; }", "avg");
        let (r, bailed) = jit_run_f64(&f, &[3, 4]);
        assert!(!bailed);
        assert_eq!(r, 3.5);
        assert_eq!(jit_run_f64(&f, &[10, 10]).0, 10.0);
    }

    #[test]
    fn float_divide_by_zero_bails() {
        // CFML `/` throws on zero divisor; the JIT must bail → re-interpret throws.
        let f = compile_fn("function d(a, b) { return a / b; }", "d");
        assert_eq!(jit_run_f64(&f, &[7, 2]).0, 3.5);
        assert!(jit_run_f64(&f, &[7, 0]).1, "divide by zero must bail");
    }

    #[test]
    fn float_literal_arithmetic_matches() {
        // function f(a) { return a * 1.5 + 0.25; }  → Double
        let f = compile_fn("function f(a) { return a * 1.5 + 0.25; }", "f");
        assert_eq!(jit_run_f64(&f, &[2]).0, 3.25);
        assert_eq!(jit_run_f64(&f, &[-4]).0, -5.75);
    }

    #[test]
    fn float_accumulator_loop_matches() {
        // Harmonic-ish sum with a Float accumulator and an Int loop counter.
        let f = compile_fn(
            "function h(n) { var s = 0.0; for (var i = 1; i <= n; i++) { s = s + 1 / i; } return s; }",
            "h",
        );
        let (r, bailed) = jit_run_f64(&f, &[4]);
        assert!(!bailed);
        let expected = 1.0 + 0.5 + 1.0 / 3.0 + 0.25;
        assert!((r - expected).abs() < 1e-12, "got {r}, want {expected}");
    }

    #[test]
    fn double_param_runs_specialised() {
        // function f(a, b) { return a * b + 1; }  with a=Double, b=Int.
        // The fixpoint upgrades the result to Float; ret_kind = Float.
        let f = compile_fn("function f(a, b) { return a * b + 1; }", "f");
        let kinds = [analysis::Kind::Float, analysis::Kind::Int];
        let raw: [i64; 2] = [2.5_f64.to_bits() as i64, 4];
        let (r, ret_float, bailed) = jit_run_kinds(&f, &kinds, &raw);
        assert!(!bailed);
        assert!(ret_float, "result of float*int+int is Float");
        assert_eq!(r, 11.0);
    }

    #[test]
    fn double_param_pure_int_op_still_returns_double() {
        // function f(a) { return a + 1; }  with a=Double.
        let f = compile_fn("function f(a) { return a + 1; }", "f");
        let kinds = [analysis::Kind::Float];
        let raw: [i64; 1] = [(-3.25_f64).to_bits() as i64];
        let (r, ret_float, _) = jit_run_kinds(&f, &kinds, &raw);
        assert!(ret_float);
        assert_eq!(r, -2.25);
    }

    #[test]
    fn float_mod_matches_fmod() {
        // function fm(a) { return (a / 1) % 0.3; }  → Double via cfml_fmod shim.
        // `(a/1)` forces Float on the lhs while keeping an int-typed param.
        let f = compile_fn("function fm(a) { return (a / 1) % 0.3; }", "fm");
        let (r, bailed) = jit_run_f64(&f, &[2]);
        assert!(!bailed);
        // 2.0 % 0.3 in IEEE-754: 2.0 = 6*0.3 + 0.2, but fmod truncates toward 0.
        let expected = 2.0_f64 % 0.3_f64;
        assert!((r - expected).abs() < 1e-12, "got {r}, want {expected}");
    }

    #[test]
    fn pow_matches_powf() {
        // function p(a, b) { return a ^ b; }  → Double via cfml_pow shim.
        let f = compile_fn("function p(a, b) { return a ^ b; }", "p");
        assert_eq!(jit_run_f64(&f, &[2, 10]).0, 1024.0);
        assert_eq!(jit_run_f64(&f, &[3, 0]).0, 1.0);
        let (r, _) = jit_run_f64(&f, &[5, 3]);
        assert_eq!(r, 125.0);
    }

    #[test]
    fn pow_with_float_operand() {
        // function p(a) { return (a / 1) ^ 0.5; }  → sqrt(a)
        let f = compile_fn("function p(a) { return (a / 1) ^ 0.5; }", "p");
        let (r, bailed) = jit_run_f64(&f, &[16]);
        assert!(!bailed);
        assert_eq!(r, 4.0);
    }

    // ── Option A: JIT → native builtin calls ────────────────────────────────

    #[test]
    fn abs_int_overload_returns_int() {
        // function f(a) { return abs(a); }  → Int result (the Int overload)
        let f = compile_fn("function f(a) { return abs(a); }", "f");
        assert_eq!(jit_run(&f, &[5]), (5, false));
        assert_eq!(jit_run(&f, &[-5]), (5, false));
        assert_eq!(jit_run(&f, &[0]), (0, false));
    }

    #[test]
    fn abs_float_overload_returns_double() {
        // function f(a) { return abs(a / 1); }  → forces Float, calls
        // cfml_abs_f64, returns Double.
        let f = compile_fn("function f(a) { return abs(a / 1); }", "f");
        let (r, bailed) = jit_run_f64(&f, &[-7]);
        assert!(!bailed);
        assert_eq!(r, 7.0);
    }

    #[test]
    fn min_returns_double_even_for_ints() {
        // CFML's `min(3, 5)` returns Double(3.0). The JIT must match.
        let f = compile_fn("function f(a, b) { return min(a, b); }", "f");
        let (r, bailed) = jit_run_f64(&f, &[3, 5]);
        assert!(!bailed);
        assert_eq!(r, 3.0);
        assert_eq!(jit_run_f64(&f, &[10, 4]).0, 4.0);
    }

    #[test]
    fn max_returns_double() {
        let f = compile_fn("function f(a, b) { return max(a, b); }", "f");
        assert_eq!(jit_run_f64(&f, &[3, 5]).0, 5.0);
        assert_eq!(jit_run_f64(&f, &[-3, -10]).0, -3.0);
    }

    #[test]
    fn nested_builtin_calls_compose() {
        // function f(a) { return max(abs(a), 5); }
        // -7 → max(7, 5) → 7.0; 2 → max(2, 5) → 5.0
        let f = compile_fn("function f(a) { return max(abs(a), 5); }", "f");
        assert_eq!(jit_run_f64(&f, &[-7]).0, 7.0);
        assert_eq!(jit_run_f64(&f, &[2]).0, 5.0);
    }

    #[test]
    fn builtin_inside_loop_matches() {
        // function f(n) { var t = 0; for (var i = 1; i <= n; i++) { t = t + abs(i - 3); } return t; }
        let f = compile_fn(
            "function f(n) { var t = 0; for (var i = 1; i <= n; i++) { t = t + abs(i - 3); } return t; }",
            "f",
        );
        // |1-3|+|2-3|+|3-3|+|4-3|+|5-3| = 2+1+0+1+2 = 6
        assert_eq!(jit_run(&f, &[5]), (6, false));
    }

    #[test]
    fn unknown_builtin_rejects() {
        // `sin` isn't in the allowlist — analysis must reject.
        let f = compile_fn("function f(a) { return sin(a); }", "f");
        let kinds = int_kinds(&f);
        assert!(analysis::analyze(&f, &kinds).is_none());
    }

    #[test]
    fn referenced_builtins_recorded() {
        // The plan must list every allowlisted name the body calls.
        let f = compile_fn(
            "function f(a, b) { return max(abs(a), min(b, 10)); }",
            "f",
        );
        let plan = analysis::analyze(&f, &int_kinds(&f)).expect("eligible");
        let names: std::collections::BTreeSet<&str> =
            plan.referenced_builtins.iter().copied().collect();
        let expected: std::collections::BTreeSet<&str> = ["abs", "max", "min"].into_iter().collect();
        assert_eq!(names, expected);
    }

    #[test]
    fn shadow_check_short_circuits_jit() {
        // First compile so the engine has a cached entry; then call try_call
        // with a shadow-checker that fires for "abs" — must return None.
        let f = compile_fn("function f(a) { return abs(a); }", "f");
        let mut engine = JitEngine {
            cache: rustc_hash::FxHashMap::default(),
            hot: HotnessTracker::new(0), // any non-cached call compiles on the first hit
            backend: translate::Backend::new().expect("backend"),
        };
        let args = vec![CfmlValue::Int(-7)];
        // First call: not hot yet (record_and_is_hot returns true at threshold+1
        // == 1, since threshold=0 → fires on the 1st call).
        let _ = engine.try_call_unshadowed(&f, &args);
        // Now compiled. With shadowing, must bail.
        let mut sh = |n: &str| n.eq_ignore_ascii_case("abs");
        assert!(engine.try_call(&f, &args, &mut sh).is_none());
        // Without shadowing, still works.
        let r = engine
            .try_call_unshadowed(&f, &args)
            .expect("compiled path must run")
            .expect("no runtime error");
        assert!(matches!(r, CfmlValue::Int(7)));
    }

    #[test]
    fn intdiv_truncates_float_operand() {
        // function f(a) { return (a / 2) \ 1; }  → float a/2 truncated to Int.
        let f = compile_fn("function f(a) { return (a / 2) \\ 1; }", "f");
        // 7/2 = 3.5 → \1 = 3 ; 9/2 = 4.5 → 4
        assert_eq!(jit_run(&f, &[7]), (3, false));
        assert_eq!(jit_run(&f, &[9]), (4, false));
    }
}
