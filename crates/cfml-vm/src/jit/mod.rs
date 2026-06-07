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
    /// `f64` result's bit pattern, re-wrapped as `CfmlValue::Double`).
    Compiled(CompiledFn, bool),
}

/// Invocation counters keyed by `BytecodeFunction.global_id`. A function becomes
/// a compilation candidate once its count reaches `threshold`.
struct HotnessTracker {
    counts: FxHashMap<u32, u32>,
    threshold: u32,
}

impl HotnessTracker {
    fn new(threshold: u32) -> Self {
        Self { counts: FxHashMap::default(), threshold }
    }

    /// Record one invocation of `global_id`; return `true` exactly once, on the
    /// call that crosses the threshold, so compilation is attempted a single
    /// time. After that the count is parked at `threshold + 1`.
    fn record_and_is_hot(&mut self, global_id: u32) -> bool {
        let c = self.counts.entry(global_id).or_insert(0);
        if *c > self.threshold {
            return false;
        }
        *c += 1;
        *c == self.threshold + 1
    }
}

/// The JIT engine, owned by the VM (one per VM instance; child cfthread VMs get
/// their own). Holds the Cranelift module that owns all executable memory, the
/// reusable compilation context, the per-function cache, and the profiler.
pub struct JitEngine {
    cache: FxHashMap<u32, CacheEntry>,
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

    /// The dispatch hook, called at the top of `execute_function_with_args`.
    ///
    /// Returns `Some(result)` only when a compiled native body ran to completion
    /// for these exact arguments; in every other case (`not hot yet`, `rejected`,
    /// `args not all Int`, `arity mismatch`, or a runtime `bail`) it returns
    /// `None` and the caller proceeds with the interpreter — so this can never
    /// change behaviour, only skip interpretation when it is provably equivalent.
    pub fn try_call(&mut self, func: &BytecodeFunction, args: &[CfmlValue]) -> Option<CfmlResult> {
        let id = func.global_id;

        // Fast path: already-known outcome.
        match self.cache.get(&id) {
            Some(CacheEntry::Unjittable) => return None,
            Some(CacheEntry::Compiled(f, ret_float)) => {
                return run_compiled(*f, func, args, *ret_float)
            }
            None => {}
        }

        // Not cached yet — only act once the function is hot.
        if !self.hot.record_and_is_hot(id) {
            return None;
        }

        // Crossed the threshold: analyse + (maybe) compile, exactly once.
        let entry = match analysis::analyze(func) {
            Some(plan) => {
                let ret_float = matches!(plan.ret_kind, analysis::Kind::Float);
                match self.backend.compile(func, &plan) {
                    Ok(ptr) => CacheEntry::Compiled(ptr, ret_float),
                    Err(_) => CacheEntry::Unjittable,
                }
            }
            None => CacheEntry::Unjittable,
        };
        let run = match &entry {
            CacheEntry::Compiled(f, ret_float) => run_compiled(*f, func, args, *ret_float),
            CacheEntry::Unjittable => None,
        };
        self.cache.insert(id, entry);
        run
    }
}

/// Marshal `args` across the ABI boundary and invoke a compiled body.
///
/// Returns `None` (→ interpret) unless every argument is a `CfmlValue::Int`, the
/// argument count matches the function's arity, and the callee did not set the
/// bail flag. On success the `i64` result is re-wrapped as `CfmlValue::Int`, or
/// as `CfmlValue::Double` (via `f64::from_bits`) when `ret_float` is set.
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
            _ => return None, // non-integer argument → let the interpreter handle it
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

    /// Analyse + compile `func`, then invoke the native body with `args`.
    /// Returns `(result, bailed)`.
    fn jit_run(func: &BytecodeFunction, args: &[i64]) -> (i64, bool) {
        let plan = analysis::analyze(func).expect("function should be JIT-eligible");
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
        let plan = analysis::analyze(func).expect("function should be JIT-eligible");
        assert_eq!(plan.ret_kind, analysis::Kind::Float, "expected a Float return");
        let mut backend = translate::Backend::new().expect("backend init");
        let ptr = backend.compile(func, &plan).expect("compile");
        let mut bail: i64 = 0;
        let r = unsafe { ptr(args.as_ptr(), args.len() as i64, &mut bail as *mut i64) };
        (f64::from_bits(r as u64), bail != 0)
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
    fn intdiv_truncates_float_operand() {
        // function f(a) { return (a / 2) \ 1; }  → float a/2 truncated to Int.
        let f = compile_fn("function f(a) { return (a / 2) \\ 1; }", "f");
        // 7/2 = 3.5 → \1 = 3 ; 9/2 = 4.5 → 4
        assert_eq!(jit_run(&f, &[7]), (3, false));
        assert_eq!(jit_run(&f, &[9]), (4, false));
    }
}
