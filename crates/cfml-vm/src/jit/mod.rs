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

use cfml_codegen::{BytecodeFunction, BytecodeOp};
use cfml_common::dynamic::CfmlValue;
use cfml_common::vm::CfmlResult;
use indexmap::IndexMap;
use rustc_hash::FxHashMap;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

mod analysis;
mod builtins;
mod osr;
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

/// Composite key for the per-loop OSR cache: `(global_id, region_start_ip)`.
/// One entry per (function, hot-loop site); a subsequent call that observes a
/// different kind layout for the same loop simply bails (no re-specialization).
type OsrKey = (u32, usize);

/// Successfully compiled OSR loop body — see [`osr::CompiledLoop`].
struct OsrCompiled {
    ptr: osr::CompiledLoop,
    slots: Vec<osr::OsrSlot>,
    exit_ip: usize,
    referenced_builtins: Vec<&'static str>,
}

enum OsrCacheEntry {
    Unjittable,
    Compiled(OsrCompiled),
}

/// The JIT engine, owned by the VM (one per VM instance; child cfthread VMs get
/// their own). Holds the Cranelift module that owns all executable memory, the
/// reusable compilation context, the per-function cache, and the profiler.
///
/// OSR (On-Stack Replacement) state lives alongside the whole-function cache:
/// hot loops in `__main__` (or any non-eligible enclosing function) compile
/// their body region to native code on a separate `osr_cache`, sharing the
/// single [`translate::Backend`] (and therefore the single JIT module, shim
/// symbols, and executable memory). See `JIT_OSR_DESIGN.md`.
pub struct JitEngine {
    cache: FxHashMap<CacheKey, CacheEntry>,
    hot: HotnessTracker,
    backend: translate::Backend,
    /// Per-(function, loop-start) cache. Once a loop is compiled or marked
    /// `Unjittable` it stays that way for the life of this engine.
    osr_cache: FxHashMap<OsrKey, OsrCacheEntry>,
    /// Hotness counter for OSR loop sites. A loop becomes a compilation
    /// candidate when its back-edge has been observed `threshold` times. Uses
    /// the same threshold as whole-function JIT; loop back-edges trip the
    /// counter faster than function calls, so OSR engages almost immediately
    /// on any loop that runs more than a handful of iterations.
    osr_hot: FxHashMap<OsrKey, u32>,
    /// Test/introspection counter — incremented exactly when a loop body is
    /// successfully compiled to native code. Distinct from the whole-fn
    /// counter so e2e tests can assert OSR specifically fired.
    osr_compiled: usize,
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
            osr_cache: FxHashMap::default(),
            osr_hot: FxHashMap::default(),
            osr_compiled: 0,
        })
    }

    /// Count of OSR-compiled loop bodies currently in cache. Used by tests to
    /// confirm OSR (not just whole-fn JIT) actually fired.
    pub fn osr_compiled_count(&self) -> usize {
        self.osr_compiled
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

    /// Try to run a hot loop natively via OSR. Returns:
    /// * `Some(exit_ip)` — the compiled body ran to completion. `locals`
    ///   and `closure_env` have been updated with the post-loop slot values;
    ///   the caller must advance the interpreter `ip` to this value.
    /// * `None` — either not hot yet, not eligible, kind mismatch, runtime
    ///   bail (divide-by-zero), or a shadowed builtin. In every case the
    ///   interpreter must continue in-place: `locals`/`closure_env` may have
    ///   been *partially* updated (bail path writes back current slot
    ///   values), so the natural fall-through — `ip = ForLoopStep.target`
    ///   to re-execute the body once more — is the safe resume point.
    ///
    /// `is_shadowed` is consulted both on the cache-hit fast path *and* on
    /// the first call after compilation — same model as `try_call`. A
    /// user-defined `function abs(x){}` wins over the JIT'd native call.
    ///
    /// Region semantics (see `JIT_OSR_DESIGN.md`):
    /// * `region_start` = `ForLoopStep.target` (loop body start).
    /// * `region_end_excl` = `ForLoopStep_ip + 1` (the dispatch loop's `ip`
    ///   value AFTER it incremented past the ForLoopStep op).
    /// * The interpreter must have already executed its own step (counter +=
    ///   1) and computed `matched == true` before calling this — the OSR
    ///   body enters at `region_start` and assumes the counter is at the
    ///   value it should run the body with.
    pub fn try_run_loop(
        &mut self,
        func: &BytecodeFunction,
        region_start: usize,
        region_end_excl: usize,
        locals: &mut IndexMap<String, CfmlValue>,
        closure_env: Option<&Arc<RwLock<IndexMap<String, CfmlValue>>>>,
        is_shadowed: &mut dyn FnMut(&str) -> bool,
    ) -> Option<usize> {
        let key: OsrKey = (func.global_id, region_start);

        // Fast path: known outcome.
        match self.osr_cache.get(&key) {
            Some(OsrCacheEntry::Unjittable) => return None,
            Some(OsrCacheEntry::Compiled(c)) => {
                for n in &c.referenced_builtins {
                    if is_shadowed(n) {
                        return None;
                    }
                }
                return run_osr_compiled(c, locals, closure_env);
            }
            None => {}
        }

        // Hotness: a back-edge fires this many times before we look.
        let threshold = self.hot.threshold;
        let count = self.osr_hot.entry(key).or_insert(0);
        if *count > threshold {
            // Already counted past threshold but cache miss — shouldn't happen
            // outside races; defensive return.
            return None;
        }
        *count += 1;
        if *count <= threshold {
            return None;
        }

        // Crossed threshold — analyse + (maybe) compile, exactly once.
        let caller_kinds = build_caller_kinds(func, region_start, region_end_excl, locals);
        let entry = match osr::analyze_loop(func, region_start, region_end_excl, &caller_kinds) {
            Some(plan) => match osr::compile_loop(&mut self.backend, func, &plan) {
                Ok(ptr) => {
                    self.osr_compiled += 1;
                    OsrCacheEntry::Compiled(OsrCompiled {
                        ptr,
                        slots: plan.slots,
                        exit_ip: plan.region_end_excl,
                        referenced_builtins: plan.referenced_builtins,
                    })
                }
                Err(_) => OsrCacheEntry::Unjittable,
            },
            None => OsrCacheEntry::Unjittable,
        };
        let run = match &entry {
            OsrCacheEntry::Compiled(c) => {
                if c.referenced_builtins.iter().any(|n| is_shadowed(n)) {
                    None
                } else {
                    run_osr_compiled(c, locals, closure_env)
                }
            }
            OsrCacheEntry::Unjittable => None,
        };
        self.osr_cache.insert(key, entry);
        run
    }
}

/// Build the caller_kinds map for an OSR analysis attempt: scan the region for
/// local-name references and resolve each against the current value in
/// `locals`. Names whose live value isn't a numeric `Int`/`Double` are
/// silently dropped — the analyser then rejects any region that reads them.
fn build_caller_kinds(
    func: &BytecodeFunction,
    region_start: usize,
    region_end_excl: usize,
    locals: &IndexMap<String, CfmlValue>,
) -> HashMap<String, analysis::Kind> {
    let mut kinds: HashMap<String, analysis::Kind> = HashMap::new();
    for ip in region_start..region_end_excl {
        let name = match &func.instructions[ip] {
            BytecodeOp::LoadLocal(n)
            | BytecodeOp::StoreLocal(n)
            | BytecodeOp::Increment(n)
            | BytecodeOp::Decrement(n)
            | BytecodeOp::AddLocalConst(n, _)
            | BytecodeOp::MulLocalConst(n, _)
            | BytecodeOp::JumpIfLocalCmpConstFalse(n, _, _, _)
            | BytecodeOp::ForLoopStep(n, _, _, _, _)
            | BytecodeOp::DeclareLocal(n) => n,
            _ => continue,
        };
        let lower = name.to_ascii_lowercase();
        if kinds.contains_key(&lower) {
            continue;
        }
        // Case-insensitive lookup against the live locals map.
        let v = locals
            .get(name)
            .or_else(|| locals.iter().find(|(k, _)| k.eq_ignore_ascii_case(&lower)).map(|(_, v)| v));
        let k = match v {
            Some(CfmlValue::Int(_)) => analysis::Kind::Int,
            Some(CfmlValue::Double(_)) => analysis::Kind::Float,
            _ => continue,
        };
        kinds.insert(lower, k);
    }
    kinds
}

/// Marshal each slot of `c` from `locals` into a packed `i64` buffer, invoke
/// the compiled body, then write every slot back to `locals` (and `closure_env`
/// when the env shares the name). Returns `Some(exit_ip)` on the success path
/// and `None` on the bail path — on bail the locals reflect the trapping
/// iteration's pre-failure state, exactly what the interpreter needs to
/// re-execute the body once more.
fn run_osr_compiled(
    c: &OsrCompiled,
    locals: &mut IndexMap<String, CfmlValue>,
    closure_env: Option<&Arc<RwLock<IndexMap<String, CfmlValue>>>>,
) -> Option<usize> {
    // ── Marshal in. A kind mismatch (slot was Int at compile, now Double,
    // or missing entirely) forces a bail to the interpreter.
    let mut buf: Vec<i64> = Vec::with_capacity(c.slots.len());
    for slot in &c.slots {
        // Case-insensitive lookup, matching the interpreter's behaviour.
        let v = locals.get(&slot.name).or_else(|| {
            locals
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(&slot.name))
                .map(|(_, v)| v)
        });
        match (slot.kind, v) {
            (analysis::Kind::Int, Some(CfmlValue::Int(i))) => buf.push(*i),
            (analysis::Kind::Float, Some(CfmlValue::Double(d))) => buf.push(d.to_bits() as i64),
            _ => return None,
        }
    }

    // ── Run.
    let mut bail: i64 = 0;
    // SAFETY: `c.ptr` is a Cranelift-emitted function with matching ABI
    // (`*mut i64`, `*mut i64`) -> `()`; the slot buffer + bail pointer are
    // both valid for the duration of the call.
    unsafe {
        (c.ptr)(buf.as_mut_ptr(), &mut bail as *mut i64);
    }

    // ── Write back every slot — success and bail alike. The compiled body
    // populates buf in both paths so the interpreter sees consistent state.
    for (i, slot) in c.slots.iter().enumerate() {
        let val = match slot.kind {
            analysis::Kind::Int => CfmlValue::Int(buf[i]),
            analysis::Kind::Float => CfmlValue::Double(f64::from_bits(buf[i] as u64)),
            _ => unreachable!("OSR slots are always Int or Float"),
        };
        // The original locals map may have a different-case key (`I` vs `i`);
        // preserve the existing casing rather than introducing a duplicate
        // entry with a different case.
        if let Some(existing_key) = locals
            .keys()
            .find(|k| k.eq_ignore_ascii_case(&slot.name))
            .cloned()
        {
            locals.insert(existing_key, val.clone());
        } else {
            locals.insert(slot.name.clone(), val.clone());
        }
        if let Some(env) = closure_env {
            let mut m = env.write().unwrap();
            // Closure env: write back only if the env already tracks this
            // name (it's a captured local); never widen the env.
            if let Some(existing_key) = m
                .keys()
                .find(|k| k.eq_ignore_ascii_case(&slot.name))
                .cloned()
            {
                m.insert(existing_key, val);
            }
        }
    }

    if bail != 0 {
        None
    } else {
        Some(c.exit_ip)
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
        // `len` is a real CFML builtin but not in the JIT shim allowlist —
        // analysis must reject. (`sin` was the original choice but it's been
        // promoted into the allowlist; pick a name that can't be JIT'd
        // because its semantics aren't pure-numeric.)
        let f = compile_fn("function f(a) { return len(a); }", "f");
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
            osr_cache: rustc_hash::FxHashMap::default(),
            osr_hot: rustc_hash::FxHashMap::default(),
            osr_compiled: 0,
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
