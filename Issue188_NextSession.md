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

## STILL OPEN — separate issue (next Wheels blocker), UNRESOLVED

With #188 + the resource recursion fixed, the full Wheels TestBox suite still
does not complete (99% CPU, HTTP 000 at 200s). **This is NOT a simple infinite
recursion** — it never trips the depth-2500 stack guard (no "Call stack
overflow") even though depth reaches 600+, so the recursion unwinds/oscillates.

I initially guessed a closure-arguments-capture bug, but that was wrong: the
ACTUAL TestBox around-each pattern (`generateAroundEachClosuresStack` with
`thread.closures` + a BARE `closureIndex` param + `aroundStub` calling
`spec.body()`) reproduces FAITHFULLY in isolation and **works** on RustCFML and
Lucee. (Only a contrived variant using `arguments.idx` — not what TestBox uses —
fails, and is irrelevant.)

Observed recursing cycle (depth probe @600, innermost first):
`runSpec (BaseSpec.cfc:1035) → runAroundEachClosures (1260) → __closure_56 →
runSpec → …`, captured while running a QueryBuilderSpec `expect().toThrow()`.

**Leading hypothesis: extreme slowness, not a hard hang.** Model INSTANCE
creation re-runs full mixin integration every time (301K `$willBeOverriddenByMixin`
calls in the first 600K calls). With 4000+ specs each making model instances,
the suite may simply be far slower than Lucee (~60s). **Next step:** instrument
to count COMPLETED specs over time — if specs keep completing it's a perf problem
(profile model-instance mixin re-integration); if stuck on one spec, find that
spec's trigger. Separate from #188; its own session.

## Repro for the suite
Webroot `/Users/alexskinner/Repos/opensource/CFMLs/wheels/public`; note the
`directory=` URL param is IGNORED by `runner.cfm` (it always runs all of
`wheels.tests.specs`). `WHEELS_CI=true RUSTCFML_JIT=0 rustcfml --serve public
--port 8599`; hit `index.cfm/wheels/core/tests?db=sqlite&reload=true`.
Lucee 7.0.4 for comparison runs on CommandBox port 8585 (webroot = RustCFML root).
