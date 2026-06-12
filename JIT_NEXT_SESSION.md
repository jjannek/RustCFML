# JIT — Next-Session Handover

Read this first, then `JIT_DESIGN.md` (Tier-1/1.5 base layer) and
`JIT_OSR_DESIGN.md` (OSR Phase-1 design — both phases now shipped).
Auto-memory `project_jit.md` mirrors this and is the canonical short-form
source of truth across sessions; this file is the long-form handover for a
fresh session that wants the full picture before starting work.

## TL;DR of current state (2026-06-10, through v0.102.0 — pending push + tag)

**v0.102.0 — SMI-safety sweep over pre-v0.101.0 Boxed shims.** Pure
refactor: every shim in the v0.92.0/v0.99.0–v0.99.3 block (`len`,
`uCase`/`lCase`/`trim`/`ltrim`/`rtrim`, `reverse`, `asc`, `stripCr`,
`htmlEditFormat`/`htmlCodeFormat`/`encodeForHtml`, `urlEncodedFormat`/
`urlDecode`/`jsStringFormat`, `left`/`right`/`mid`/`repeatString`,
`find`/`findNoCase`, `replace`/`replaceNoCase`, `arrayLen`,
`structKeyList` — ~24 entries, 28 call sites) now materialises its
Boxed input via `boxed::materialize_tagged` instead of
`boxed::borrow_tagged`. The v0.99.6+ member-IC can return SMI Int inline
(low-bit-tagged i61); `borrow_tagged` panics on non-TAG_PTR, so passing
an Int-flowed-from-member-access value through any of these shims would
have aborted. Untriggered in repo until now (no test passed an Int
member-access through these shims), trivial to trip in real CFML —
`len(p.age)` after a SetProperty/GetProperty IC populates with an Int
shape was the smoking gun. v0.101.0 shims already used
`materialize_tagged`; this brings the older surface in line and closes
the latent hazard (gotcha #32). Section comment updated. Two match
expressions (`cfml_len_boxed_i64`, `cfml_array_len_boxed_i64`) switched
from `match v` to `match &v` to keep `v.as_string()` reachable in
arms that don't bind it; one `if let` (`cfml_struct_key_list_boxed`)
switched to `if let CfmlValue::Struct(s) = &v` to leave `v` borrowed
for the else branch. **Tests**: 88 unit (+1
`pre_v0101_shims_accept_smi_int_inputs` — passes `try_tag_smi_int(42)`
through `cfml_len_boxed_i64`, `cfml_ucase_boxed`, `cfml_asc_boxed_i64`,
`cfml_find_boxed_boxed_i64`, and `cfml_array_len_boxed_i64`; pre-fix
all five would abort) + 70 e2e + 5 recursion + 4 soak + 2 fuzz. Full
suite `tests/runner.cfm` 3612/3612 identical `RUSTCFML_JIT=0` vs
`RUSTCFML_JIT_THRESHOLD=1`; wasm32 build green. **Perf**: refcount-bump
clone per Boxed shim arg vs prior bare ref deref — negligible; in-repo
bench/baseline kernels stable within noise of v0.101.0. No new shims;
the existing `cfml-vm/src/jit/builtins.rs` test
`boxed_shims_match_interpreter` still uses `borrow_tagged` to inspect
shim outputs (correct — those are guaranteed TAG_PTR returns from
`arena::box_into_active`). **Version bump**: 0.101.0 → 0.102.0 (workspace
package + all 7 path-dep specs). (2026-06-10)

## Pre-v0.102.0 state — v0.101.0 (pushed + tagged `76218bf`)

**v0.101.0 — 13 new Boxed-arg predicate/collection shims (commit `76218bf`).**
Mechanical extension of the v0.99.0–v0.99.3 surface; Boxed-arg shim
surface grows 25 → 38 entries.

- **Type predicates** (Boxed → Boxed Bool): `isNumeric`, `isArray`,
  `isStruct`, `isBoolean`, `isSimpleValue`, `isNull`.
- **Collection predicates** (Boxed → Boxed Bool): `arrayIsEmpty`,
  `structIsEmpty`.
- **Collection sizes** (Boxed → Int): `structCount`, `listLen` (default
  delimiter `,`).
- **Collection serialise** (Boxed → Boxed): `arrayToList` (default
  delimiter `,`).
- **2-arg predicates** (Boxed, Boxed → Boxed Bool): `structKeyExists`,
  `arrayContains`, `arrayContainsNoCase`.

Bool-returning shims wrap `CfmlValue::Bool` into the active arena
(`Kind::Bool` cannot escape the stack to a local/return), so JIT'd output
stringifies as "YES"/"NO" / "true"/"false" identically to the
interpreter. Int-returning shims return raw `i64`.

**Important discovery / gotcha #32**: All new shims use
`boxed::materialize_tagged`, **NOT** `boxed::borrow_tagged`. Since v0.99.6
the member-IC can return SMI Int inline (low-bit-tagged i61), and
`borrow_tagged` panics on non-TAG_PTR. `materialize_tagged` synthesises
`CfmlValue::Int` for SMI and Arc-clones the heap pointee (cheap for
String/Array/Struct). **Existing v0.99.0–v0.99.3 shims still use
`borrow_tagged`** — latent bug, untriggered because no test passes
Int-flowed-from-member-access through them. **Listed as next-up sweep**.

**Codegen quirk** (gotcha #33): `isNull(bareIdent)` is special-cased in
`cfml-codegen/src/compiler.rs:2759` to emit `TryLoadLocal + IsNull` ops
that bypass the builtin Call path. The new `isnull` shim therefore only
fires for non-identifier args like `isNull(obj.field)`, `isNull(fn())`,
etc. Both shapes are covered by e2e — see
`isnull_via_member_access_in_jitted_udf_matches_interpreter`.

**Tests**: 82 unit (+1 — `predicate_shims_match_interpreter`; the
existing `boxed_overloads_match_only_boxed_args` test extended for the
v0.101.0 surface) + 70 e2e (+7 — `isnumeric_predicate_in_jitted_udf` /
`type_predicate_family_in_jitted_udf` /
`collection_count_shims_in_jitted_udf` /
`array_to_list_in_jitted_udf` / `struct_key_exists_in_jitted_udf` /
`array_contains_in_jitted_udf` /
`isnull_via_member_access_in_jitted_udf`) + 5 recursion + 4 soak + 2
fuzz. Full suite `tests/runner.cfm` 3612/3612 identical
`RUSTCFML_JIT=0` vs `RUSTCFML_JIT_THRESHOLD=1`. wasm32 build green.

## Pre-v0.101.0 state — v0.100.0 (pushed + tagged `484c6e8`)

**v0.100.0 — SetProperty/StoreLocalProperty IC (write-side mirror of
v0.99.5's read IC).** Per-call-site monomorphic IC over `obj.prop =
value` (`SetProperty`) and `local.prop = value` (`StoreLocalProperty`)
on plain-Struct receivers. Same `[shape, idx, kind]` slot layout. Hot
path: shape-match → new `CfmlStruct::set_at_index(idx, val)` does a
value-only update at known index **without bumping shape_id**, so
reader-side ICs for the same shape stay warm across writes. Cold path:
`get_ci_indexed` → either overwrite at the found idx (no bump) or
`insert` a new key (shape bumps; IC re-populated). **Bails** on non-Struct
+ Components/NativeObject parents (any Struct carrying `__variables` /
`__properties` / `__super` — interp's setter machinery has writeback
semantics the JIT can't replicate). Analyser admits values as
Int/Float/Boxed (non-Boxed boxed via the same `ensure_boxed` helper
Concat uses). **OSR also extended in the same commit** (mirror of
v0.99.8 OSR-for-read), so write-heavy serve-mode `__main__` loops are
covered.

**Perf**: `write_ic_bench.cfm` (5M iter × 2 writes/iter × 100 outer
calls): **2.45s → 0.57s (~4.3×)**, `osr_compiled=1`. Existing baselines
unchanged within noise (numeric 180×, udf_call_graph 85×, string 1.31×,
struct_member 9.7×).

## Pre-v0.100.0 state — v0.99.8 (pushed + tagged `ebd19f4`)

**v0.99.8 — OSR admission for member reads + Boxed arith.** Mechanical
mirror of v0.99.5/6/7 inside `osr.rs::compile_loop` + `analyze_loop` +
`simulate_block`. The IC shim and slow-shim plumbing already exist on
`Backend`; OSR now imports them and emits the same code shape. Three
changes: (1) `analyze_loop`'s op-subset arm admits `LoadLocalProperty`
(interns the local slot as a Boxed receiver) and `GetProperty`, both
counted as useful work; (2) `simulate_block` admits the member ops
(LoadLocalProperty requires Boxed slot, both push Boxed) and relaxes
`Add|Sub|Mul` to use `analysis::num_bin_kind` so Boxed operands type
through; (3) `compile_loop` pre-scans member sites + allocates IC slots,
imports `member_get`/`add|sub|mul_boxed` shim refs, and routes Add/Sub/Mul
through `arith_boxed_smi` when either operand is Boxed. Also added: an
OSR-local **slot-kind Infer fixpoint** so `total = total + obj.prop`
(initially Int slot, body stores Boxed result) widens `total` to Boxed
across iterations (mirrors `analysis.rs` Mode::Infer behaviour). And
`build_caller_kinds` now seeds slots referenced by `LoadLocalProperty`
too — otherwise the receiver local has no caller_kinds entry and the
intern step rejects. **Perf**: `bench/baseline/struct_member_kernel.cfm`
(`__main__` outer loop, 5M iters of `total + obj.x + obj.y * 2 + obj.z
- obj.w`) **1.86s → 0.19s (~9.8×)** — was 1.02× pre-v0.99.8. The other
three baselines unchanged (numeric_kernel 180×, udf_call_graph 82× —
heuristic still rejects thin wrapper, string_kernel 1.27× within noise).
**Tests**: 78 unit + 58 e2e (+4 OSR member: `osr_member_read_in_main_loop`
/ `osr_member_read_with_sub_and_mul` / `osr_member_read_case_insensitive`
/ `osr_member_read_bails_on_non_struct_receiver`) + 5 recursion + 4 soak
+ 2 fuzz. Suite 3571/3571 identical interp vs JIT, wasm32 green. One
unit test renamed (`analyze_loop_rejects_string_concat` →
`analyze_loop_accepts_string_concat_with_boxed_slot`) — old behaviour
was the pre-v0.91.0 surface, v0.99.8 Infer upgrade now legitimately
accepts the same loop.

## Pre-v0.99.8 state — v0.99.7 (pushed + tagged `c3222f4`)

**v0.99.7 — Sub/Mul admit Boxed operands (SMI fast path + slow shims).**
Mirror of v0.99.6 Add: new `cfml_jit_sub_boxed` / `cfml_jit_mul_boxed`
extern "C" shims (SMI fast path on both inputs, else `to_number` coercion
to `Double` matching the interpreter's `numeric_op`). Translator's
`add_boxed_smi` generalised into `arith_boxed_smi(NumOp, slow_shim)` so
Add/Sub/Mul share the tag-check fast path and slow-shim plumbing — the
`BytecodeOp::Add | Sub | Mul` arm picks the slow shim by op. Analyser's
Sub/Mul arms now use `num_bin_kind` (was guarded by `is_num()` rejecting
Boxed). No new plumbing; pure mechanical extension. **Perf**: `function
calc(p) { return p.a + p.b * 2 + p.c - p.d; }` × 10M iters: 22.32s →
13.46s (**1.66×**) — matches v0.99.6's Add ratio on the full
struct_member_kernel shape. The `bench/baseline/struct_member_kernel.cfm`
(`__main__` loop, currently 1.02×) still won't move until OSR also admits
member-read ops + Boxed arith — see v0.99.8 candidate below. **Tests**:
78 unit (+4 sub/mul shim tests) + 54 e2e (+4 sub/mul Boxed UDF). Full
suite 3571/3571 identical interp vs JIT, wasm32 green. No regression on
numeric (189×) or udf_call_graph (79×).

## Pre-v0.99.7 state — v0.99.6 (pushed + tagged `659f7c2`)

**v0.99.6 — SMI Int tagging (TAG_INT=0b001) + Add admits Boxed.** Low-bit
SMI Int tagging works on all targets (PAC/MTE modify HIGH bits only;
8-aligned `Box` pointer LOW bits are untouched). i61 range; out-of-range
falls back to heap-box. Three integrated changes: (1) `cfml_jit_box_int`
returns SMI when value fits — zero-allocation path for the common case;
(2) member-IC slot grows to `[shape, idx, kind]` and on Int hits encodes
the result as SMI inline (no arena tracking); (3) `Add` analyser admits
Boxed operands and the translator emits an inline tag-check fast path
(`(a^TAG_INT) | (b^TAG_INT)) & TAG_MASK == 0` → untag, iadd, retag with
SMI-fit overflow check; slow paths fall through to `cfml_jit_add_boxed`).
Concat/add shims use new `materialize_tagged` instead of `borrow_tagged`
to handle SMI inputs. Sub/Mul stay Int/Float-only (would need matching
sub/mul slow shims; numeric struct members are typically summed not
multiplied, so the practical loss is small). Float SMI / NaN-pun deferred
to v0.99.7+ Phase B (gates on `cfg(target_arch="x86_64")` and an opt-in
`unsafe-nanbox-aarch64` feature). **Perf**: `function sum4(p) { return p.a + p.b + p.c + p.d; }`
× 10M iters: 22.16s → 13.21s (**1.68×**). Headroom is now interpreter
loop body (`__main__` doesn't JIT in CLI mode) + member-IC shim call
overhead — see v0.99.7 candidates. **Tests**: 73 unit + 50 e2e (+4) +
5 recursion + 4 soak + 2 fuzz; suite 3571/3571 identical interp vs JIT;
wasm32 green.

## Pre-v0.99.6 state (through v0.99.5 — pushed + tagged)

A working, opt-in (now **default-on**) **Cranelift JIT** is on `main`:

- **Tier-1** integer numeric kernels — *whole-function* JIT
- **Tier-1.5** floating-point kernels — `Double` literals, `/`, float-typed locals, `Double` args, float `%` via libcall, `^` via libcall
- **Option A** native builtin calls — `abs`/`min`/`max` + 15 pure-math (floor/ceiling/round/sgn/fix + sqr/exp/log/log10/sin/cos/tan/asin/acos/atan) + 6 bit-twiddling (bitAnd/Or/Xor/Not/Shln/Shrn) + `pow()` function call form
- **v0.99.0 Boxed-arg string shims** — `len` (Boxed → Int), `uCase`/`lCase`/`trim`/`ltrim`/`rtrim` (Boxed → Boxed). First installment of the v0.92.0-roadmap "string/array shim surface". `KindReq::Boxed` variant in the shim table; analyser's Call::Builtin arm relaxed to admit Boxed args; codegen path unchanged (existing `to_i64(v, Kind::Boxed) = v` pass-through). Perf: 5.3× on a hot uCase+lCase+trim+len kernel.
- **v0.99.1 Six more Boxed-arg shims** — `reverse` / `stripCr` / `htmlEditFormat` / `htmlCodeFormat` / `encodeForHtml` (Boxed → Boxed), and `asc` (Boxed → Int). Mechanical extension of v0.99.0; no new plumbing. Brings the Boxed-arg surface to 12 entries. Perf: 1.36× on a shim-heavy kernel (allocation-dominated; bigger wins come from kernels that avoid the alloc tax).
- **v0.99.2 — 11 more mechanical Boxed shims** — single-arg infallible (`urlEncodedFormat`, `urlDecode`, `jsStringFormat`) + multi-arg (`left`/`right`/`mid`/`repeatString` Boxed+Int, `find`/`findNoCase` Boxed+Boxed→Int, `replace`/`replaceNoCase` Boxed+Boxed+Boxed→Boxed). No new plumbing — translate.rs Builtin codegen is fully generic for any arity × any Kind combo. Brings Boxed surface 12 → 23 entries.
- **v0.99.3 — bail plumbing for builtin shims** — new `Shim.bailable: bool`. Bailable shims get a trailing `*mut i64` bail param in their signature; the translator + osr both append the bail pointer to the call arglist and emit a post-call `brif bail, bail_block, cont` (mirrors UDF Call's bail pattern). On bail the engine re-runs the function under the interpreter so the runtime error path (throw / cfcatch) executes identically. Two new shims using it: `arrayLen(Boxed) → Int` (BAILABLE — Lucee@7 parity, QueryColumn errors so the JIT bails to let the interpreter throw `Can't cast`) and `structKeyList(Boxed) → Boxed` (infallible).
- **v0.99.4 — `shape_id` on CfmlStruct (no JIT change)** — pure cfml-common plumbing for the v0.99.5 IC. Wraps the inner `Arc<RwLock<IndexMap>>` with a `StructInner { map, shape_id: u64 }`. Shape ID is bumped on **structural** change only (new key inserted, key removed, clear when non-empty, `get_or_insert_struct` when absent/non-struct, and unconditionally on every `with_write` since the closure has arbitrary access). Value-only updates (same key, new value) leave shape alone so an IC over `obj.prop` stays warm for value swaps. Shape IDs from a process-wide AtomicU64 (`0` reserved as never-populated sentinel).
- **v0.99.5 — monomorphic member-access IC** for `obj.prop` (`GetProperty`) and `local.prop` (`LoadLocalProperty`). New `cfml_jit_member_get_boxed(obj_tagged, name_ptr, name_len, ic_slot, bail) -> i64` shim in `jit/shims.rs`. Fast path: cmp `s.shape_id()` with `cached_shape`; on match, `s.get_at_index(cached_idx)` (O(1) IndexMap position lookup); arena-box; return. Cold path: `s.get_ci_indexed(name)` (case-insensitive scan); update IC. Bails when receiver is not a `CfmlValue::Struct` — Components / Queries / Closures / native objects keep the more elaborate interpreter dispatch. Per-call-site IC slot storage in `Backend.member_ic_slots: Vec<Box<UnsafeCell<[u64;2]>>>` (Boxes give stable addresses through Vec growth). Property names interned via `Backend.member_names` (same gotcha #15 discipline). **OSR intentionally NOT extended for these ops in v0.99.5** — OSR's catch-all `_ => return None` rejects loops containing them, so OSR transparently skips. v0.99.6 follow-up. Perf: 1.35× on a string-concat-heavy struct kernel (`p.firstName & "|" & p.lastName & "|" & p.age` × 3M). First win on struct-member-shaped workloads (1.00× pre-v0.99.5). Numeric member arithmetic (`p.a + p.b`) still bails because the arith analyser doesn't admit Boxed operands — see "v0.99.6 candidate" below.
- **Option D** OSR (on-stack replacement) — Phase 1 (ForLoopStep) + Phase 2 (Jump / JumpIfTrue / JumpIfFalse back-edges), covering counted for-loops, while/until loops, and do-while/do-until loops in any function (including `__main__`)
- **Phase 1 UDF→UDF direct calls** (v0.84.0) — JIT'd caller invokes another JIT'd UDF via a libcall dispatcher (thread-local engine ptr). Self-recursion supported (fib, factorial); leaf-first warm-up brings entire call chains native.
- **Phase 2 UDF→UDF indirect dispatch** (v0.86.0) — caller may now bind a not-yet-compiled foreign callee speculatively (`ret_kind = BindingRet::Int` default since v0.90.1; previously `ret_float=false`). The dispatcher checks the actual cached callee's `ret_kind` against the caller's expectation at runtime; on mismatch it surfaces `*bail=2`, which the outer `try_call` uses to evict the caller from the cache so it re-specializes against the now-known callee on its next hot trip. Unlocks **mutual recursion** (A↔B), **3-cycles** (A→B→C→A), and **forward calls** (caller compiles before callee). Leaf-first warm-up no longer required.
- **Option γ Boxed mid-body** (v0.90.0) — `Kind::Boxed` flows through the body, not just across the ABI. New per-call value arena (`jit/arena.rs`) plus runtime shims (`jit/shims.rs`) for `box_int`/`box_float`/`concat_boxed`/`add_boxed`/`str_literal`. Analyser admits `BytecodeOp::String` and `BytecodeOp::Concat`; codegen lowers Concat by boxing non-Boxed operands via the box shims then calling `cfml_jit_concat_boxed`. Unlocks any hot loop whose only "polymorphic" ops are string literals and `&` concat (e.g. `function buildLine(prefix, n) { var s = prefix; for (var i = 1; i <= n; i++) { s = s & "-" & i; } return s; }`).
- **UDF→UDF Boxed dispatch** (v0.90.1) — `UdfRefBinding.ret_float: bool` grew to `ret_kind: BindingRet` (Int/Float/Boxed). Resolver admits Boxed-arg and Boxed-ret callees; dispatcher's `expected_ret_float` boolean is now an `expected_ret_kind: i64` (0/1/2) with bail=2 on mismatch. Translator emits the code and pushes Boxed call results as `Kind::Boxed` so they feed straight into the v0.90.0 IR pipeline. Unlocks JIT-caller → JIT-callee chains where the callee takes or returns a `CfmlValue::String` (or any non-numeric Boxed value).
- **Option A — `__main__` admission** (`d69cc5f`, no version bump — folded into the v0.91.x stack) — drops the `if func.name == "__main__" { return None; }` early-bail in the whole-fn analyser. `__main__` is admissible on its merits (zero args, no defaults, existing op-by-op pass rejects every side-effecting top-level construct). CLI runs never cross the hotness threshold for `__main__` (it's called once); the gate only fires in serve mode where `__main__` is the per-request body.
- **Option B — OSR Boxed slots + UDF dispatch** (v0.91.0) — OSR (on-stack replacement) now matches whole-fn JIT's v0.90.0+v0.90.1 capability set. A hot loop region can compile to native when its body uses `Kind::Boxed` locals, `String` literals, `&` concat, and direct calls to other JIT-compiled UDFs (Int/Float/Boxed args + Int/Float/Boxed returns). Previously OSR rejected any region with a non-Int/Float local or a non-builtin Call. `build_caller_kinds` now emits `Kind::Boxed` for any non-Int/Float live local; `run_osr_compiled` installs a per-call `ArenaGuard`, box-clones entry-side Boxed slots, borrows + clones-out exit values, then drains. `OsrCompiled` carries `referenced_udfs` revalidated on every call (parallel to whole-fn `try_call`).
- **OSR-UDF admission heuristic** (v0.91.1) — v0.91.0's UDF-in-OSR engaged on hot outer loops whose body was a thin UDF-call wrapper (`load + call + store + step`), where the `cfml_call_jit_udf` libcall's ~100ns/call overhead pessimised vs the interpreter's already-cached `Call → try_call` path. Surfaced as 82× → 75× on `udf_call_graph.cfm` (-6.2pp). `analyze_loop` now rejects OSR when the region contains any UDF call AND fewer than 2 "useful work" ops (arithmetic / comparison / logic / String / Concat / native builtin Call). Restores baseline perf; preserves the capability for mixed-work loops.
- **CLI ergonomics** — `--no-jit`, `--jit-threshold N`, `--jit-stats` flags; `RUSTCFML_JIT=0` / `RUSTCFML_JIT_THRESHOLD=N` / `RUSTCFML_JIT_DEBUG=1` env vars

**Coverage**: any hot kernel composed of integer + float arithmetic, comparisons, the fused loop ops, the allowlisted builtins, the bit-ops, *calls to other JIT'd UDFs (Int/Float/Boxed specializations)*, **string literals**, **`&` concat**, the v0.99.0/v0.99.1/v0.99.2 **Boxed-arg shims** (23 entries: `len`/`asc`/uCase/lCase/trim/ltrim/rtrim/reverse/stripCr/htmlEditFormat/htmlCodeFormat/encodeForHtml/urlEncodedFormat/urlDecode/jsStringFormat/left/right/mid/repeatString/find/findNoCase/replace/replaceNoCase), the v0.99.3 **bailable shims** (`arrayLen`/`structKeyList`), and v0.99.5 **member-access reads** (`obj.prop` via GetProperty + `local.prop` via LoadLocalProperty on Struct receivers) compiles to native after the threshold crosses. As of v0.91.0 OSR-compiled hot loops cover the same arith/concat/UDF surface in any enclosing function (including `__main__`), with the heuristic limiting OSR-UDF admission to loops that do real work between UDF calls — but OSR does NOT yet admit the v0.99.5 member ops. Arrays, struct literals (`BuildStruct`), property WRITE, method calls (`CallMethod`), `LoadGlobal` of plain variable names, arithmetic on Boxed values, and string/array shims beyond the v0.99.2 set still bail to the interpreter.

**Defaults**: `jit` is in `cfml-vm`'s and `cli`'s default feature sets. A plain `cargo build` ships a JIT-on binary. Opt out with `RUSTCFML_JIT=0`, `--no-jit`, or `--no-default-features`. Wasm targets (`cfml-worker`, `rustcfml-wasm`) use `default-features = false`, and the cranelift deps themselves are `[target.'cfg(not(target_arch = "wasm32"))'.dependencies]`, so wasm builds get neither the deps nor the code.

**Perf — pure-numeric kernels** (release, 100M-iter hot loops in `__main__`; pure interpreter vs default-on JIT):
- counted for-loop: 35.84 s → 0.12 s (~300×)
- while-loop: 36.93 s → 0.12 s (~308×)
- do-while: 38.15 s → 0.12 s (~318×)
- math-heavy kernel (sqr+log+sin+cos): 13.09 s → 0.10 s (~131×)
- full test suite: 150.66 s → 31.43 s (~4.8× wall-clock; suite is mostly cold paths but a few thread/sleep tests are CPU-bound enough to benefit)

**Perf — UDF→UDF (v0.84.0; pre-Phase-1 baseline = interpreter, because the analyser previously rejected any caller with a UDF call)**:
- helper-in-hot-loop, 10M helper() calls: 117.2 s → 1.1 s (~107×)
- fib(22) × 20 self-recursion: 13.1 s → 5 ms (~2600× — native recursion eliminates interpreter dispatch overhead)
- 3-level call chain (top→mid→leaf), 3.5M total calls: 35.4 s → 832 ms (~43× — most representative of real CFML code)

**Tests** (through v0.99.5): **67** in-crate unit (v0.99.2 extended `boxed_overloads_match_only_boxed_args` for the multi-arg shapes — (Boxed,Int) / (Boxed,Int,Int) / (Boxed,Boxed)→Int / (Boxed,Boxed,Boxed)) + **46** e2e (`tests/jit_numeric.rs`; v0.99.2 added `url_and_js_string_format` / `left_right_mid` / `find_and_replace` / `repeat_string`; v0.99.3 added `array_len_shim` / `array_len_bails_on_query_column` / `struct_key_list_shim`; v0.99.5 added `member_get_loadlocalproperty` / `member_get_case_insensitive` / `member_get_missing_key_returns_null` / `member_get_bails_on_non_struct_receiver`) + 5 recursion (`tests/jit_recursion.rs`) + 4 soak (`tests/jit_soak.rs`) + 2 fuzz (`tests/jit_fuzz.rs`; 200 + 1000 random programs bit-identical interp vs JIT). Full suite `tests/runner.cfm` 3571/3571 identical with `RUSTCFML_JIT=0` and `RUSTCFML_JIT_THRESHOLD=1`. wasm32 build green; 0 new warnings.

**Perf — v0.99.5 member-access IC** (release):
- struct concat kernel (`p.firstName & "|" & p.lastName & "|" & p.age` × 3M): 7.86 s → 5.83 s (~1.35×). `fn_compiled=1` confirms whole-fn JIT engages. First win on struct_member_kernel-shaped workloads.
- numeric struct kernel (`p.a + p.b + p.c + p.d` × 10M): still 1.00× — the IC reads the value into Boxed, then `Add` rejects Boxed operands (arith analyser still Int/Float-only). **The unbox-on-arith follow-up (v0.99.6 candidate below) is what unlocks numeric member kernels.**

## Version timeline (this codebase)

| Version | Commit | Change |
|---|---|---|
| v0.73.0 | `558c675` | Tier-1 + Tier-1.5 opt-in JIT |
| v0.74.0 | `2c5333c` | Double args, float % via fmod, ^ via pow libcalls |
| v0.75.0 | `cc6279a` | Option A — native builtin calls (abs/min/max) + Lucee-parity reject of builtin redefinition |
| v0.76.0 | `cac00fb` | OSR Phase 1 — ForLoopStep back-edges |
| (no bump) | `ea2e255` | JIT default-on for native builds |
| v0.77.0 | `f2f7427` | OSR Phase 2 — Jump back-edges (while/until) |
| v0.78.0 | `165fdb4` | OSR Phase 2 cont'd — JumpIfTrue/False (do-while/do-until) + shared shadow guard |
| v0.79.0 | `8a4b6a2` | 15 math shims |
| v0.80.0 | `f43ba97` | 6 bit-twiddling shims |
| v0.81.0 | `aac24af` | CLI `--no-jit` / `--jit-threshold` |
| v0.82.0 | `8c4982d` | `--jit-stats` + CLI default features fix |
| v0.83.0 | `c830cca` | `pow()` function-call shim |
| v0.84.0 | `aaa9587` | UDF→UDF direct calls via libcall dispatch + fbc-reset-on-error |
| v0.85.0 | `cef4ed5` | 5 new shims: incrementValue/decrementValue + bitMaskRead/Set/Clear |
| v0.86.0 | `cbe2594` | Phase 2: jit_resolve_fn-style indirect dispatch — mutual recursion + forward UDF calls; speculation-mismatch eviction; fuzz + soak harness |
| v0.87.0 | `f1e8911` | `CfmlValue::String(String) -> String(Arc<String>)` — prerequisite for Option-γ tag-pointer polymorphic values (per `JIT_POLY_DESIGN.md`). Refcount-bump clones. Breaking change for downstream `cfml-common` consumers. |
| v0.88.0 | `6f10fe2` | `Kind::Boxed` forward-declared in the analyser lattice + new `jit::coverage` module. `--jit-coverage` flag (and `RUSTCFML_JIT_COVERAGE=1` env var) dumps per-file Option-γ admissibility report. Measurement-only; no codegen. |
| v0.89.0 | `df012f3` | **Boxed scalar in/out at the ABI.** New `jit/boxed.rs` (tag-pointer encoding; `TAG_PTR = 0b000` for heap `Box<CfmlValue>`; `box_value`/`reclaim_tagged`). Signature scheme widened from 1-bit float bitmap to 2-bit kind tuple (Int=00 / Float=01 / Boxed=10), `MAX_JIT_PARAMS` 32→16. `CacheEntry::Compiled` gains tri-state `RetKind` (Int/Float/Boxed). `signature_for` now admits any `CfmlValue` (non-Int/Float crosses as Boxed). Analyser admits `Kind::Boxed` for params, LoadLocal/StoreLocal, and Return — every other consumer (arith / cmp / branch / Call args / Dup) still rejects. UDF→UDF resolver refuses Boxed-arg + Boxed-ret callees (caller has no IR-level box ops to consume them). `run_compiled` marshals Boxed args via `box_value`, reclaims on return; pass-through inputs return their tagged pointer (ownership transfer detected by pointer equality). Translate.rs needs **no codegen changes** — Boxed slots route through the existing I64 path. Unlocks pure pass-through functions (`function f(x) { return x; }`) — small standalone win, but lays the infrastructure for v0.90.0's `+`/concat shims. **Tests**: 56 unit + 21 e2e (3 new Boxed pass-through), full suite 3540/3540 identical RUSTCFML_JIT=0 vs RUSTCFML_JIT_THRESHOLD=1, wasm32 green. |
| v0.90.0 | `074c142` | **Mid-body Boxed operations — String literals + Concat at the IR level.** New `jit/arena.rs` (per-call thread-local arena tracking shim allocations; reclaimed on body return or bail), `jit/shims.rs` (`cfml_jit_box_int`, `cfml_jit_box_float`, `cfml_jit_concat_boxed`, `cfml_jit_add_boxed` runtime shims; bit-exact with the interpreter's `Concat` / `Add` semantics). `translate::Backend` gains per-literal `string_literals: Vec<Box<str>>` interning + a `cfml_jit_str_literal(ptr, len) -> i64` shim. Analyser admits `BytecodeOp::String(_)` → `Kind::Boxed` and `BytecodeOp::Concat` with any combination of `Int`/`Float`/`Boxed` operands → `Kind::Boxed`. Codegen lowers Concat by boxing non-Boxed operands via the box shims, then calling `cfml_jit_concat_boxed`. `run_compiled` installs an `ArenaGuard` around the body call and drains the arena on the way out — except on a successful `Boxed` return, where the engine reclaims the matching tag itself. Unlocks `function buildLine(prefix, n) { var s = prefix; for (var i = 1; i <= n; i++) { s = s & "-" & i; } return s; }` — the string_kernel inner loop. **Tests**: 59 unit (3 new in `arena`, 6 new in `shims`, 1 renamed in `analysis`) + 24 e2e (3 new Boxed-concat: `string_literal_pass_through_jits`, `boxed_concat_in_jitted_udf_matches_interpreter`, `boxed_concat_with_float_operand_matches_interpreter`), full suite 3540/3540 identical RUSTCFML_JIT=0 vs RUSTCFML_JIT_THRESHOLD=1, wasm32 green. |
| v0.90.1 | `c89d00b` | **UDF→UDF Boxed dispatch — tri-state `expected_ret_kind`.** `UdfRefBinding.ret_float: bool` → `ret_kind: BindingRet` (Int/Float/Boxed; new enum in `jit/analysis.rs`, parallel to `RetKind` in `jit/mod.rs`, with `to_binding`/`as_code` helpers). Resolver drops `sig_has_boxed` rejection and the `RetKind::Boxed => None` arm — any Compiled callee binds; not-yet-compiled callees still speculate `BindingRet::Int` and let the dispatcher surface bail=2 on mismatch. `udf_binding_still_valid` collapses to a single `ret_kind.to_binding() == binding.ret_kind` check. `cfml_call_jit_udf` ABI changes from `expected_ret_float: i64` (boolean) to `expected_ret_kind: i64` (0/1/2); `dispatch_jit_udf` compares to cached `ret_kind.as_code()` and bails=2 on mismatch (no special-case for Boxed). Self-recursion mistype check generalises from "ret is Float" to "ret is not Int". Translator emits the 0/1/2 code from the binding and pushes the result as `Kind::Boxed` when the binding says so — Boxed values flow into the v0.90.0 IR pipeline. Analyser's Call site now splits arg-kind validation per-marker: builtins still Int/Float-only, UDF callsites also admit Boxed args. Dead-code cleanup: `sig_has_boxed`, `arg_kind_code`, `sig_nargs`, `RetKind::is_float`, `RetKind::from_binding` removed (-~30 LoC). **Tests**: 65 unit + 26 e2e (2 new — `jit_caller_invokes_boxed_returning_udf_and_matches_interpreter`, `jit_caller_threads_boxed_arg_through_to_jitted_callee`), full suite 3540/3540 identical RUSTCFML_JIT=0 vs RUSTCFML_JIT_THRESHOLD=1, wasm32 green. |
| (no bump) | `d69cc5f` | **Option A — admit `__main__` for whole-fn JIT.** Drops the early-bail in `analysis.rs::analyze`; `__main__` is now admissible on its merits. CLI-only impact: `__main__` is called exactly once per process so the hotness counter never trips — the practical unlock is **serve mode** (where `__main__` runs per-request and crosses threshold). The op-by-op admission already rejects every side-effecting top-level construct (writeOutput, includes, cfquery, struct/array, …) so the change cannot introduce a semantic divergence. |
| v0.91.0 | `02ab030` | **Option B — OSR Boxed slots + UDF dispatch.** OSR now matches whole-fn JIT's v0.90.0+v0.90.1 capability set. `build_caller_kinds` emits `Kind::Boxed` for any non-Int/Float live local. `run_osr_compiled` installs a per-call `ArenaGuard`, box-clones entry-side Boxed slots, borrows + clones-out exit values, then drains. `OsrCompiled` carries `referenced_udfs` revalidated on every call (parallel to whole-fn `try_call`). `analyze_loop` takes a `UdfResolver`; admits `String`/`Concat` and `LoadGlobal` of UDF names. `compile_loop` allocates a stack slot for UDF call args and emits the `cfml_call_jit_udf` libcall with bail=2 speculation-mismatch handling — all mirroring `translate.rs`. 4 OSR call sites in `lib.rs` pass a new `jit_udf_lookup(user_functions, name)` closure. **Surfaced a 6.2pp regression on `udf_call_graph` (82× → 75×)** because the libcall dispatcher costs ~100ns/call vs the interpreter's already-cached `Call → try_call`, and the kernel's outer body has no real work to amortise against — closed in v0.91.1. **Tests**: 65 unit + 28 e2e (2 new — `osr_boxed_concat_loop_in_main_matches_interpreter`, `osr_calls_jitted_udf_from_outer_loop_in_main`). |
| v0.99.0 | `1243dc7` | **Boxed-arg string shims — `len`/`uCase`/`lCase`/`trim`/`ltrim`/`rtrim`.** First v0.92.0-roadmap installment. New `KindReq::Boxed` variant; 6 `extern "C"` shims in `jit/builtins.rs` that `borrow_tagged()` the argument and either return an i64 (`len → Int`) or `arena::box_into_active(CfmlValue::string(...))` and return the tagged pointer (case/trim → Boxed). Analyser's Call::Builtin arm relaxed from "Int/Float only" to "Int/Float/Boxed"; lookup_overload now decides per-shim. translate.rs needs **no codegen changes** — Backend already maps `Kind::Boxed → I64` in the ABI signature builder, and `to_i64(v, Kind::Boxed) = v` is the existing pass-through. Bit-exact with `cfml-stdlib::builtins::fn_len`/`fn_ucase`/etc. Engine version 0.98.0 → 0.99.0 (note: JIT and engine track diverged since v0.91.x — `v0.92.0`–`v0.98.0` tags were taken by interim engine bug-fix releases). **Perf**: dedicated kernel `function clean(s) { return uCase(trim(s)) & "|" & lCase(trim(s)) & "|" & len(s); }` over 5M iters: 91.97s → 14.68s (~5.3×). The 4 in-repo bench/baseline kernels unchanged within noise. **Tests**: 65 unit (+2 — `boxed_overloads_match_only_boxed_args`, `boxed_shims_match_interpreter`) + 32 e2e (+3 — `ucase_lcase_concat_in_jitted_udf_matches_interpreter`, `len_of_boxed_string_arg_returns_int_in_jit`, `trim_family_in_jitted_udf_matches_interpreter`), full suite 3571/3571 identical RUSTCFML_JIT=0 vs RUSTCFML_JIT_THRESHOLD=1, wasm32 green. |
| v0.99.1 | `c517dfa` | **6 more Boxed-arg shims** — `reverse` / `stripCr` / `htmlEditFormat` / `htmlCodeFormat` / `encodeForHtml` (Boxed→Boxed) + `asc` (Boxed→Int). Mechanical extensions on v0.99.0 infrastructure (no new plumbing). Brings the Boxed-arg surface to 12 entries. **Perf**: 1.36× on a shim-heavy kernel (`htmlEditFormat` + `reverse` + `asc` + `stripCr` + concat × 2M; 10.78s → 7.90s); alloc-dominated. **Tests**: 67 unit (extended in place) + 35 e2e (+3), suite 3571/3571 identical, wasm green. |
| v0.99.2 | `63aff83` | **11 more mechanical Boxed shims** — single-arg infallible (`urlEncodedFormat`/`urlDecode`/`jsStringFormat`, Boxed→Boxed) + multi-arg (`left`/`right`(Boxed,Int)→Boxed; `mid`(Boxed,Int,Int)→Boxed; `repeatString`(Boxed,Int)→Boxed; `find`/`findNoCase`(Boxed,Boxed)→Int; `replace`/`replaceNoCase`(Boxed,Boxed,Boxed)→Boxed — 3-arg default scope=one). NO new shim plumbing — translate.rs's existing Builtin codegen is fully generic over arity × Kind combination (`to_i64(Boxed) = v` pass-through, `to_i64(Int) = v` no-op). Brings Boxed-arg shim surface from 12 → 23 entries. **Tests**: 67 unit (boxed_overloads test extended for (Boxed,Int)/(Boxed,Int,Int)/(Boxed,Boxed)→Int/(Boxed,Boxed,Boxed) shapes) + 39 e2e (+4: url_and_js_string_format / left_right_mid / find_and_replace / repeat_string). Full suite 3571/3571 identical, wasm green. |
| v0.99.3 | `9810db3` | **Bail plumbing for builtin shims + arrayLen/structKeyList.** New `Shim.bailable: bool`. Bailable shims get a trailing `*mut i64` bail param in their signature (`ptr_ty` appended to `shim_ids` Signature when `bailable`); translator + osr both append `b.use_var(bail_var)` to the call arglist and emit a post-call `brif bail, bail_block, cont` (mirrors UDF Call dispatcher's bail pattern, gotcha #20 generalised). On bail the engine re-runs the function under the interpreter so cftry/cfcatch and the runtime error path execute identically. Two new shims: `arrayLen(Boxed) → Int` (BAILABLE — Lucee@7 parity: QueryColumn errors; the shim sets `*bail = 1` and the interpreter throws `Can't cast`) and `structKeyList(Boxed) → Boxed` (infallible — non-struct returns empty string; default delimiter `,`; visible_struct_keys hides arguments-scope markers). 55 existing shims kept `bailable: false`. **Tests**: 67 unit + 42 e2e (+3: array_len_shim / array_len_bails_on_query_column / struct_key_list_shim — the bail test exercises the full re-interpret-on-bail path through cftry/cfcatch). Full suite 3571/3571 identical, wasm green. |
| v0.99.4 | `70dd201` | **`shape_id` on CfmlStruct (no JIT change yet — pure plumbing for v0.99.5).** Wraps inner `Arc<RwLock<IndexMap<String, CfmlValue>>>` with `Arc<RwLock<StructInner { map, shape_id: u64 }>>`. Shape IDs allocated from a process-wide `AtomicU64` counter (`0` reserved as never-populated sentinel — uninitialised IC slots always miss). Bumps on **structural** change only: new key inserted (`insert` when prev is None), `remove`/`remove_ci` when a key was actually removed, `clear` when non-empty before, `get_or_insert_struct` when key was absent or held a non-struct, and unconditionally on every `with_write` (the closure has arbitrary access). Does NOT bump on value-only updates (same key, new value) so JIT ICs stay warm. Public API: new `CfmlStruct::shape_id() -> u64`, `get_at_index(usize) -> Option<CfmlValue>`, `get_ci_indexed(&str) -> Option<(usize, CfmlValue)>`. Every existing CfmlStruct method keeps its signature unchanged; downstream crates needed zero edits. `CfmlValue::Struct` still an 8-byte Arc handle (no size change). **Tests**: cfml-common 1/1, JIT 67 unit + 42 e2e, full suite 3571/3571 both ways. wasm32 green. **Pure plumbing — zero perf change**. |
| v0.99.5 | `12247c2` | **Monomorphic member-access IC for GetProperty + LoadLocalProperty** (Phase-3-equivalent first cut). New `cfml_jit_member_get_boxed(obj_tagged, name_ptr, name_len, ic_slot, bail) -> i64` shim in `jit/shims.rs`. **Fast path**: `cmp s.shape_id(), cached_shape`; on match `s.get_at_index(cached_idx)` (O(1) IndexMap position lookup); arena-box; return tagged ptr. **Cold path**: `s.get_ci_indexed(name)` (case-insensitive scan via the new v0.99.4 helper); update `[cached_shape, cached_idx]`; return value. Missing key returns Null. **Bails when receiver is not a `CfmlValue::Struct`** — Components / Queries / Closures / NativeObjects keep the more elaborate interpreter dispatch. Per-call-site IC storage in `Backend.member_ic_slots: Vec<Box<UnsafeCell<[u64;2]>>>` (each Box is a 16-byte stable heap allocation; the Vec just holds 8-byte Box handles, so growing it doesn't move the slots — pointers stay valid for the Backend's lifetime). Property names interned via `Backend.member_names: Vec<Box<str>>` (same stable-address discipline as `string_literals`, gotcha #15). Analyser admits both ops: `LoadLocalProperty(local, prop)` requires the local slot to be Boxed, pushes Boxed; `GetProperty(prop)` pops Boxed, pushes Boxed. **OSR intentionally NOT extended** — its catch-all `_ => return None` rejects loops containing these ops, so OSR transparently skips them (v0.99.6 follow-up). **Tests**: 67 unit + 46 e2e (+4: member_get_loadlocalproperty / member_get_case_insensitive (mixed-case key resolves via cold-path CI scan, then warm path) / member_get_missing_key_returns_null / member_get_bails_on_non_struct_receiver). Full suite 3571/3571 identical, wasm green. **Perf**: 7.86s → 5.83s (~1.35×) on `function get(p) { return p.firstName & "|" & p.lastName & "|" & p.age; }` × 3M iters; `fn_compiled=1` confirms IC engages. First perf win on struct_member-shaped workloads (1.00× pre-v0.99.5). Numeric struct kernel (`p.a + p.b + p.c + p.d`) still 1.00× because `Add` rejects Boxed operands — see v0.99.6 candidate below. |
| v0.99.7 | `c3222f4` | **Sub/Mul admit Boxed operands** — mirror of v0.99.6 Add. New `cfml_jit_sub_boxed`/`cfml_jit_mul_boxed` extern "C" shims (SMI fast path; numeric_op-equivalent coercion fallback). `add_boxed_smi` generalised to `arith_boxed_smi(NumOp, slow_shim)`; Backend gets `sub_boxed_id`/`mul_boxed_id`; `BytecodeOp::Add|Sub|Mul` share one arm in translate.rs. Analyser's Sub/Mul arms now use `num_bin_kind` (drops the `is_num()` guard that rejected Boxed). **Perf**: full struct_member shape (`p.a + p.b * 2 + p.c - p.d`) × 10M iters 22.32s → 13.46s (**1.66×**). **Tests**: 78 unit (+4 shim) + 54 e2e (+4 sub/mul Boxed UDF). Suite 3571/3571 identical, wasm green. No regression on numeric/udf_call_graph baselines. |
| v0.99.8 | `ebd19f4` | **OSR admission for member reads + Boxed arith.** Mechanical mirror of v0.99.5/6/7 inside `osr.rs::compile_loop` + `analyze_loop` + `simulate_block`. (1) op-subset arm admits `LoadLocalProperty` (interns the local slot Boxed) + `GetProperty`, both counted as useful work; (2) simulate_block admits the member ops and relaxes Add/Sub/Mul to `analysis::num_bin_kind` (Boxed-aware); (3) compile_loop pre-scans member sites + alloc IC slots, imports member_get + add/sub/mul_boxed shim refs, routes Add/Sub/Mul through `arith_boxed_smi` when either operand Boxed. Added an OSR-local **slot-kind Infer fixpoint** so `total = total + obj.prop` (initially-Int slot, body stores Boxed) widens to Boxed across iters. `build_caller_kinds` seeds slots from `LoadLocalProperty` receivers too. **Perf**: `bench/baseline/struct_member_kernel.cfm` (`__main__` outer loop, 5M iters `total + obj.x + obj.y * 2 + obj.z - obj.w`) **1.86s → 0.19s (~9.8×)** — was 1.02× pre-v0.99.8. **Tests**: 78 unit + 58 e2e (+4 OSR member). Suite 3571/3571 identical, wasm green. |
| v0.100.0 | `484c6e8` | **SetProperty/StoreLocalProperty IC (write-side mirror of v0.99.5).** Per-call-site monomorphic IC for `obj.prop = value` (SetProperty) + `local.prop = value` (StoreLocalProperty) on plain-Struct receivers. Same `[shape, idx, kind]` slot layout. Hot path: shape-match → new `CfmlStruct::set_at_index(idx, val)` does value-only update at known index WITHOUT bumping shape_id, so reader-side ICs for the same shape stay warm. Cold path: `get_ci_indexed` → either overwrite at found idx (no bump) or `insert` new key (shape bumps; IC re-populated to new shape). **Bails** on non-Struct + Components/NativeObject parents (any Struct with `__variables`/`__properties`/`__super` markers). Analyser admits values as Int/Float/Boxed. **OSR also extended in same commit.** **Perf**: write_ic_bench (5M iter × 2 writes/iter × 100 outer calls): **2.45s → 0.57s (~4.3×)**. **Tests**: 81 unit (+3 shim) + 63 e2e (+5). Suite 3571/3571 identical, wasm green. |
| v0.102.0 | _pending_ | **SMI-safety sweep over pre-v0.101.0 Boxed shims.** Pure refactor (~28 call sites across ~24 shims): every shim in the v0.92.0/v0.99.0–v0.99.3 block now materialises its Boxed input via `boxed::materialize_tagged` instead of `boxed::borrow_tagged`. The v0.99.6+ member-IC can return SMI Int inline (low-bit-tagged i61); `borrow_tagged` panics on non-TAG_PTR. Latent bug since v0.99.6, untriggered in repo but trivial to trip in real CFML (e.g. `len(p.age)` after a GetProperty IC populates with Int shape). v0.101.0 shims already used `materialize_tagged`; this brings the older surface in line. Two `match v` → `match &v` in `cfml_len_boxed_i64`/`cfml_array_len_boxed_i64` (arms reference `v` directly); one `if let X = v` → `if let X = &v` in `cfml_struct_key_list_boxed`. **Tests**: 88 unit (+1 `pre_v0101_shims_accept_smi_int_inputs` — passes `try_tag_smi_int(42)` through `len` / `uCase` / `asc` / `find` / `arrayLen`; pre-fix all five would abort) + 70 e2e + 5 rec + 4 soak + 2 fuzz. Suite 3612/3612 identical RUSTCFML_JIT=0 vs THRESHOLD=1, wasm32 green. No perf change (refcount-bump clone is negligible vs the shim body work). |
| v0.101.0 | `76218bf` | **13 Boxed-arg predicate/collection shims** — mechanical extension of v0.99.0–v0.99.3 surface; Boxed-arg shim surface 25→38. Type predicates (Boxed→Boxed Bool): `isNumeric`, `isArray`, `isStruct`, `isBoolean`, `isSimpleValue`, `isNull`. Collection predicates: `arrayIsEmpty`, `structIsEmpty` (Boxed→Boxed Bool); `structCount`, `listLen` (Boxed→Int); `arrayToList` (Boxed→Boxed). 2-arg: `structKeyExists`, `arrayContains`, `arrayContainsNoCase` (Boxed,Boxed→Boxed Bool). Bool-returning shims wrap `CfmlValue::Bool` into the active arena (Kind::Bool can't escape stack-to-local/return — stringifies "YES"/"NO" not "1"/"0"). **All new shims use `materialize_tagged`, NOT `borrow_tagged`** — since v0.99.6 the member-IC may return SMI Int inline and `borrow_tagged` panics on non-TAG_PTR. **Existing v0.99.0–v0.99.3 shims still use `borrow_tagged`** (latent bug — listed as next-up sweep). Codegen quirk: `isNull(bareIdent)` is special-cased to TryLoadLocal+IsNull (bypasses Call path); the new `isnull` shim only fires on non-identifier args. **Tests**: 82 unit (+1 predicate_shims_match_interpreter; boxed_overloads test extended) + 70 e2e (+7) + 5 rec + 4 soak + 2 fuzz. Suite 3612/3612 identical, wasm green. |
| v0.91.1 | `09e0493` | **OSR-UDF admission heuristic.** `analyze_loop` counts useful-work ops in the region (arithmetic / comparison / logic / String / Concat / native builtin Call) and rejects OSR when the region has any UDF call AND useful_ops < `MIN_USEFUL_FOR_UDF` (=2). Sized so the bench's 1-Add-per-UDF-call shape rejects, but realistic mixed-work loops (≥2 useful ops) admit. Restores baseline `udf_call_graph` perf (78× ≈ 82× baseline) without losing the v0.91.0 capability for loops that actually benefit. **Tests**: 65 unit + 29 e2e (1 new regression guard — `osr_rejects_thin_udf_wrapper_loop`), full suite 3581/3581 identical, wasm32 green. |

## Polymorphic roadmap (post-v0.88.0)

The coverage signal on `tests/runner.cfm` validates Option-γ: 989 functions,
**16% JIT today → 39% admissible under Option-γ** (+23pp = doubling). Top
non-supported ops are `String` (12.5k), `LoadGlobal` (9.9k), `Call` (9.9k),
followed by member ops in the few-hundreds. See `JIT_POLY_DESIGN.md` for
the full plan; sequenced phases:

- **v0.89.0** — ✅ SHIPPED: Boxed scalar in/out at the ABI.
- **v0.90.0** — ✅ SHIPPED: mid-body Boxed via per-call arena + box/concat
  shims; analyser admits `BytecodeOp::String(_)` and `BytecodeOp::Concat`.
- **v0.90.1** — ✅ SHIPPED (pushed + tagged `c89d00b`): UDF→UDF Boxed
  dispatch. Tri-state `expected_ret_kind`; resolver admits Boxed-arg /
  Boxed-ret callees; translator pushes Boxed results into the
  v0.90.0 pipeline. **Did NOT move string_kernel** (still 1.29×, within
  noise) because the bench's outer loop lives in `__main__`, which the
  whole-function analyser still refuses, and OSR doesn't admit UDF calls
  inside hot-loop regions. The new e2e tests demonstrate the dispatch
  works end-to-end — wins materialise only in non-main JIT-eligible
  chains. Real string-heavy unlock needs either (a) the analyser to
  admit `__main__`, or (b) OSR to accept UDF calls (see below).
- **v0.91.0** — ✅ SHIPPED (pushed + tagged `02ab030`): Option B —
  OSR Boxed slots + UDF dispatch. OSR now matches whole-fn JIT's
  v0.90.0+v0.90.1 capability set; hot loops in any enclosing function
  (including `__main__`) can compile when the body uses `Kind::Boxed`
  locals, String literals, `&` concat, and direct calls to other JIT'd
  UDFs. **Surfaced a 6.2pp regression on `udf_call_graph` because OSR
  engaged on thin UDF-call-wrapper loops where the dispatcher libcall
  is a net pessimization — closed in v0.91.1 with an admission
  heuristic.**
- **v0.91.1** — ✅ SHIPPED (pushed + tagged `09e0493`): OSR-UDF
  admission heuristic. Counts useful-work ops in the region; rejects
  OSR when any UDF call exists AND useful_ops < 2. Restores
  `udf_call_graph` baseline. Preserves the v0.91.0 capability for
  mixed-work loops.
- **v0.92.0+** — Two main directions still open, scope with the user
  before starting:
  - **Phase-3-equivalent member ICs**: shape IDs on
    `CfmlStruct`/`CfmlComponent`, monomorphic ICs over
    `LoadLocalProperty`/`GetProperty`/`CallMethod`/`SetProperty`
    (~1.5k ops; OO/WireBox code). Targets `struct_member_kernel`
    (currently 1.00×).
  - **String/array shim surface** on top of the Boxed infrastructure
    (`len`, `uCase`, `arrayLen`, `[idx]` indexing, …). Mechanical, like
    Option-A was, but on Boxed operands. Targets `string_kernel`-shape
    real workloads now that the IR pipeline is in place.

Real-world baseline (v0.87.0, captured 2026-06-09 in `bench/baseline/`,
*uncommitted* working files):
- numeric_kernel: 43.08s interp → 0.24s jit (**179×**)
- udf_call_graph: 78.33s → 0.95s (**82×**)
- string_kernel: 0.88s → 0.89s (**1.00×** — Option-γ target)
- struct_member_kernel: 1.76s → 1.75s (**1.01×** — v0.91.0 target)
- tests/runner.cfm full suite: 35.83s → 36.72s (**0.98×** — mostly cold paths)

**v0.90.0 measurement (2026-06-09, same harness):**
- numeric_kernel: 44.10s interp → 0.24s jit (**183×** — within noise)
- udf_call_graph: 78.28s → 0.95s (**82×** — unchanged)
- string_kernel: 0.89s → 0.68s (**1.31×** — v0.90.0 unlock, `buildLine`
  inner concat loop now JITs; `__main__`'s outer loop calling `buildLine`
  still interprets and caps the win)
- struct_member_kernel: 1.76s → 1.72s (**1.02×** — v0.91.0 target)
- tests/runner.cfm full suite: 31.34s → 31.32s (**1.00×** — mostly cold
  paths; v0.90.0 has no negative impact)

**v0.90.1 measurement (2026-06-09, same harness, pre-push):**
- numeric_kernel: 43.31s interp → 0.24s jit (**180×** — within noise)
- udf_call_graph: 78.06s → 0.95s (**82×** — unchanged)
- string_kernel: 0.89s → 0.69s (**1.29×** — within noise of v0.90.0;
  see "did NOT move" note in the roadmap above — `__main__` outer loop
  + OSR's no-UDF-calls restriction together gate the headline win)
- struct_member_kernel: 1.70s → 1.74s (**0.98×** — within noise)
- tests/runner.cfm full suite: 31.45s → 31.43s (**1.00×** — unchanged)

`fn_compiled=1` on `string_kernel.cfm` confirms `buildLine` enters native
code. v0.90.1 ships the dispatch plumbing (proven by 2 new e2e tests)
without a perf regression anywhere.

**v0.91.0 measurement (2026-06-09, post-push, *before* the v0.91.1 heuristic):**
- numeric_kernel: 43.67s interp → 0.24s jit (**182×** — within noise)
- udf_call_graph: 76.13s → 1.00s (**76×** — **6pp regression** vs 82×
  baseline; `osr_compiled=2` confirms the outer __main__ for-loops
  now OSR-compile, but the dispatcher libcall costs ~100ns/call vs
  the interpreter's already-cached `Call→try_call` path)
- string_kernel: 0.88s → 0.68s (**1.29×** — unchanged; `osr_compiled=2`
  shows both inner+outer loops compile, but inner dominates)
- struct_member_kernel: 1.68s → 1.74s (**0.97×** — within noise)
- tests/runner.cfm full suite: 31.88s → 52.08s (**0.61×** — *not real*;
  cfhttp/sleep tests are wildly variable on this suite; 5-trial stress
  showed interp 34-55s, JIT 41-4158s with hang outliers, same noise
  pattern on the v0.90.1+Option-A binary, so the bench's runner.cfm
  number is not signal)

**v0.91.1 measurement (2026-06-09, post-push, with admission heuristic):**
- numeric_kernel: 43.99s interp → 0.24s jit (**183×** — within noise)
- udf_call_graph: 74.77s → 0.96s (**78×** — back to baseline territory;
  `osr_compiled=0` confirms the heuristic correctly rejects the thin
  UDF-wrapper outer loops; direct 10-trial comparison vs v0.90.1's
  binary gave 81× ≈ 82× within tighter noise)
- string_kernel: 0.89s → 0.67s (**1.33×** — within noise; the heuristic
  also rejects string_kernel's outer loop, same as v0.91.0 net result)
- struct_member_kernel: 1.71s → 1.71s (**1.00×** — clean)
- tests/runner.cfm full suite: 41.08s → 30.87s (**1.33×** — also noise,
  see v0.91.0 row above)

These baseline numbers are the reference point for v0.92.0+ perf claims.
The shell harness lives in `bench/baseline/run_baseline.sh` (also
uncommitted, but durable across sessions because next-session can recreate
it from `JIT_POLY_DESIGN.md`).

## Settled design decisions (2026-06-08/09)

Recorded so next session doesn't re-litigate:

- Representation: **Option γ (hybrid tag-pointer)**, low 3 bits / 8 tags.
  Rejected Option α (boxed `*const CfmlValue`, too slow for Int-heavy hot
  paths) and Option β (NaN-box, too risky on PAC-enabled Apple Silicon and
  too large a silent-miscompile surface for our 3581-assertion suite to
  guard).
- String upgrade: **shipped v0.87.0** as `String(Arc<String>)`. Breaking
  change accepted.
- Tag-bit budget: **3 bits / 8 tags**, with headroom for future variants.
- Test bar: **stricter Phase-2 bar** (unit + e2e + full suite identical +
  wasm + differential fuzz + 30× soak + perf A/B per phase).
- Real-world baseline: **WireBox HTTP setup deferred** until v0.90.0+ when
  perf-delta numbers are needed (WireBox CLI invocation doesn't resolve
  components without HTTP serve mode). For v0.88.0 the runner.cfm + four
  in-repo kernels are sufficient signal.

## Files (status: through v0.91.1 pushed + tagged)

| File | What |
|------|------|
| `Cargo.toml` | cranelift 0.132 crates in `[workspace.dependencies]` |
| `crates/cfml-vm/Cargo.toml` | `jit` in default features; deps `optional` + target-gated `cfg(not(target_arch="wasm32"))` |
| `crates/cli/Cargo.toml` | `jit` in default features; passthrough `jit = ["cfml-vm/jit"]` |
| `crates/cli/src/lib.rs` | `--no-jit` / `--jit-threshold` / `--jit-stats` flags; `JIT_STATS_REQUESTED` atomic |
| `crates/cfml-vm/src/lib.rs` | `mod jit;`; `jit: Option<jit::JitEngine>` field; whole-fn hook at top of `execute_function_with_args` (now also builds the `udf_lookup` closure for Phase-1 resolution); OSR hooks at `ForLoopStep` / `Jump` / `JumpIfTrue` / `JumpIfFalse` matched-true branches; `jit_is_shadowed` free helper; **v0.91.0**: new free helper `jit_udf_lookup(user_functions, name) -> Option<jit::UdfMeta>`, passed into all 4 `try_run_loop` call sites so OSR can resolve UDF callsites; `jit_compiled_count` / `osr_compiled_count` accessors |
| `crates/cfml-vm/src/jit/mod.rs` | `JitEngine` (cache + hot + backend + osr_cache + osr_hot + osr_compiled), `HotnessTracker`, ABI trampolines `run_compiled` / `run_compiled_with_engine` / `run_osr_compiled` (v0.90.0: wraps in `ArenaGuard` and drains arena on the way out), `try_call`, `try_run_loop`, `build_caller_kinds`. **Phase 1**: `UdfMeta`, `UdfRefBinding`, `sig_from_kinds`, `udf_binding_still_valid`, `dispatch_jit_udf`, `ENGINE_PTR` thread-local. **v0.90.1**: `RetKind::as_code()` (0/1/2 encoding for the dispatcher ABI) + `RetKind::to_binding()`; `dispatch_jit_udf` takes `expected_ret_kind: i64` (was `expected_ret_float: bool`) with bail=2 on cached-vs-expected mismatch; resolver drops `sig_has_boxed` rejection and `RetKind::Boxed => None` arm; self-call mistype check generalises to "ret is not Int". **v0.91.0**: `build_caller_kinds` emits `Kind::Boxed` for any non-Int/Float live local (was: silent drop); `OsrCompiled` carries `referenced_udfs`; `run_osr_compiled` installs per-call `ArenaGuard`, box-clones entry Boxed slots, borrows + clones-out exit live values, drops guard, drains; `try_run_loop` takes a `udf_lookup` callback + builds an analyser resolver that consults the engine's cache (speculates `BindingRet::Int` for not-yet-compiled callees, same as whole-fn), revalidates `referenced_udfs` on every cached-OSR call. |
| `crates/cfml-vm/src/jit/arena.rs` | **v0.90.0**: per-call `Arena` (Vec<usize> of tagged ptrs), `ArenaGuard` (RAII install/restore via thread-local), `track`/`box_into_active` helpers used by shims |
| `crates/cfml-vm/src/jit/shims.rs` | **v0.90.0**: `cfml_jit_box_int`, `cfml_jit_box_float`, `cfml_jit_concat_boxed`, `cfml_jit_add_boxed` (extern "C" runtime shims; bit-exact with interp Add/Concat) |
| `crates/cfml-vm/src/jit/analysis.rs` | whole-fn eligibility/CFG/dataflow → `Plan`; `Kind` lattice (Int / Float / Bool / Builtin / **UdfRef** / **Boxed**). `analyze(func, kinds, udf_resolver)`. **v0.90.0**: `BytecodeOp::String(_)` → `Kind::Boxed`; `BytecodeOp::Concat` admits any combo of Int/Float/Boxed operands. **v0.90.1**: new `BindingRet` enum (Int/Float/Boxed) replaces `UdfRefBinding.ret_float: bool`; `Call` site splits arg-kind validation per-marker (builtins still Int/Float-only, UDF callsites also admit Boxed). |
| `crates/cfml-vm/src/jit/translate.rs` | `Backend` (owns `JITModule`), `compile()` (wraps `compile_inner` with fbc-reset-on-error), bytecode → Cranelift IR. Phase-1: `cfml_call_jit_udf` + per-fn `udf_args_slot`. **v0.90.0**: pre-declared box_int/box_float/concat_boxed/add_boxed/str_literal `FuncId`s + per-Backend `string_literals: Vec<Box<str>>` interning + `ensure_boxed` helper + codegen for `BytecodeOp::String(_)` and `BytecodeOp::Concat`. **v0.90.1**: `cfml_call_jit_udf` extern's `expected_ret_float: i64` → `expected_ret_kind: i64` (0/1/2); call-site emits the 0/1/2 code from `binding.ret_kind` and pushes the result as `Kind::Boxed` when `BindingRet::Boxed`. |
| `crates/cfml-vm/src/jit/osr.rs` | `LoopPlan`, `OsrSlot`, `analyze_loop` (accepts ForLoopStep / Jump / JumpIfTrue / JumpIfFalse back-edges to `region_start`), `compile_loop` with in/out ABI `fn(io_locals: *mut i64, bail: *mut i64)`; unit tests for analyse + compile + round-trip. **v0.91.0**: admits `Kind::Boxed` slots; admits `BytecodeOp::String(_)` / `BytecodeOp::Concat`; `analyze_loop` takes a `UdfResolver` and intern UDF names + resolves bindings at simulate_block; `LoopPlan` carries `udf_call_at` + `referenced_udfs`; `compile_loop` imports box_int/box_float/concat_boxed/str_literal/udf_dispatch shim refs, interns string literals before the FunctionBuilder, allocates a per-region UDF args stack slot, and emits the `cfml_call_jit_udf` libcall with bail=2 handling. **v0.91.1**: `MIN_USEFUL_FOR_UDF = 2` admission heuristic — count arithmetic/comparison/logic/String/Concat/native-builtin-Call ops; reject if any UDF call exists AND useful_ops + builtin_calls < 2. |
| `crates/cfml-vm/src/jit/builtins.rs` | `Shim` table: 22 entries — abs/min/max + math + bit-twiddling + pow |
| `crates/cfml-vm/tests/jit_numeric.rs` | 29 e2e tests (v0.90.0 added `string_literal_pass_through_jits`, `boxed_concat_in_jitted_udf_matches_interpreter`, `boxed_concat_with_float_operand_matches_interpreter`; v0.90.1 added `jit_caller_invokes_boxed_returning_udf_and_matches_interpreter`, `jit_caller_threads_boxed_arg_through_to_jitted_callee`; **v0.91.0 added `osr_boxed_concat_loop_in_main_matches_interpreter`, `osr_calls_jitted_udf_from_outer_loop_in_main`; v0.91.1 added regression guard `osr_rejects_thin_udf_wrapper_loop`**) |
| `JIT_DESIGN.md` | Tier-1/1.5 architecture write-up (historical accurate) |
| `JIT_OSR_DESIGN.md` | OSR Phase 1+2 design (done; reference) |

## Build / test / run cheatsheet

```bash
# unit + e2e (jit on by default now; no --features jit needed)
cargo test -p cfml-vm --lib jit::                           # 65 unit
cargo test -p cfml-vm --test jit_numeric -- --test-threads=1   # 29 e2e
cargo test -p cfml-vm --test jit_recursion --test jit_soak --test jit_fuzz --release -- --test-threads=1   # 5 recursion + 4 soak + 2 fuzz

# full suite both ways (must be identical)
RUSTCFML_JIT=0 cargo run -- tests/runner.cfm                # interpreter
RUSTCFML_JIT_THRESHOLD=1 cargo run -- tests/runner.cfm      # JIT aggressive

# unaffected-build guards
cargo build --no-default-features --features std,real-threads -p cfml-vm   # no JIT
cargo build -p cfml-worker -p rustcfml-wasm --target wasm32-unknown-unknown

# A/B perf
cargo build --release
RUSTCFML_JIT=0 ./target/release/rustcfml file.cfm           # interpreter
./target/release/rustcfml file.cfm                          # JIT on (default)
./target/release/rustcfml --jit-stats file.cfm              # report counts
```

Env / CLI knobs:
- `RUSTCFML_JIT=0` / `--no-jit` — disable
- `RUSTCFML_JIT_THRESHOLD=N` / `--jit-threshold N` — change threshold (default 50)
- `--jit-stats` — print `fn_compiled=N osr_compiled=M` to stderr after run

## Hard-won gotchas (don't relearn these)

1. **`StoreLocal` pops its value**, and codegen emits a spurious statement-level `Pop` after it. The interpreter's `Pop` ignores empty-stack — so analysis + translate must treat **`Pop`-on-empty as a no-op**.
2. **cranelift 0.132 `declare_var(ty)` returns a fresh `Variable`** (no caller-chosen index). Keep a `slot → Variable` table.
3. `Module` trait must be in scope for `make_context` / `declare_function`.
4. CFML int overflow **wraps** — use plain `iadd`/`isub`/`imul`.
5. Analysis runs over the **reachable** CFG so the dead trailing `Null; Return` epilogue never disqualifies a function.
6. **Booleans never enter a slot or escape via a non-branch consumer.**
7. **OSR nested-loop bug**: inner `ForLoopStep`'s matched=false branch must fall through to the next basic block, NOT to `writeback_block`. Only the *outermost* step exits to writeback. Regression-tested.
8. **OSR closure_env sync**: every write-back slot must also be propagated to `closure_env` when it tracks that name — sibling closures otherwise see stale values.
9. **`bitNot` Lucee parity**: shim truncates to `i32` before `!`, then re-extends to `i64`. Rust's `!i64` on a 64-bit value would diverge in bits 32+.
10. **`round()` Lucee parity**: shim uses `(x + 0.5).floor() as i64` (half-up toward positive infinity, matching Java `Math.round`). Rust's `f64::round` is half-away-from-zero — would diverge on negatives.
11. **OSR `Jump`-back hook**: `ip` is post-incremented past the Jump op, so the back-edge test is `*target < ip - 1`. The region is `[target, ip)`, exit_ip == ip.
12. **`FunctionBuilder` drop without `finalize()` corrupts `FunctionBuilderContext`** — the next `FunctionBuilder::new` panics on `assertion failed: func_ctx.is_empty()`. Any `return Err(...)` inside the IR-emit block triggers this. Fix: `compile` wraps `compile_inner` and resets `self.fbc = FunctionBuilderContext::new()` on Err. Cheap, eliminates a whole class of latent panics. Was actually triggered by Phase-1 codegen disagreement (LoadGlobal of a non-builtin name now legal in analysis, was still Err in codegen) — the codegen mismatch is fixed, and the resilience layer means future analyser/translator drift can't crash either.
13. **UDF→UDF self-recursion needs insert-before-run** — cache the new entry first, then run the compiled body. Otherwise the body's first self-call via the dispatcher misses the cache and bails. With insert-before-run, native recursion works on the very first outer call.
14. **Self-recursion ret-kind is bootstrap-circular**: at analysis time you don't know your own return kind, but typing a self-call needs it. Phase-1 optimistically binds `ret_float = false`, then post-checks: if the body's actual `ret_kind` turned out `Float` AND any referenced UDF binding has `(global_id, sig)` == caller's, reject. Float-returning self-recursive functions are not JIT'd in Phase 1.
15. **v0.90.0 string literals must be interned BEFORE the `FunctionBuilder` is constructed.** `intern_literal(&mut self, …)` needs `&mut self.string_literals`, but `FunctionBuilder::new(&mut ctx.func, &mut self.fbc)` already holds borrows on neighbouring fields — Rust will refuse the split inside the IR-emit block. Pre-scan all `BytecodeOp::String(_)` IPs into a `HashMap<usize, (*const u8, i64)>` first; codegen reads from that map by IP.
16. **v0.90.0 arena lifetime**: `ArenaGuard` must be `drop`-ped *before* the engine touches `arena` directly (drain_except), or else the still-installed thread-local pointer aliases the `&mut Arena` borrow. Pattern: install guard → call body → `drop(_guard)` → `arena.drain_except(keep)`.
17. **v0.90.0 `pop_value!` rejects `Kind::Boxed`** by design (it polices arithmetic / comparison / logic operand kinds against the monomorphic lattice). New ops that *do* accept Boxed (`Concat`, `Return` via `pop_assignable!`, `StoreLocal` via `pop_assignable!`) must NOT route through `pop_value!`. Use the bare `pop!` and re-check the operand kind locally.
18. **v0.90.0 interpreter-`Add` of (String, String) is concat, not numeric coercion.** Looks like a bug — Lucee parses `"3" + "4"` as 7 — but it's the documented RustCFML behaviour today. The v0.90.0 `cfml_jit_add_boxed` shim mirrors it bit-for-bit so the JIT can't diverge; if/when the interpreter is fixed the shim must follow.
19. **v0.90.1 self-call mistype generalises.** Before v0.90.1 the post-analysis check rejected the JIT when `ret_kind == Float` and a self-call existed (the resolver had typed it Int). Now `BindingRet` carries three states and the resolver still speculates `Int` for the self-call case; the post-check must reject for `Float` OR `Boxed` ret_kinds. Mistakenly leaving it as `is_float()` would silently miscompile a Boxed-returning self-recursive function.
20. **v0.90.1 UDF args can carry Boxed across the call boundary.** A Boxed value handed to the dispatcher is just an `i64` tagged pointer that lives in the *caller's* arena (the dispatcher invokes the callee's compiled body directly — NOT via `run_compiled` — so no new `ArenaGuard` is installed). That's correct as long as the callee doesn't keep the pointer past its own return: today it can't, because there's nothing that escapes a JIT body other than the return value or shim-allocated boxes (also caller-arena). If a future shim mutates global state and stashes a tagged pointer, this assumption breaks.
21. **v0.90.1 `wrap` as a CFML function name collides with a stdlib builtin.** Pure test-authoring pitfall: any name in the builtins table will get the "already used by a built in Function" rejection at parse-time. Use a non-builtin name (the e2e suite uses `passThrough` for the same shape).
22. **v0.91.0 hotness gate prevents `__main__` from ever JITing in CLI mode.** `HotnessTracker::record_and_is_hot` returns `true` exactly when `count == threshold + 1`. `__main__` is called *once* per CLI process, so threshold=50 means never; even threshold=1 means never (count reaches 1, needs 2 to fire). Option A's whole-fn admission of `__main__` therefore has zero CLI-perf impact by design — it's a serve-mode-only unlock (per-request `__main__` crosses the threshold after enough hits). Don't waste time hunting for a CLI win from Option A.
23. **v0.91.0 OSR-UDF without a heuristic regresses on thin UDF-wrapper outer loops.** Surfaced as 82× → 75× on `udf_call_graph.cfm`. The `cfml_call_jit_udf` libcall costs ~100ns/call more than the interpreter's already-cached `Call → try_call → compiled body` path; when the outer body has no real work to amortise against (e.g. `total = id(k);`), OSR is a net pessimization. v0.91.1's `MIN_USEFUL_FOR_UDF = 2` heuristic in `analyze_loop` is the fix: count arithmetic / comparison / logic / String / Concat / native-builtin-Call ops; reject if a UDF call exists AND the count is too low. Bench `bench/baseline/udf_call_graph.cfm` is the regression guard.
24. **v0.91.0 OSR caller_kinds widens silently.** `build_caller_kinds` now emits `Kind::Boxed` for *any* non-Int/Float live local — so a Boxed slot intern can succeed even though the analyser was historically conservative there. The cost is one-time per loop site (cached Unjittable on failure), but it widens the set of loops that *attempt* analysis. Keep an eye out if compile-time on serve-mode startup creeps up.
25. **v0.91.0 OSR Boxed marshalling: ArenaGuard drop ordering matters as much for OSR as for whole-fn (gotcha #16).** The `run_osr_compiled` pattern is: install guard → marshal-in box-clones → run compiled body → borrow-and-clone-out live values → **drop guard** → `arena.drain_except(None)` → writeback `locals`/`closure_env`. Touching `arena` while the guard's thread-local pointer is still installed aliases the `&mut Arena` borrow.
26. **runner.cfm is not a perf signal.** The bench script's `runner.cfm` row is noisy enough to be useless (cfhttp HTTP timeouts, sleep tests, occasional 4000s+ hangs). 5-trial stress test on a fixed binary showed interp 34-55s, JIT 41-4158s. Use the 4 in-repo kernels (numeric / udf_call_graph / string_kernel / struct_member_kernel) as the reliable signal; runner.cfm matters only for *correctness* (3540/3540 with JIT off vs threshold=1), not perf.

## v0.99.6 — gotcha additions

27. **v0.99.6 SMI Int range is i61, not i64.** `try_tag_smi_int(i)`
    returns `None` when `(i<<3)>>3 != i` — i.e. `i ∈ [−2^60, 2^60−1]`.
    Beyond that you must heap-box. The `cfml_jit_box_int` shim and the
    member-IC encode path both fall through correctly. Hot kernels
    operating on small integers (loop counters, struct members, IDs)
    never trip this; large fixnum bench data does (see test
    `add_boxed_smi_handles_large_int_via_box_int_overflow`).
28. **v0.99.6 `add_boxed_smi` lives next to `num_bin`, NOT inside it.**
    Routing decision is on the **caller side** at the bytecode-dispatch
    `BytecodeOp::Add` arm: peek the top two stack kinds, branch into
    `add_boxed_smi` if either is `Kind::Boxed`, else fall through to the
    classic `num_bin` (pure Int/Float fast path). Sub/Mul stay numeric-
    only — extending them needs matching `cfml_jit_sub_boxed` /
    `cfml_jit_mul_boxed` shims (mechanical) AND the `add_boxed_smi`
    helper rewritten to be generic in `NumOp`. v0.99.6 deferred this.
29. **v0.99.6 `BlockArg` wrapper for `jump`.** Cranelift 0.132's
    `InstBuilder::jump(block, &[BlockArg])` takes block args not raw
    `Value`s. Pattern:
    ```rust
    let arg: cranelift_codegen::ir::BlockArg = v.into();
    b.ins().jump(common, &[arg]);
    ```
    `From<Value> for BlockArg` covers the common case. Don't bind the
    `BlockArg` in a `let .. = v.into()` without the explicit type
    annotation — inference can fall over.
30. **v0.99.6 IC slot grew from `[u64; 2]` to `[u64; 3]`.** Backend
    field is `Vec<Box<UnsafeCell<[u64; 3]>>>`. The third slot
    (`cached_kind`) is 0 = uninitialised, 1 = Int (SMI fast), 2 =
    Double (heap box for now), 3 = other (heap box). Sentinel `shape=0`
    + `kind=0` is "never populated"; first call shape-misses and
    populates everything. Drift (`obj.x = "foo"` after the IC saw Int)
    updates `cached_kind` in place — shape doesn't change, just the
    encoding switches.
31. **v0.99.6 OSR-Boxed slot read-back now uses `materialize_tagged`,
    not `borrow_tagged().clone()`.** Because OSR slots can hold SMI
    tags after v0.99.6, the old `borrow_tagged` (which asserts
    `tag == TAG_PTR`) would panic. `materialize_tagged` is the
    polymorphic helper that synthesises `CfmlValue::Int` for SMI and
    clones the pointee for heap tags. Same change pattern applies to
    any future site that reads a Boxed slot back to a CfmlValue.

## Next options — scope with the user before starting big ones

### ✅ SHIPPED v0.99.7 — Sub/Mul on Boxed operands

Mechanical mirror of v0.99.6 Add. Sub/Mul now admit Boxed and route
through `arith_boxed_smi(NumOp)` with matching slow shims. UDF-wrapped
struct_member shape: **1.66×**. **The `bench/baseline/struct_member_kernel.cfm`
(`__main__` outer loop) still won't move** — its body uses `obj.prop`
member reads + Add/Sub/Mul in `__main__`. CLI `__main__` doesn't trip the
hotness threshold (gotcha #22), and OSR rejects GetProperty + Boxed
arith. That bench moves once v0.99.8's OSR-admission lands (see next).

### v0.99.7 candidate — Float SMI / NaN-pun (Phase B)

Float values currently always allocate when crossing the Boxed
boundary. Phase B encodes f64 via NaN-pun (use the QNaN payload bits
to store a pointer / inline the f64).

**Platform-gated**:
- `#[cfg(target_arch = "x86_64")]` — safe blanket-allow.
- `#[cfg(all(target_arch = "aarch64", not(target_vendor = "apple")))]`
  with a `--features unsafe-nanbox-aarch64` opt-in. Apple Silicon
  always-on PAC modifies high bits of pointers, which would conflict.
  Linux ARM (Graviton, Ampere, Pi 5) generally doesn't sign data
  pointers by default but is distribution-dependent; the opt-in
  feature lets users who know their kernel turn it on.
- Apple Silicon: stays heap-boxed.

Estimate ~400-600 LoC. Significantly more invasive than Int SMI
(every shim that materialises an operand has to learn the f64 tag).

### v0.99.7 candidate — Member-IC inline (skip the shim call entirely)

The v0.99.5/v0.99.6 IC is still a libcall: each `obj.prop` is a 5-arg
function call. On the hot path, that's ~10ns of call overhead per
access. For `p.a + p.b + p.c + p.d`, that's 40ns × 10M = 0.4s of
pure call overhead — measurable.

Inline-IR alternative: load `obj`'s tag check + shape pointer
directly into IR; cmp with cached_shape; fast-branch to
`s.get_at_index(cached_idx)` (still a single load + clone of an
IndexMap entry, but no full function call). Cold path remains a
shim call.

Estimate ~300 LoC. Could shave the `sum4` kernel from 13.2s to
~7-8s (closer to ~3× over interpreter). Combine with v0.99.7
Sub/Mul to make `struct_member_kernel` move too.

### v0.99.7 candidate — OSR admission for GetProperty / LoadLocalProperty

OSR's analyse_loop catch-all + simulate_block reject loops with these
ops in v0.99.6. Mechanical extension: mirror translate.rs's IC codegen
inside osr.rs::compile_loop, plus add the two ops to analyse_loop's
admit list and simulate_block's stack-kind sim. The IC slots can be
allocated through the same `alloc_member_ic_slot` helper. Estimated
~150 LoC, no design questions. Important for serve-mode
`__main__`-loop hot paths (CLI `__main__` doesn't JIT — see gotcha #22).

### v0.99.7 candidate — SetProperty IC (write path)

Mirror of the read IC for `obj.prop = value` and `local.prop = value`
(StoreProperty + StoreLocalProperty). The shim must:
- Cmp current shape with cached; if match, `s.with_write(|m| m.get_index_mut(idx)` to overwrite — but `with_write` bumps shape_id unconditionally (gotcha: that invalidates the IC slot we just used).
- A targeted helper `CfmlStruct::set_at_index(idx, val) -> bool` would let writes happen WITHOUT bumping shape (since it's a value-only update at a known index).

Real-world impact: WireBox-style component setters, `record.field = x`
loops. Probably ~250 LoC including the new helper.

## Recommended v0.102.0+ ordering

1. ~~**Sweep existing v0.99.0–v0.99.3 shims for SMI safety**~~ —
   **DONE in v0.102.0.** All ~24 older shims now use
   `materialize_tagged`; regression test
   `pre_v0101_shims_accept_smi_int_inputs` covers the Int-SMI →
   pre-v0.101.0-shim hazard.
2. **More Boxed-aware shims** (~mechanical, no new plumbing) —
   continue the v0.99.0–v0.99.3 + v0.101.0 surface expansion. Easy
   candidates: `arrayFirst`/`arrayLast`/`arrayAvg`/`arraySum`
   (Boxed→Boxed numeric); `listFirst`/`listLast`/`listRest`/`listGetAt`
   (Boxed→Boxed); `listAppend`/`listPrepend`/`listInsertAt` (Boxed
   multi-arg→Boxed); `arrayContainsType`-style; `arrayPush`/`arrayPop`
   mutation (needs care — non-pure). Profile-guided picks once
   real-world workloads run.
3. **Member-IC inline IR** (~300 LoC, design needed) — skip the
   libcall entirely. Handover notes flagged this as "lock+clone on
   hit is irreducible, max realistic win ~1.05×" when scoping v0.100.0,
   but worth re-checking when a real workload says the libcall
   overhead is measurable. Discuss with user first.
4. **CallMethod IC** (~500-800 LoC, design needed) — method-dispatch
   IC on Components. WireBox/OO surface. Needs shape model on
   `CfmlComponent` + vtable-style method cache + dispatch through
   `__super` chain. Targets `struct_member_kernel`-shaped OO workloads.
5. **Float SMI / NaN-pun (Phase B)** (~400–600 LoC, platform-gated) —
   `#[cfg(target_arch = "x86_64")]` blanket-allow, `#[cfg(all(target_arch = "aarch64", not(target_vendor = "apple")))]`
   behind `--features unsafe-nanbox-aarch64` opt-in (Apple Silicon PAC
   conflicts with high-bit NaN payload). Apple Silicon stays heap-boxed.
   Smaller wins in typical CFML (Int dominates); land last.

### Next — More Boxed-aware string/array shims *(continuation of v0.99.1)*

v0.99.0 shipped the first six (`len`, `uCase`, `lCase`, `trim`, `ltrim`,
`rtrim`); v0.99.1 added six more (`reverse`, `asc`, `stripCr`,
`htmlEditFormat`, `htmlCodeFormat`, `encodeForHtml`). All twelve are
infallible single-Boxed-arg shims. Natural follow-ups still on the
mechanical track (no new plumbing), in rough order of leverage:

1. **More single-arg infallible string formatters** — `lsParseEuroCurrency`
   (locale-free path), `formatBaseN` (2-arg Int+Int→Boxed), `urlEncodedFormat`,
   `urlDecode`. The URL family is mechanical (just `cfml-stdlib::fn_url_*`
   lifted).
2. **`arrayLen`** — like `len` but ERRORS on QueryColumn (per Lucee@7
   parity). Needs a bail mechanism on builtin shims: extend `Shim`
   with `bailable: bool`, append `*mut i64` to the call site, emit a
   `brif bail, …` after the call (the UDF call site does this
   already — mirror that). Once landed, also unlocks `structKeyList`,
   `arrayToList`, etc.
4. **2-arg string shims** — `mid(s, start, len)`, `replace(s, find,
   with)`, `find(needle, haystack)`. Just more entries in the table.
5. **Boxed args mixed with Int**: e.g. `mid(s, 2, 5)` accepts Boxed +
   Int + Int. The existing `KindReq` lattice already supports
   per-position acceptance; `to_i64(Boxed) = v` pass-through carries
   tagged ptrs, `to_i64(Int) = v` is a no-op for ints.

Bail plumbing for builtin shims is the one engineering item gating
several of the above. Sketch:

```rust
// in translate.rs Call::Builtin arm, after the `call`:
if shim.bailable {
    let bail_val = b.ins().load(I64, MemFlags::new(), bail_addr, 0);
    let bail_set = b.ins().icmp_imm(IntCC::NotEqual, bail_val, 0);
    let cont = b.create_block();
    b.ins().brif(bail_set, bail_block, &[], cont, &[]);
    b.switch_to_block(cont);
}
```

…and pass `bail_addr` as the trailing `*mut i64` arg in the call. The
shim does `unsafe { *bail = 1; return 0; }` on error.

### v0.93.0 — Member access ICs (Phase-3-equivalent)

Adds `shape_id` to `CfmlStruct` / `CfmlComponent` for monomorphic ICs.
`cfml_jit_load_member_boxed(obj, name, ic_slot) -> usize`. Targets
`LoadLocalProperty` / `GetProperty` / `CallMethod` / `SetProperty`
(~1.5k ops on the suite; OO/WireBox code). ~800-1200 LoC. Will move
`struct_member_kernel` (currently 1.00×).

### Tune the OSR-UDF heuristic against real workloads

`MIN_USEFUL_FOR_UDF = 2` is calibrated against the 4 in-repo kernels.
A real workload pass (WireBox controller, ColdBox route dispatch) might
reveal that K=3 catches more pessimizations, or that K=1 misses real
wins. Bench-sweep before re-tuning. Right now this is a single `const`
in `crates/cfml-vm/src/jit/osr.rs::analyze_loop`.

### Threshold tuning research *(no code; profile real workloads)*

Default threshold is 50. With v0.90.0/v0.90.1's Boxed-arg specializations
+ v0.91.0's OSR Boxed slots, the per-call cost profile has shifted.
Bench-sweep (25 / 50 / 100 / 200) on a representative real CFML app
before defaulting to a new value.

### Extract interpreter to `vm/interpreter.rs` *(cosmetic, risky)*

`lib.rs` is ~14k lines. Pulling the dispatch loop into its own file
would help readability but the refactor surface is large and the win is
purely cosmetic.

### Option C — NaN-box Tier-2 *(PARKED 2026-06-08; do not start without measured-workload evidence)*

~5k LoC of unsafe Cranelift + side-exit deopt state machine. NaN-box
high-bit assumptions can collide with PAC/MTE on Apple Silicon. Our
suite is semantics coverage, not deopt-state coverage. Option γ (boxed
pointer, fully rolled out as of v0.91.x) gives ~80% of the wins without
the deopt-machine surface area.

### Threshold tuning research *(no code; profile real workloads)*

The default threshold is 50. With v0.90.0/v0.90.1 the JIT now also attempts Boxed-arg specializations and UDF→UDF Boxed dispatch (which may bail on speculation mismatch), so the per-call cost profile may have shifted. Bench-sweep (25 / 50 / 100 / 200) on a representative real CFML app before defaulting to a new value.

### Extract interpreter to `vm/interpreter.rs` *(cosmetic, risky)*

`lib.rs` is ~14k lines. Pulling the dispatch loop into its own file would help readability but the refactor surface is large and the win is purely cosmetic.

### Option C — NaN-box Tier-2 *(PARKED 2026-06-08; do not start without measured-workload evidence)*

~5k LoC of unsafe Cranelift + side-exit deopt state machine. NaN-box high-bit assumptions can collide with PAC/MTE on Apple Silicon. Our suite is semantics coverage, not deopt-state coverage. Option γ (boxed pointer, currently being rolled out) gives ~80% of the wins without the deopt-machine surface area.

## Commit/push reminders (user prefs, from memory)

- **No `Co-Authored-By` lines on git commits.**
- **Always ask before `git push`.** Committing locally to `main` is fine when asked.
- **Don't commit working/handoff/planning docs** (this file, `JIT_DESIGN.md`, `JIT_OSR_DESIGN.md`) without explicit approval.
- Before tagging anything: also run the wasm32 build (CLAUDE.md warning).
