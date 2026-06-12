# RustCFML JIT — Design & Implementation Notes

A multi-tier, **opt-in** Cranelift JIT for the RustCFML VM, in the spirit of
MatchBox (the Rust BoxLang runtime). The stack-based bytecode interpreter in
`crates/cfml-vm/src/lib.rs` remains the default execution engine and the
universal fallback; the JIT only ever *replaces* interpretation when it can
prove the result is identical.

> Status: **Tier 1 (integer numeric kernels) + Tier 1.5 (floating-point
> kernels) shipped behind `--features jit`.** Measured ~12× on a hot integer
> kernel and ~7.8× on a hot float kernel; the full CFML suite (3536 assertions)
> passes identically with the JIT forced on for every eligible function.
>
> **Tier 1.5 (f64) summary.** A `Kind` lattice `Int | Float | Bool` (shared by
> `analysis.rs` + `translate.rs`). Args stay integer at the ABI (the engine bails
> unless every arg is `CfmlValue::Int`), so float-ness arises only *inside* a
> function — from `Double` literals, the `/` operator (always `f64`; **bails on a
> zero divisor**, which the interpreter throws on), or a float-typed local. Each
> local slot is *uniformly* `Int` or `Float` via a monotonic Float-upgrade
> fixpoint plus a consistency pass that rejects storing an `Int` value into a
> `Float` slot (a path-dependent type a monomorphic JIT can't model) and pins
> param slots to `Int`. `+ - *` keep wrapping int ops for two ints and promote to
> `fadd/…` on any float operand; `%` is int-only (no Cranelift `frem`); `\`
> always returns Int (`fcvt_to_sint_sat` on float operands). A `Float` return is
> bit-cast `f64`→`i64` and re-wrapped as `CfmlValue::Double` via `f64::from_bits`.
> Not yet covered: `Double` *arguments*, float `%`, and `Pow`.

---

## 1. Goals & non-negotiables

- **Optional Cargo feature `jit`**, *not* in `default`. A normal build, the test
  suite, and every downstream consumer are byte-for-byte unaffected.
- **Disabled entirely on wasm32.** The cranelift crates are gated to
  `cfg(not(target_arch = "wasm32"))`, so enabling `jit` on a wasm target is a
  build-time impossibility (Cloudflare worker / `rustcfml-wasm` never see it).
- **Fast startup / small binary** by default — cranelift is absent unless opted in.
- **Correctness is absolute.** The JIT is a pure optimisation. Anything it can't
  prove safe (an unsupported op, a non-integer argument, a runtime divide-by-zero)
  falls back to the interpreter with no observable difference.

## 2. The impedance mismatch (why Tier 1 is narrow)

The interpreter is deeply dynamic:

- Operand stack: `Vec<CfmlValue>` — a 32-byte tagged union.
- Locals: `IndexMap<String, CfmlValue>` — string-keyed, case-insensitive.
- Almost every op can allocate, take an `Arc`, or touch a `RwLock`.

A JIT that tried to model all of that would just re-implement the interpreter.
So Tier 1 carves out the subset where native code is a pure, register-resident
win and *provably* equivalent: **integer arithmetic and counted loops in
side-effect-free functions.** This is also exactly where the existing fused
super-instructions (`ForLoopStep`, `JumpIfLocalCmpConstFalse`, `AddLocalConst`,
`MulLocalConst`, `Increment`/`Decrement`) already point.

## 3. Tiering

| Tier | Engine | Scope |
|------|--------|-------|
| 0 | Interpreter | Everything. Default + fallback. |
| **1** | **Cranelift** | **Whole functions whose every reachable op is integer-only, side-effect-free arithmetic + counted loops. Shipped.** |
| **1.5** | **Cranelift** | **Adds floating-point kernels: `Double` literals, `/`, and float-typed locals (integer args only). Shipped.** |
| 2+ | Cranelift (future) | `Double` args + float `%`/`Pow`; mixed int/double with type guards + deopt; string concat; JIT→JIT / JIT→native calls. |

A future structural step (not done) is extracting the interpreter loop into
`vm/interpreter.rs`; today it stays in `lib.rs` (~14k lines) to avoid a risky
refactor.

## 4. Value model & the ABI boundary

Tier 1 represents every operand-stack slot and every local as a native `i64`.

- Eligibility forbids `Double` literals and non-integer ops, so locals provably
  never hold a non-integer ⇒ **no per-op type guards inside the loop** (unlike
  MatchBox, which JITs polymorphic code and needs NaN-box tag checks).
- Compiled functions use this ABI:

  ```rust
  unsafe extern "C" fn(args: *const i64, nargs: i64, bail: *mut i64) -> i64
  ```

  `args` is the unwrapped `CfmlValue::Int` arguments in declaration order;
  `*bail` is set to `1` to request deopt; the `i64` return is the result (valid
  only when `*bail == 0`). This is adapted from MatchBox's
  `fn(*mut u64 locals, *const heap, *mut u64 out) -> u64`: RustCFML locals are an
  `IndexMap`, not a contiguous slot array, so we pass the args in and return the
  result directly instead of through a locals pointer.
- The Rust-side trampoline (`run_compiled` in `jit/mod.rs`) refuses (→ interpret)
  unless every argument is `CfmlValue::Int` and `args.len() == params.len()`; on
  success it re-wraps the `i64` as `CfmlValue::Int`.

### Arithmetic & deopt

- Wrapping `iadd`/`isub`/`imul` — **bit-exact** with the interpreter, which does
  `CfmlValue::Int(i + j)` (`lib.rs:2506`), i.e. Rust `+`, which wraps in release
  builds (CFML here does **not** promote int→double on overflow).
- `Mod`/`IntDiv` branch to a shared **bail block** when the divisor is `0` *or*
  on the `INT_MIN / -1` case (which Cranelift's `sdiv`/`srem` would trap on). The
  bail block stores `1` to `*bail` and returns; the engine then re-runs the
  **interpreter** on the same `(func, args)`. Because a Tier-1 function is pure,
  re-running from scratch yields an identical result — the only "effect" is the
  return value.

## 5. Why whole-function "bail and re-interpret" is safe

Tier-1 eligibility excludes every side-effecting op (calls, method/property/index
ops, globals, output, include, throw, closures, heap mutation). A pure function
of its arguments produces the same result however many times it runs, so a
runtime deopt simply discards the partial native execution and lets the
interpreter compute the answer. No on-stack replacement, no side-exit state.

## 6. Static analysis (`jit/analysis.rs`) — the correctness core

All checks run over the **reachable** CFG only, so a dead trailing `Null; Return`
epilogue (which codegen appends to every function) never disqualifies a function.

1. **Pre-flight** — reject `__main__` and any function with defaulted params
   (Tier 1 binds args positionally).
2. **Leaders & basic blocks** — leaders = ip 0, every branch target, and the ip
   after every branch/return. Each leader maps to a half-open `[start, end)` block.
3. **Reachability** — BFS from entry over CFG edges (reading only the structural
   terminator). Unreachable blocks are dropped and never validated, so their ops
   (e.g. the dead `Null`) are irrelevant.
4. **Op-subset + reserved-scope check** — only the supported ops (below); any
   other op, or a `LoadLocal`/`StoreLocal`/`DeclareLocal` of a reserved scope
   name (`variables`, `arguments`, `this`, …), rejects the function.
5. **Operand-stack discipline + value kinds** — within each block the operand
   stack starts and ends empty (true for structured CFML). A two-valued kind
   lattice (`Int` / `Bool`) is tracked: comparison/logical results are `Bool` and
   may only be consumed by a branch or another logical op — **never stored into a
   local nor returned**. This guarantees every local and every return value is an
   integer (so re-wrapping as `CfmlValue::Int` is correct, and a function that
   returns a boolean stays interpreted). `Pop` tolerates an empty stack, mirroring
   the interpreter's `stack.pop()`-ignores-`None` (codegen emits a spurious
   statement-level `Pop` after a value-less `StoreLocal`).
6. **Definite assignment** — a forward dataflow (intersection at merges; entry
   seeded with params). A local read on a path where it may be unassigned rejects
   the function, preserving the interpreter's "undefined variable ⇒ error".
7. **No fall-off** — control cannot reach the end of the body without a `Return`
   (which would otherwise return CFML null, not an integer).

### Supported op subset

`Integer`, `True`/`False`, `LoadLocal`/`StoreLocal`/`DeclareLocal` (non-scope),
`Add`, `Sub`, `Mul`, `Mod`, `IntDiv`, `Negate`, `Eq`/`Neq`/`Lt`/`Lte`/`Gt`/`Gte`,
`And`/`Or`/`Not`/`Xor`, `Increment`/`Decrement`, `AddLocalConst`/`MulLocalConst`,
`JumpIfLocalCmpConstFalse`, `ForLoopStep`, `Jump`/`JumpIfFalse`/`JumpIfTrue`,
`Pop`/`Dup`, `Return` (with a value), `LineInfo` (ignored). Anything else ⇒ the
function is cached `Unjittable` and always interpreted.

## 7. Translation (`jit/translate.rs`) — Cranelift gotchas

- **Locals are Cranelift `Variable`s** (`def_var`/`use_var`); Cranelift builds
  SSA / phis automatically. We keep a `slot → Variable` table because in
  cranelift 0.132 `declare_var(ty)` *returns* a fresh `Variable` (it no longer
  takes a caller-chosen index — a change from the 0.129 MatchBox used).
- **Belt-and-suspenders zero-init**: every local is `def_var`'d to `0` in the
  prologue so a `use_var` is always well-formed on every path. The
  definite-assignment analysis proves that `0` is never actually observed.
- **The operand stack is a compile-time `Vec<Value>`**, reset per block; the
  analysis guarantees it is empty at block boundaries, so nothing crosses and we
  never need block params for it (avoids MatchBox's spill-to-stack-slot dance).
- **Block sealing**: we create all blocks up front and call `seal_all_blocks()`
  once at the end — the simplest correct strategy (loop back-edges and the
  mid-block divide-guard split blocks are all sealed together at finalize).
- **Divide guard** splits the current block: `brif(bad, bail_block, cont)` then
  continues in `cont`. The compile-time operand stack carries SSA `Value`s across
  the split unchanged (entry dominates).
- **Calling convention**: `make_signature()` uses the host ISA's default C
  convention, which matches Rust's `extern "C"` on the host — so the transmuted
  function pointer is ABI-correct.

## 8. Engine, hotness & cache (`jit/mod.rs`)

- `JitEngine` owns the `JITModule` (and thus all executable memory), the
  `FunctionBuilderContext`, a `HotnessTracker`, and a per-`global_id` cache
  (`Unjittable | Compiled(ptr)`).
- **Hotness**: invocations are counted by `BytecodeFunction.global_id`. On the
  call that crosses the threshold (default 50, env `RUSTCFML_JIT_THRESHOLD`), the
  function is analysed and compiled exactly once; the outcome is cached. Cold and
  rejected functions cost one hashmap probe.
- The engine is `!Send` (so is the VM already). Child `cfthread` VMs are built
  fresh from `ThreadSeed`, so each gets its own engine — nothing is shared across
  threads.

## 9. Integration with the VM

`crates/cfml-vm/src/lib.rs`, all `#[cfg(feature = "jit")]` (MatchBox pattern):

- `mod jit;`
- field `jit: Option<jit::JitEngine>` on `CfmlVirtualMachine`.
- `jit: jit::JitEngine::maybe_new()` in `new()` — `None` if `RUSTCFML_JIT=0` or
  the host ISA can't initialise.
- At the **top of `execute_function_with_args`** (before the recursion guard):

  ```rust
  #[cfg(feature = "jit")]
  {
      if let Some(engine) = self.jit.as_mut() {
          if let Some(result) = engine.try_call(func, &args) {
              return result;
          }
      }
  }
  ```

  `func`/`args` are the caller's, not borrowed from `self.jit`, so there is no
  borrow conflict. With the feature off the whole block vanishes — a literal
  no-op, no cranelift in the dependency tree.

There is also `pub fn jit_compiled_count(&self)` (cfg-gated) for observability /
tests.

## 10. Verification

- `jit/analysis.rs` unit tests (8): accept arithmetic & counted loops; reject
  reserved scopes, unsupported ops, read-before-assign, boolean returns, and a
  reachable void epilogue; ignore a dead trailing `Null`.
- `jit/mod.rs` unit tests (6): compile real CFML to bytecode, then
  `analyze → Backend::compile → call the native pointer`, comparing to the
  closed form — straight-line arithmetic, variable- and const-bound loops,
  factorial, and divide/intdiv-by-zero bail.
- `tests/jit_numeric.rs` (3, `#![cfg(feature = "jit")]`): run real CFML through
  the public VM API with the threshold forced to 1 (so the dispatch hook +
  hotness + cache + trampoline all engage), asserting both the result *and* that
  `jit_compiled_count() >= 1`.
- **Full CFML suite**: `cargo run -- tests/runner.cfm` → 3536/3536; and
  `RUSTCFML_JIT_THRESHOLD=1 cargo run --features jit -- tests/runner.cfm` →
  3536/3536 (identical, with the JIT compiling every eligible function).
- wasm32 (`cfml-worker`, `rustcfml-wasm`) and the default build/tests: unchanged.

### Reproduce the A/B

```bash
cargo build --release -p rustcfml-cli                  # jit off
cargo build --release -p rustcfml-cli --features jit   # jit on
./target/release/rustcfml bench.cfm                    # ~12× faster with jit on
RUSTCFML_JIT=0 ./target/release/rustcfml bench.cfm     # kill-switch → interpreter
```

A hot kernel (`function work(n){ var t=0; for(var i=1;i<=n;i++){ t = t + (i%7) - (i\3) + i*2; } return t; }`
called 3,000,000×) measured **46,276 ms → 3,738 ms** with identical output.

## 11. Out of scope (future tiers)

f64/decimal kernels; mixed int/double with deopt guards; string concatenation;
JIT→JIT and JIT→native calls; on-stack replacement for long-running loops in
`__main__`; extracting the interpreter into `vm/interpreter.rs`.
