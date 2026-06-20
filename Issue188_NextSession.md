# Issue #188 — hasMany association-create hangs — RESOLVED

## Root cause (NOT what the prior handoff guessed)

The 97% CPU spin was **not** in the association/`compareTo` path. It was a
**`<cfloop>` code-generation bug**: the tag preprocessor lowered
`<cfloop from to index="local.i">` to `for (var local.i = 1; …)`. A C-style
`for (var local.i = …)` is malformed — `var` only accepts a SIMPLE name — and
RustCFML **dropped the initializer**: the counter started empty, lagged by one,
and over-ran (5→6 iterations). Wheels' `populate.cfm` nests three such loops
(`<cfloop query>` → `from/to index="local.i"` → `from/to index="local.i2"`)
to create galleries/photos; the malformed `var local.i` corrupted the function's
`local` scope and the loop never terminated — `model("gallery")` was called 1346+
times (its class created exactly once, so the model cache was fine).

`createPost`/`createProfile` themselves were never the problem — they complete;
populate just hangs a few lines later in the gallery loop. The reporter's
"hasMany create hangs" was the gallery `model().create()` loop, not the dynamic
finder.

## Fixes shipped (all verified vs Lucee 7.0.4, regression-tested)

1. **cfloop scoped index** — `tag_parser.rs` `parse_cfloop_tag` from/to branch:
   omit `var` when the index is scope-qualified (`local.i`) → emits
   `for (local.i = 1; …)`. (Lucee never emits `var` for a scoped index.)
2. **`var local.X` codegen** — `compiler.rs` `Statement::Var`: normalize the
   redundant `local.` prefix on a single-segment name to the bare key, so
   hand-written `for (var local.i = …)` also works (Lucee runs it fine; we now
   match). Covers the parser path the preprocessor fix alone didn't.
3. **argumentCollection precedence** — `lib.rs` (CallNamed inline expansion,
   `reorder_named_args_with_extras`, and the onMissingMethod arg builder):
   explicit named args now win over `argumentCollection` keys **regardless of
   call-site order**. Previously a spread that appeared AFTER the explicit arg
   clobbered it (last-wins binding). This fixed a SECOND hang surfaced once the
   cfloop fix let populate finish: Wheels' recursive
   `resource(name=part, argumentCollection=arguments)` (mapper/resources.cfc:62)
   kept its comma-list `name` and recursed unbounded → "infinite recursion:
   resource (depth 512)".

Regression test: `tests/core/test_scoped_loop_index_and_argcoll.cfm` (6 asserts,
green on RustCFML and Lucee). Gates all green: `cargo test --workspace` (JIT
76/76), CLI runner 4322/4322, wasm32 build, serve-mode cold+warm 4356/4356.

## FOLLOW-ON #1 — call-stack frame leak on exception unwind — FIXED (v0.235.0)

After #188 + the resource recursion, the full Wheels TestBox suite still hung at
99% CPU. Root cause found by instrumenting `runSpec`-entry depth: it climbed
MONOTONICALLY (6 → 15 → 24 → … → 614), ~9 frames per spec. A call-stack frame
LEAK — `execute_function_with_args` pushes a `CallFrame` (lib.rs ~2632) and pops
it on the normal-exit (epilogue ~6834) and `Return`-op (~4833) paths, but the ~30
scattered `return Err(…)`/`?` error-propagation paths do NOT pop. Every
`expect().toThrow()` (thousands across the suite) unwinds an exception across
frames, leaking one frame per level. Inflated depth made every spec slower
(O(depth) stack work) and would eventually trip the depth-2500 guard.

**Fix:** rename the body to `execute_function_body` and wrap it in a thin
`execute_function_with_args` that snapshots `call_stack`/`try_stack` depth before
the call and `truncate()`s back to it after — reclaiming any leaked frame on ANY
exit path (robust to `?` and every `return Err`). Verified: `runSpec`-entry
depth now bounded at **6** (was 614+). Gates green (workspace/JIT 76/76, runner
4322, wasm, serve cold+warm 4356). No regressions.

## STILL OPEN — closure loop-var reset (4th bug): suite stalls on ONE spec

With the frame leak fixed the suite PROGRESSES (depth flat at 6, ~73–140 specs/s
≈ 2–4× Lucee — normal interpreter overhead, NOT a regression) but never finishes:
it **stalls on a single spec that becomes an infinite loop**.

**Which spec:** `wheels.tests.specs.view.assetsSpec` →
`Tests that assetDomain / returns same domain for asset` (assetsSpec.cfc:48-56).
Found by logging `#SPECNAME` at runSpec entry (last logged = the spinner).

**samply** (`samply record -s -o profile.json.gz -n -- <cargo-built rustcfml> …`;
NB: do NOT wrap with `/usr/bin/env` — system-signed binaries can't be profiled):
100% of samples in that spec's loop → `$assetDomain` → `$get`. **Memory:** RSS
climbs ~90 KB/s during the spin (each leaked iteration allocates) — bounded only
by the request timeout.

**Root cause:** the spec body is
```cfml
iEnd = 100
for (i = 1; i lte iEnd; i++) { expect(e).toBe(_controller.$assetDomain(assetPath)) }
```
On RustCFML the loop var `i` **resets to 1 on every iteration** (logged
`i=1` before AND after the call, guard climbing) → infinite. `i`/`iEnd` are
unscoped in an arrow-closure. Minimal repro (saved in scratchpad
`repro_assetdomain/`): RustCFML RUNAWAY, **Lucee 7 → i=11 (ok)**. Two factors,
removing EITHER fixes it:
  1. The looped var (`obj`/`_controller`) is set by a SIBLING closure
     (beforeEach) → shared captured scope, unscoped.
  2. The body closure is invoked INDIRECTLY via the around-each
     (`arguments.spec.body(arguments.spec.data)` — a struct-stored closure call),
     not directly. A direct `body()` call does NOT reset `i`.
A method call inside the loop then resets the closure's `i` (and `count`).

**Engine area:** the closure captured-scope merge/writeback in
`cfml-vm/src/lib.rs` (~4154 merged_scope build, ~4206 writeback-into-caller-locals,
~6947 closure_parent_writeback diff — same machinery as the #187 leak). The exact
line that drops `i` needs runtime instrumentation of the closure's locals across
the inner method call. Distinct from #188 / the frame leak; its own fix.

(Secondary perf note, not the blocker: model INSTANCE creation re-runs full mixin
integration every time — worth caching per-class — but that's slowness, not the
hang.)

## Repro for the suite
Webroot `/Users/alexskinner/Repos/opensource/CFMLs/wheels/public`; note the
`directory=` URL param is IGNORED by `runner.cfm` (it always runs all of
`wheels.tests.specs`). `WHEELS_CI=true RUSTCFML_JIT=0 rustcfml --serve public
--port 8599`; hit `index.cfm/wheels/core/tests?db=sqlite&reload=true`.
Lucee 7.0.4 for comparison runs on CommandBox port 8585 (webroot = RustCFML root).
