# JIT — Phase-2 Plan (post-v0.85.0)

> **STATUS: PARTIALLY SUPERSEDED 2026-06-08.**
> * Phase 2 (`jit_resolve_fn` indirection) — SHIPPED as v0.86.0, commit `cbe2594`.
> * Phase 1 (fallback shims for `+` / concat) — **NOT IMPLEMENTED**. The
>   design here was wrong: a polymorphic operand at `+` has nowhere to live
>   without a tagged-value representation. Replaced by the v0.90.0 phase in
>   `JIT_POLY_DESIGN.md`.
> * Phase 3 (member-access ICs) — **NOT IMPLEMENTED**. Same wall: `obj.prop`
>   returns a `CfmlValue` of unknown kind. Replaced by the v0.91.0 phase in
>   `JIT_POLY_DESIGN.md`.
>
> Kept in-tree for historical context. **Read `JIT_POLY_DESIGN.md` for the
> live plan covering everything past v0.86.0.**

Status (when this was written): **DRAFT — awaiting sign-off before any code lands.**

This document plans the three closing gaps between RustCFML's current JIT and
the MatchBox JIT (which is the same-tier reference codebase we benchmarked
against on 2026-06-08). The gaps, scored by payoff/risk, are:

1. **Fallback shims for `+` / concat** — keep the JIT body alive when one
   operand is non-numeric instead of bailing the whole call.
2. **`jit_resolve_fn` indirection** — unlock mutual recursion (A↔B) and
   forward UDF calls (caller compiles before callee).
3. **Member-access ICs with shape IDs** — JIT `obj.prop` reads via a
   shape-guard + indexed load, with monomorphic inline caches.

Each ships as its own version bump (v0.86.0, v0.87.0, v0.88.0). The whole
plan totals ~1500-2300 LoC across three sessions of focused work.

Read alongside:
- `JIT_DESIGN.md` — Tier-1 / Tier-1.5 base
- `JIT_OSR_DESIGN.md` — OSR Phase 1+2 (shipped)
- `JIT_NEXT_SESSION.md` — short-form handover, current state of `main`

---

## Cross-cutting test bar (the "stricter" bar agreed 2026-06-08)

Every phase must pass all of these before merge:

1. **JIT unit tests** in `crates/cfml-vm/src/jit/*.rs` — narrow tests of the
   new IR-emit / analysis / cache logic. Same style as the existing 54.
2. **JIT e2e tests** in `crates/cfml-vm/tests/jit_numeric.rs` (or a new
   `jit_polymorphic.rs` / `jit_recursion.rs` / `jit_members.rs` as the topic
   demands). Drive the public VM API; compare JIT output to the interpreter
   oracle over a non-trivial program.
3. **Full CFML suite identical**: `tests/runner.cfm` must report the same
   pass count and the same SUMMARY line with `RUSTCFML_JIT=0` and with
   `RUSTCFML_JIT_THRESHOLD=1`. Today: **3581/3581**.
4. **wasm32 build green**: `cargo build -p cfml-worker -p rustcfml-wasm
   --target wasm32-unknown-unknown` must succeed. All Phase-2 code lives
   under the existing `cfg(not(target_arch = "wasm32"))` and `feature = "jit"`
   gates; wasm targets opt out via `default-features = false`.
5. **Differential fuzzing**: a small harness that generates random programs
   from a constrained grammar relevant to the phase (e.g. random arithmetic
   trees for Phase 1, random call graphs for Phase 2, random struct
   read/write programs for Phase 3) and asserts JIT-off output == JIT-on
   output for each. Target: 10,000 random programs per phase, no
   divergences, runs in CI under 60s.
6. **Soak test**: run the full suite 10× back-to-back in a single process
   with `RUSTCFML_JIT_THRESHOLD=1` (or via `serve` mode, hammering the same
   request 10× to exercise warm-then-warmer code paths). No leaks, no
   slowdowns, no crashes. Catches cache-lifetime and Cranelift-context
   reuse bugs that single-run tests miss.
7. **Perf A/B**: a focused micro-bench measuring the new path
   (interpreter vs JIT-on) on a representative kernel. Numbers go into
   the commit message and into `JIT_NEXT_SESSION.md`'s perf table.

The differential-fuzz + soak harnesses themselves should land before
Phase 1 starts, so we exercise the existing JIT first to establish a
known-clean baseline. They live under `crates/cfml-vm/tests/` and are
reusable across phases.

---

## Phase 1 — Fallback shims for `+` / concat *(v0.86.0)*

### Goal

Today the analyser rejects any `Add` whose operands could be polymorphic
(e.g. an `Int + String_that_parses_as_number`). The whole function bails to
the interpreter. MatchBox instead emits the JIT'd op as a runtime call to a
fallback shim that coerces operands or builds a concat result, returning a
status of 0/1 (ok / deopt-this-site-only).

### Scope — what's in

- `cfml_fallback_add(heap_ptr, a_bits, b_bits, out_ptr) -> u64`:
  - If both operands coerce to numeric (`Int`/`Double`/string-of-number),
    return their numeric sum.
  - If either is a string that doesn't parse, perform string concatenation
    and return the new `CfmlValue::String`.
  - Otherwise: write `1` to a deopt flag and bail this *one call*, not the
    function.
- `cfml_fallback_concat(...)`: same shape, string-concat only — used when
  codegen knows the source is the `&` operator (always string).
- Analyser change: `Kind` gains a `Numeric` lattice point above `Int`/`Float`
  but below "anything could land here." `+` between `(Numeric, Numeric)`
  emits the fallback path. `+` between known-monomorphic operands keeps the
  fast path.
- Codegen: at each `Add`/`StringConcat` site, choose direct IR vs a libcall
  to the fallback shim based on the kind lattice.
- Runtime: the fallback shim needs read-only access to the VM heap for the
  string-allocation path — we already pass an engine pointer for Phase-1
  UDF calls, so the same channel is reused.

### Scope — what's out

- No fallback for `-` / `*` / `/` / `%` — these are numeric-only in CFML
  semantics (string operands throw rather than coerce). Keep the bail.
- No fallback for comparison ops (yet) — same reasoning; revisit if
  profiling shows a real win.
- No JIT of pure string functions (`len`, `ucase`, etc.) — separate work.

### Files touched

| File | Change |
|------|--------|
| `crates/cfml-vm/src/jit/analysis.rs` | New `Kind::Numeric`; `+` typing rule; `Plan` records per-call fallback flag |
| `crates/cfml-vm/src/jit/translate.rs` | New libcall registrations; `Add` arm splits monomorphic-fast vs fallback-shim |
| `crates/cfml-vm/src/jit/runtime.rs` *(new)* | `extern "C" fn cfml_fallback_add/concat`; helpers for string-to-number coercion that exactly mirror `CfmlValue::to_number` |
| `crates/cfml-vm/src/jit/mod.rs` | Engine-ptr thread-local extended so runtime shim can reach the heap |
| `crates/cfml-vm/tests/jit_polymorphic.rs` *(new)* | E2E tests: int+int (fast), int+"5" (coerce), int+"hello" (concat), int+arr (deopt) |

### ABI

```rust
// returns 0 = success (out_ptr populated with new CfmlValue::Int | Double | String),
//         1 = deopt this site (interpreter should re-execute from saved IP)
extern "C" fn cfml_fallback_add(
    engine: *mut JitEngine,
    a_bits: u64,           // CfmlValue bit-pattern (no NaN-boxing — just a discriminant + payload)
    b_bits: u64,
    out_ptr: *mut u64,     // written on success
) -> u64;
```

We do **not** introduce NaN-boxing. `a_bits` / `b_bits` are tags into a
small per-call value pool the JIT owns. This is materially cheaper to land
than a full Tier-2 NaN-box (which stays parked — see handover).

### Risks

- Coercion-rule drift between the shim and `CfmlValue::to_number`. Mitigation:
  the shim *calls* the existing coercion helper; it doesn't reimplement it.
- Heap allocation in JIT'd code is new. The interpreter already allocates;
  we just need to confirm the GC (refcount) machinery is safe to drive from
  a non-VM thread context. (It is — `CfmlValue::String` is `Arc<String>`.)

### Expected payoff

Real-world CFML mixes `Int` and `"5"`-shaped strings constantly (form
fields, URL params, query columns). On hot kernels that today bail entirely
because of one polymorphic `+`, expect **10-50× speedup** to come back —
not the full ~300× of pure-monomorphic kernels, but a large fraction of
the previously-uncovered surface.

### Test bar specifics

- Fuzz harness: a grammar that emits random arithmetic-expression trees
  over a mix of `Int`/`Double`/`String_of_number`/`String_random` leaves,
  evaluated inside a hot loop. 10,000 programs, no divergences.
- Soak: full suite 10×; assert `JitEngine::jit_compiled_count` is
  monotonic and stable across runs.

---

## Phase 2 — `jit_resolve_fn` indirection *(v0.87.0)*

### Goal

Today (v0.84.0) UDF→UDF only works for self-recursion (the cache entry is
inserted before the body runs) and leaf-first warm-up (callee must already
be compiled when caller compiles). MatchBox uses an indirect call through
a `gc_id → fn_ptr` table that the dispatcher updates whenever a function
later compiles, so mutual recursion (A↔B) and forward calls (A calls
not-yet-compiled B) both work transparently.

### Scope — what's in

- A per-engine `compiled_fns_by_global_id: HashMap<u32, FnPtr>` map.
  `global_id` already exists from v0.66.0 (stable function identity), so
  no new identity work is needed.
- A runtime helper `cfml_resolve_jit_udf(global_id) -> *const u8`:
  - Returns the compiled function pointer if cached.
  - Returns null if not yet compiled.
- Codegen: at each UDF call site, emit:
  1. Call `cfml_resolve_jit_udf(global_id_literal)`.
  2. If non-null, indirect-call through it with the captured signature.
  3. If null, libcall to the existing dispatcher that falls back to the
     interpreter for *this call only* (not the whole function).
- Remove the speculative-self-recursion machinery from v0.84.0 — the new
  indirect path handles self-calls uniformly (callee = self, lookup finds
  the cache entry we just inserted).
- Topological warm-up is no longer required. Functions compile in the
  order the threshold trips; later callers find them automatically.

### Scope — what's out

- Closure invocation (still calls a closure object, not a plain UDF) —
  separate work, has its own captured-scope machinery.
- Method dispatch (`obj.foo()`) — bundled into Phase 3 because it depends
  on shape-aware property lookup of the bound function.
- Argument-type mismatch handling on indirect call — if a caller's
  bindings say `(Int, Int) -> Int` but the callee was recompiled with a
  different signature (rare, only after deopt), the dispatcher detects
  the mismatch and falls back. Same machinery as today's
  `udf_binding_still_valid`, just consulted at indirect-call time.

### Files touched

| File | Change |
|------|--------|
| `crates/cfml-vm/src/jit/mod.rs` | New `compiled_fns_by_global_id` map; `cfml_resolve_jit_udf` libcall; dispatcher updates on insert |
| `crates/cfml-vm/src/jit/analysis.rs` | UDF call typing no longer requires callee `Compiled` — only requires `global_id` known + signature inferable from caller's arg kinds |
| `crates/cfml-vm/src/jit/translate.rs` | Emit indirect-call sequence: resolve → branch-null → fast-call vs slow-dispatch |
| `crates/cfml-vm/tests/jit_recursion.rs` *(new)* | Mutual recursion (even/odd), 3-cycle (A→B→C→A), forward call (A calls not-yet-compiled B then B's threshold trips) |

### ABI

```rust
// thread-local engine ptr (already exists from v0.84.0)
extern "C" fn cfml_resolve_jit_udf(global_id: u32) -> *const u8;
```

The indirect call is a regular Cranelift `call_indirect` with the
signature declared at the call site (built from the caller's plan's
inferred arg-kind list).

### Risks

- Signature mismatch at runtime: caller built indirect call for
  `(Int, Int) -> Int` but callee compiled for `(Float, Int) -> Float`.
  Mitigation: the resolver returns null in that case (it also checks the
  cached signature), forcing the slow dispatcher.
- Self-recursion still needs the "insert before run" invariant —
  preserved by the same code path as v0.84.0, just generalized.

### Expected payoff

Phase 1 of UDF→UDF (v0.84.0) measured **43× on a 3-level call chain**
and **2600× on self-recursive `fib`**. Phase 2 lifts the
caller-must-precede-callee constraint, expected to make those numbers
the *floor* of UDF-heavy code rather than the ceiling: closer to those
speedups for any call graph, not just leaf-first ones.

### Test bar specifics

- Fuzz harness: random DAG / cycle of UDFs (3-8 functions, mixed
  signatures), called from a hot driver loop. Compare JIT-off vs
  threshold=1 outputs for 10,000 random graphs.
- Soak: confirm `compiled_fns_by_global_id` doesn't leak entries —
  10× full-suite passes leaves the map at the same size as 1× pass for
  the same source code.

---

## Phase 3 — Member-access ICs with shape IDs *(v0.88.0)*

### Goal

Today `obj.prop` is always interpreted — JIT bails on any `GetMember` /
`SetMember` op. MatchBox guards property access with a shape ID (a
monotonic counter the struct backing-store gets when a key set changes),
inline-caches the (shape_id, slot_index) pair at the call site, and on a
hit emits a direct indexed load. On miss it calls a runtime helper that
either updates the IC (still monomorphic) or sets a polymorphic flag.

### Scope — what's in

- `shape_id: u32` field on `CfmlStruct`'s backing record (and same for
  `CfmlComponent`'s `variables` and `this` stores).
- A shape registry in `cfml-common` that maps key-set fingerprints to
  shape IDs. New shape IDs are minted lazily on first inspection.
- Invalidation: any `insert`, `remove`, or rename of a key on a shape
  bumps that backing store to a fresh shape ID. The IC notices the
  mismatch on next read and re-resolves.
- A per-call-site IC slot inside the JIT cache: stores
  `Monomorphic { shape_id, key_index }` or `Polymorphic`.
- Runtime helper `cfml_ic_load_member(struct_ptr, key_intern_id,
  ic_slot_ptr, out_ptr) -> u64`:
  - Returns 0 + updates `out_ptr` on success.
  - Returns 1 on miss / polymorphic / not-a-struct (interpreter resumes).
- Codegen for `GetMember`: emit shape-guard → indexed-load fast path;
  on guard fail, libcall to the IC fallback. `SetMember` follows the
  same pattern with an extra "shape might change" check.

### Scope — what's out

- Method dispatch (`obj.foo(args)`) — uses the same machinery to find
  the function value, then routes through Phase 2's `jit_resolve_fn`.
  Worth doing in this phase if budget allows; otherwise a v0.88.x
  follow-up.
- Polymorphic ICs (more than one cached shape) — first version is
  monomorphic-only. Polymorphic ICs are an obvious follow-up if
  profiling shows real workloads thrashing the IC.
- Array `[idx]` access — different op, separate work.

### Files touched

| File | Change |
|------|--------|
| `crates/cfml-common/src/dynamic.rs` | `CfmlStruct` gets a `shape_id: AtomicU32`; bump on `insert_new_key` / `remove`. `CfmlComponent` mirrors for its two backing stores |
| `crates/cfml-common/src/shape.rs` *(new)* | Shape registry; key-set → shape_id interning; key→slot lookup |
| `crates/cfml-vm/src/jit/analysis.rs` | `GetMember` / `SetMember` admissible when the receiver is `Kind::Struct` or `Kind::Component`; bail otherwise |
| `crates/cfml-vm/src/jit/translate.rs` | Emit shape-guard + indexed load; per-call IC slot allocation; libcall on miss |
| `crates/cfml-vm/src/jit/ic.rs` *(new)* | IC slot layout; `cfml_ic_load_member`; `cfml_ic_store_member`; key interning |
| `crates/cfml-vm/tests/jit_members.rs` *(new)* | Monomorphic hot read, shape evolution invalidates IC, polymorphic forces deopt, component vs struct paths, key absent vs present |

### ABI

```rust
extern "C" fn cfml_ic_load_member(
    receiver_bits: u64,    // CfmlValue::Struct(...) or Component(...) bit-pattern
    key_id: u32,           // interned key identity
    ic_slot: *mut IcSlot,  // mutable cache slot
    out_ptr: *mut u64,     // receives CfmlValue on success
) -> u64;                  // 0 = ok, 1 = deopt
```

`IcSlot` is a small struct (16 bytes: tag + shape_id + key_index +
padding) embedded in the compiled function's data segment, one per
member-access site.

### Risks

- **Shape-evolution coverage.** Many CFML idioms mutate structs (`s.x = ...`
  adds a key if absent). Each shape transition is a fresh shape_id, so
  hot loops that grow a struct will thrash the IC. Mitigation: detect
  "shape-stable site" at analysis time (same op never appends a new key)
  and skip the IC entirely there.
- **Concurrent mutation** from `cfthread` — `CfmlStruct` is already
  `Arc<RwLock>`. Shape bumps must be atomic and ordered with the lock,
  not just `Atomic::fetch_add`. Mitigation: bump under the write lock,
  read shape_id while holding the read lock. The fast-path IC read can
  do a relaxed-load + post-validate-under-lock dance like a Java VM.
- **`cfml-common` change** is the most cross-cutting in this plan —
  every consumer of `CfmlStruct::new` etc. needs to keep compiling. The
  field is private with a getter, so this is mostly a recompile, not
  source edits.

### Expected payoff

Object-heavy CFML — DI containers (WireBox), OO controllers, query
result objects — currently sees ~zero JIT coverage because the analyser
trips on the first `obj.prop`. With monomorphic ICs landing, hot
methods that read 5-10 fields should compile and run **3-15× faster**.
Less than the numeric kernel speedups, but applied to a much larger
swath of real CFML code.

### Test bar specifics

- Fuzz harness: random programs that create a struct, read+write a
  random subset of its keys in a hot loop, occasionally add/remove a
  key. JIT-off output == JIT-on output across 10,000 programs.
- Soak: 10× suite passes with a representative OO benchmark (e.g.
  spin up a WireBox-style injector, resolve 1000 components). Confirm
  IC slots don't leak, shape registry stays bounded.
- A dedicated test for the shape-stable-site analysis: a hot
  read-only loop on a never-mutated struct should compile to a single
  shape-guard + indexed-load and never call the IC fallback again.

---

## Ordering rationale

We do 1 → 2 → 3 in that order because each compounds on the previous:

- Phase 1 (fallback shims) widens the JIT-eligible surface. When
  Phase 2 lands, it benefits from the wider acceptance.
- Phase 2 (`jit_resolve_fn`) makes UDF graphs JIT regardless of
  compile order. When Phase 3 lands, member-access call sites that
  resolve to methods (`obj.foo()`) plug into Phase 2's dispatcher.
- Phase 3 alone — landing first — would be the heaviest single move
  and the most disruptive to `cfml-common`, with the smallest fraction
  of code paths it can JIT today (because so much else still bails).

---

## Out-of-scope (still parked)

- **NaN-boxed Tier-2** (Option C in the handover). Verdict from 2026-06-08
  still stands: ~5k LoC of unsafe Cranelift + side-exit state machine,
  ~20% suite wall-clock for the surface added. Reconsider only after
  Phases 1-3 are in and a measured real-world workload demonstrates a
  polymorphic kernel none of 1-3 can reach.
- **Tier-3 array iteration JIT** (MatchBox's homogeneous-array path).
  CFML arrays are uniformly heterogeneous (`Array<CfmlValue>`), so the
  "float-only array" specialization doesn't drop in. A different shape
  of array-typed JIT is worth considering after Phase 3.
- **Recompile-mode tracking** (MatchBox's `fn_deopt_counts` /
  `fn_recompile_mode`). Useful only if we observe deopt-thrashing in
  the wild; not worth pre-building. Phase 1's fallback shims already
  reduce the deopt blast radius from "whole function" to "this call."

---

## Open questions

- Phase 1's `cfml_fallback_add` mirrors the interpreter's existing
  coercion path. Confirm the coercion helper is `Send`/safely callable
  from a thread holding no VM lock (it should be — it works on a
  `&CfmlValue`).
- Phase 3's shape registry: per-engine, or global? Per-engine is
  simpler (no cross-engine ID collisions); global is faster (no
  registry pointer threading). Recommend per-engine — RustCFML is
  rarely multi-engine in one process today.
- CI time budget. The fuzz + soak harnesses must stay under ~2 minutes
  total; otherwise we'll regret adding them. Initial targets in this
  doc assume that constraint; revise if early measurements show
  10,000 fuzz programs takes longer than ~30s per phase.
