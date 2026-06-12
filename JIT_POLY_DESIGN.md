# JIT — Polymorphic value representation (design)

Status: **DESIGN — awaiting sign-off before any code.**

This document scopes the *next* fundamental gap in the JIT: today every
value flowing through compiled code is a raw `i64` or `f64` scalar. There
is no representation for "a value whose CFML type I don't statically
know." Until we have one, three large pieces of work cannot land:

* **Phase 1** of `JIT_PHASE2_PLAN.md` (fallback shims for `+` / concat)
  — a polymorphic operand at `+` has nowhere to live.
* **Phase 3** of the same plan (member-access ICs) — `obj.prop` returns
  a `CfmlValue` of unknown kind.
* All string / array / struct manipulation inside JIT'd code.

The wall is the same in all three: the JIT can prove a slot is Int *or*
Float, but cannot represent "a value whose kind we'll learn at runtime."

This doc picks the representation that unblocks all three.

Read alongside:
- `JIT_DESIGN.md` — Tier-1 / Tier-1.5 base.
- `JIT_OSR_DESIGN.md` — OSR Phase 1+2.
- `JIT_NEXT_SESSION.md` — short-form current state of `main` (v0.86.0).
- `JIT_PHASE2_PLAN.md` — historical; Phase 2 shipped, Phases 1+3 stale.

---

## Problem statement

`CfmlValue` is a 32-byte Rust enum with ~13 variants (post-PR-A). The
JIT today handles only two of them — `Int(i64)` and `Double(f64)` — as
raw scalars in registers / stack slots. Every analyser bail today is
"I cannot prove this slot is one of those two."

The fix is *not* "support more variants as scalars." Strings cannot live
in a register. Components are pointers to large compound structures.
The fix is "introduce a *boundary* representation that all variants can
cross, and a fast path that stays in raw scalars when monomorphism is
provable."

This is the same conclusion every modern dynamic-language JIT reaches —
V8 (SMI + HeapObject), JSC (NaN-boxed JSValue), MatchBox (NaN-boxed
BxValue), LuaJIT (tag-on-pointer). The representations differ; the
shape — "tagged 64-bit value, immediates for hot kinds, pointers for
the rest" — does not.

---

## Three candidate representations

### Option α — Boxed pointer (`*const CfmlValue`)

Every polymorphic slot holds an 8-byte pointer to a heap-allocated
`CfmlValue`. The JIT loads / stores / passes pointers; runtime shims
allocate `Box<CfmlValue>` (or pull from a pool), and JIT'd code stores
the raw pointer. Lifetime is managed by an explicit allocator the JIT
calls; the GC story reduces to "drop the box when the slot is overwritten
or the call returns."

* **Pro:** trivially correct — `CfmlValue` is already the canonical
  representation; no novel encoding to debug.
* **Pro:** no architecture-specific bit tricks (works identically on
  x86_64, aarch64, Apple Silicon with PAC, future targets).
* **Con:** every polymorphic op allocates. A polymorphic `+` is now
  ~50-100ns (allocator + cache miss + 8-byte indirection), versus
  ~1-2ns for a NaN-boxed `+`. Many real CFML idioms (`x = x + 1`
  where `x` was a string-of-number) become ~10-50× slower than today's
  best-case monomorphic JIT.
* **Con:** the JIT cache holds raw pointers across compiled-body
  boundaries; Rust's borrow checker can't see them, so any sloppy
  lifetime in the surrounding interpreter (e.g. dropping the value pool
  while compiled code still has references) is unsafe-undefined.

### Option β — NaN-box (`u64` with tag bits in unused f64 NaN payload)

Every value is a `u64`. Real f64s are themselves (any non-NaN bit
pattern). Reserved NaN payloads tag Int, Bool, Null, and pointers to
heap-allocated heavy values (`CfmlValue::String` / `Array` / `Struct`
/ etc.). This is V8/JSC/MatchBox's representation.

* **Pro:** numeric path is *the same as today* — Doubles are already
  raw f64; Ints get a tagged-immediate encoding so we don't allocate
  for any small-int arithmetic.
* **Pro:** common ops (`+`, comparison) on monomorphic numeric inputs
  reduce to existing IR — no allocation on the hot path.
* **Con:** the tag scheme must dodge real-world NaN payloads from
  `0.0 / 0.0`, libm, transcendentals, etc. Easy to get wrong; bugs
  manifest as silent miscompiles.
* **Con:** Apple Silicon PAC eats the top 8 bits of pointers when
  enabled (system framework call boundaries). Need a low-tag scheme
  (bottom 3 bits) or careful pointer-stripping at the boundary; both
  are well-known but neither is free.
* **Con:** every `CfmlValue::Struct` / `Array` / `Component` read in
  CFML produces a NaN-boxed pointer; the dispatcher must un-box back
  to a Rust `CfmlValue` at every call-out to the interpreter. Allocation
  amortises across many ops only for *steady-state polymorphic* code;
  for mostly-monomorphic code with a single polymorphic site, NaN-box
  pays the boundary cost on every shim call.

### Option γ — Hybrid tag-pointer (V8 SMI-style)

Every value is a `usize` (pointer-width). Bottom 3 bits are a tag:

* `0b000` → pointer to `CfmlValue::Null/Bool/Closure/Function/...`
  on the heap (8-byte aligned, so low 3 bits zero).
* `0b001` → Int (shifted 3, signed; 61 bits of payload, sufficient
  for all CFML Int ops since CFML wraps at i64 anyway → bail to boxed
  Int on overflow).
* `0b010` → Bool / Null (immediates).
* `0b011` → tagged-Double (we steal one bit by always-truncating an
  f64 to fit; OR we don't tag Doubles at all and instead heap-box
  them when polymorphism is needed — see Recommendation below).
* Higher tag bits available for future variants.

* **Pro:** simpler than NaN-box (no f64-bit-pattern gymnastics; no
  PAC interaction).
* **Pro:** Int is immediate (no allocation in the polymorphic case)
  — and CFML programs are Int-heavy.
* **Con:** Doubles either lose precision (truncation) or always heap-
  allocate when crossing into polymorphic slots. Heap-allocation of
  Doubles is a real cost; transcendental builtins produce them
  constantly.
* **Con:** still novel encoding; bugs in tagging/untagging silently
  corrupt operands.

---

## What about the existing monomorphic fast path?

All three options must coexist with today's `Kind::Int` / `Kind::Float`
slots — those are the JIT's main win and we will not regress them.

The analyser already proves "this slot stays Int (or Float) for the
whole body." Those slots remain raw `i64` / `f64` in registers. **A new
`Kind::Boxed` (or `Kind::Tagged`) joins the lattice for slots the
analyser cannot prove monomorphic**, and the IR emits a kind-check
at the only place kinds meet: at *operations* that consume one boxed
and one raw operand.

Concretely:

* Monomorphic numeric kernels: zero change. Same IR, same speed.
* Mixed-kind kernel (e.g. `Int slot + String slot at a `+` site`):
  the analyser marks one slot Boxed; the other operand is converted
  to Boxed (allocation OR immediate-tag, depending on Option) at the
  use site; the `+` is implemented by a runtime shim that returns
  Boxed. The shim's result flows in Boxed slots until proven
  un-boxed.
* `obj.prop` produces a Boxed value. If the analyser sees it
  immediately stored into a slot already typed Boxed (or unused), fine;
  if it sees it stored into an Int slot, that's a kind mismatch and the
  caller bails as before.

The key insight: introducing a polymorphic representation *widens the
JIT's coverage*. It does not *replace* the monomorphic path.

---

## Lifetime / allocation model

This is where the options diverge most sharply.

### Option α (Boxed pointer)

Every shim that produces a polymorphic value allocates a `Box<CfmlValue>`
and leaks the raw pointer to the JIT'd caller. The caller stores the
pointer in a slot. When a slot is overwritten, the previous box is
dropped. When the compiled body returns, the return-value box is
re-wrapped into a `CfmlValue` and returned to the interpreter; all
other boxes in slots are dropped.

Naive `Box::new` per op is too slow. Need a per-call **value arena**:
a `Vec<CfmlValue>` the JIT's prologue allocates and the epilogue
deallocates. Slot pointers reference into the arena. Re-use within a
call is automatic (overwriting a slot doesn't free, but the arena is
freed at function return).

Issue: arena grows unbounded in a long-running call. Workaround:
arena rotation at OSR sites, or just bound at a few KB and bail above.

### Option β (NaN-box)

Heavy values (`CfmlValue::String/Array/Struct/...`) are heap-allocated
in a separate **GC heap** the JIT can address with a pointer. Numeric
immediates carry no allocation. Refcount semantics: dropping a tagged
pointer slot decrements the refcount of the underlying `Arc`-wrapped
backing store; since `CfmlArray`/`CfmlStruct`/`CfmlQuery` are already
Arc-handles, the JIT just needs to call `Arc::clone` / `Arc::drop` at
the right ABI points (or hand-roll a refcount inc/dec via a libcall
shim).

`CfmlValue::String` is the trickier one — it currently holds an owned
`String`, not an Arc. Two paths:
* Convert `CfmlValue::String(String)` to `CfmlValue::String(Arc<String>)`
  globally. Touches many call sites in `cfml-common`/`cfml-stdlib`. ~1k
  LoC of mechanical changes plus careful review of the few places we
  mutate strings.
* Keep `String` as-is; the JIT-side string representation is
  `Arc<String>` only inside the JIT cache, with a conversion at the
  ABI boundary. Simpler, more boundary cost.

### Option γ (Hybrid tag-pointer)

Same allocation/refcount story as β for pointers; immediates cover Int
and Bool/Null only.

---

## Recommendation

**Option γ (Hybrid tag-pointer) with `String` upgraded to `Arc<String>`.**

Reasoning:

1. CFML is **Int-heavy** in real code. Loop counters, indices, scope
   keys treated as numbers, status codes, query column counts — all Int.
   Keeping Int as a tagged immediate (no allocation) is the single
   biggest win.
2. CFML is **Double-light**. Transcendentals are rare outside scientific
   workloads. Heap-allocating Doubles in polymorphic slots is acceptable
   — and the monomorphic Float fast path doesn't touch this code, so
   `f = sin(x)` kernels stay at v0.74.0 speed.
3. **No PAC interaction**: low-tag (bottom 3 bits) needs only 8-byte
   alignment on heap pointers, which `Box<CfmlValue>` already
   guarantees. Safe on Apple Silicon and every other supported target.
4. **Simpler than NaN-box**: no f64-payload-bit dance. Bugs are
   easier to spot and to fuzz.
5. **`String` → `Arc<String>`** is a one-time tax but pays for itself
   forever: `CfmlValue::String` becomes 8 bytes instead of 24, the
   enum drops to ~24 bytes (no longer String-dominated), and JIT'd
   code can `Arc::clone` instead of allocating a fresh `String` for
   every assignment.

The case against Option β (NaN-box) is *not* that it's wrong — it
demonstrably works in V8 / JSC / MatchBox. It is that the
silent-miscompile risk surface (NaN payload collisions, PAC
interactions, transcendental result quirks) is large, and our
3581-assertion suite is *not* the right shape to catch those bugs.
We would be spending session-budget chasing miscompiles in
domain-specific code; Option γ trades a small per-Double-allocation
tax for a vastly simpler debugging surface.

The case against Option α (pure boxed pointer) is the per-op
allocation cost. Even with a per-call arena, the Int-heavy hot path
loses the inline-tag advantage Option γ keeps. Phase-1-equivalent
work would land *correct* but *slow*.

---

## Migration phases under Option γ

A complete rollout sequenced for incremental shipability — each phase
ships on its own, leaves the JIT in a working state, and is
independently testable.

### v0.87.0 — `CfmlValue::String(Arc<String>)`

Prerequisite. Touches `cfml-common` and every `cfml-stdlib` consumer.
~400-700 LoC mechanical change. No JIT code yet. **Decoupling
ship:** verify no perf regression on the interpreter (Arc::clone is
cheap; might even be a slight win in string-heavy code).

### v0.88.0 — `Kind::Boxed` in the analyser only

Introduce the kind in the lattice; admit it as a slot kind; reject all
ops on it for now (still bails). Lets us *measure* how much surface
becomes admissible — a coverage signal before any codegen work.

### v0.89.0 — Boxed scalar in / out at the ABI

Compiled functions can accept Boxed arguments and return Boxed values.
Arguments arrive as tagged `usize`; returns marshal back to
`CfmlValue` at the call boundary. No mid-body boxing yet. Unlocks
**callers that take a `CfmlValue` parameter but never need to operate
on it polymorphically** (pass-through). Small win on its own; mostly
infrastructure.

### v0.90.0 — Boxed `+` (Phase-1-equivalent)

Per-call arena. Runtime shims `cfml_jit_add_boxed` /
`cfml_jit_concat_boxed`. `Kind::Boxed + Kind::Boxed → Kind::Boxed`.
Codegen for mixed `Kind::Int + Kind::Boxed`: tag the Int, call shim,
result Boxed. Mirrors MatchBox's `jit_fallback_add` but with Option-γ
tags. **Stricter test bar**: fuzz mixed-kind arithmetic; 10× soak; full
suite identical.

### v0.91.0 — Boxed member access (Phase-3-equivalent)

Adds `shape_id` to `CfmlStruct`/`CfmlComponent`. Per-call-site IC slot.
`cfml_jit_load_member_boxed` returns Boxed. Unlocks OO CFML.

### v0.92.0+ — String builtins, array indexing, …

With Boxed in place, the remaining surface (`len`, `uCase`,
`arrayLen`, `[idx]` indexing, etc.) becomes mechanical shim additions
much like Option-A was.

---

## Risks (chosen Option γ, this phasing)

1. **`String` → `Arc<String>` churn.** Every place that pattern-matches
   on `CfmlValue::String(ref s)` for a mutable operation must clone the
   Arc into a fresh `String` (copy-on-write). The set is small (string
   builtins that *mutate* — there are very few; CFML strings are
   immutable in practice) but needs an audit.
2. **Tag-pointer arithmetic in IR.** Every load/store of a Boxed slot
   must mask the tag; every comparison against an immediate must
   un-tag. Easy to forget in one of N codegen sites. Mitigation:
   centralise tag/untag helpers in `jit/box.rs`, never inline the
   ops, and assert tag-equality at every shim entry.
3. **Per-call arena lifetime.** If a JIT'd body bails mid-way and the
   interpreter takes over, the arena must drop in a deterministic
   order. Need explicit `arena.clear()` on the bail path.
4. **Coverage signal misleading.** v0.88.0 might report "30% more
   slots admissible under Kind::Boxed" — but if real-world hot paths
   don't take any of them, the wins are theoretical. Mitigation:
   instrument *real* CFML benchmarks (WireBox, Wheels routing, query
   processing) before committing to v0.90.0+.

---

## Out of scope

* **Full GC.** Refcount via existing `Arc<...>` handles is sufficient
  — no cycle collection needed. CFML doesn't have user-creatable
  cycles in pure-data graphs (closures capture by Arc, but that's
  the only case, and the Arc cycle is short-lived).
* **Generational allocator.** Per-call arena suffices for v0.90.0.
  Revisit only if profiling shows arena allocation in the top-5
  hot paths.
* **Polymorphic ICs (>1 cached shape).** v0.91.0 ships monomorphic
  only. Polymorphic IC is a v0.92.x follow-up if real workloads
  exercise it.
* **Inlining of trivial UDF callees.** Cross-cutting work; defer.

---

## Open questions for sign-off

1. **`String` → `Arc<String>`** is a globally-visible API change for
   anyone consuming `cfml-common` as a library (downstream native
   modules). Is that breaking change acceptable, or do we need a
   compatibility shim?
2. **Tag-bit budget.** Option γ uses 3 bits (8 tags max). Are we
   confident that covers everything we'll want to tag in the next
   2-3 years? Reserving more bits is cheap *now*, expensive later.
3. **Real-world benchmarks.** Before v0.90.0 we should commit to
   running the WireBox-port suite and a Wheels/Mustache template
   render as perf workloads. Are those workloads in shape to run, or
   does some test/bootstrap work need to land first?
4. **Cargo-cult bench question:** should we measure today's
   v0.86.0 vs interpreter on those real workloads *before* starting
   Option γ, to set a baseline? My instinct is yes — otherwise
   "v0.90.0 is 1.4× faster" is information-free without a reference
   point.
