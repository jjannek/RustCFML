# RustCFML Observability, Instrumentation & Debugging — Design Notes

> Status: **design proposal / research synthesis** (2026-06-23). Nothing here is
> implemented yet. This document maps the territory, surveys how BoxLang and
> FusionReactor do it, and proposes a layered design that gives us
> FusionReactor-class power with OpenTelemetry-based, production-safe
> instrumentation. It deliberately separates three things that get conflated.

## 0. Three capabilities, not one

"Debugging support" actually means three separate systems with very different
designs and risk profiles. Designing them as one blob is the main trap.

| Capability | What it answers | Audience | Risk in prod |
|---|---|---|---|
| **A. Interactive debugger** | "Pause here, show me the variables, step." | Dev (and occasionally a guarded prod session) | High if it pauses traffic |
| **B. Observability / APM** | "Which request was slow, where did the time go, how often does it error?" | Ops + dev, always-on | Must be ~0 overhead |
| **C. Classic CF debug output** | "Append per-template/per-query timing to the page." | Dev, dev-only pages | Medium (leaks internals) |

A, B and C share **one foundation** (an internal hook/event bus) but ship as
independent layers. Build the foundation once; the rest are subscribers.

---

## 1. The architectural thesis: we own the loop

This is the single most important fact for the whole design.

BoxLang compiles CFML **to real JVM bytecode**. That's why its tooling looks the
way it does:

- Its debugger (`ortus-boxlang/BoxLang` → `src/main/java/ortus/boxlang/debugger/`)
  is a **DAP server that drives the JVM via JDI** (`com.sun.jdi.*`). It sets JVM
  breakpoints, walks JVM stack frames, then maps the compiled Java class+line
  back to the original `.bx`/`.cfc` source via embedded `SourceMap`s
  (`DebugAdapter.getSourceMapFromJavaLocation()`). Init strategies cover inline,
  web-server, and attach-to-running-JVM.
- Its production APM story is **FusionReactor**, a `-javaagent` that does
  **bytecode weaving** (`java.lang.instrument`) to wrap JDBC drivers, the servlet
  API and CF engine internals with no code changes.
- Its **Event Snapshot** needs a **native (JNI) Debug library** just to read Java
  locals off a live stack frame, because the JVM doesn't expose them by
  reflection.

**RustCFML has none of that machinery — and needs none of it.** We are a
stack-based bytecode VM we wrote. We can:

- read the current source line at any instruction for free (`self.current_line` /
  `self.current_column`, updated by every `BytecodeOp::LineInfo` at
  `cfml-vm/src/lib.rs:7218`);
- read the live CFML call stack directly (`self.call_stack: Vec<CallFrame>`,
  `cfml-vm/src/lib.rs:799`; `CallFrame` carries `function_name`, `template`,
  `line`, `caller_line` at `:1275`);
- already build full stack traces on every error (`build_stack_trace()` at
  `:2058`, called by `wrap_error()` at `:2245`);
- already time every query (`Instant` at `:10415` / elapsed at `:10496`, stored
  on the Query object).

So **no bytecode weaving, no JDI, no native locals reader, no source-map
round-trip.** The portable lesson from BoxLang is its **`InterceptorService`
event bus** (pre/post hooks around functions, queries, requests, BIFs, HTTP) —
that's the language-independent piece and exactly what we should model. The
lesson from FusionReactor is its **sampling profiler** and its **single-thread,
auto-resuming debugger** — both of which are *easier* for us than for a JVM
agent because we control dispatch.

---

## 2. What BoxLang and FusionReactor actually do (condensed)

### BoxLang (today)
- **Debugger:** full DAP (breakpoints, exception breakpoints, step in/out/over,
  call stack, scopes, variable inspect + edit) via JDI + source maps. Shipped as
  the `bx-debugger` module + `vscode-boxlang` extension.
- **Instrumentation:** `InterceptorService`/`InterceptorPool` with a `BoxEvent`
  enum of interception points. Relevant pairs: `onRequestStart`/`onRequestEnd`,
  **`preFunctionInvoke`/`postFunctionInvoke`** (+`onFunctionException`),
  `preTemplateInvoke`/`postTemplateInvoke`, `onBIFInvocation`/`postBIFInvocation`,
  `preQueryExecute`/`postQueryExecute` (+ `onTransaction*`),
  `onHTTPRequest`/`onHTTPResponse`, `logMessage`, app/session lifecycle.
  Handlers in Java *or* BoxLang; pre/post + mutable struct = stash start-tick,
  compute elapsed. This is the model.
- **Built-in observability:** **no** classic CF debug footer in core; web debug
  panels are add-ons (`cbdebugger`, the `bx-debug` POC, both built *on* the
  interceptor bus). Deep first-party profiling is **`bx-mcp`** — **paid**.
- **OTel:** **no BoxLang-native OTLP exporter.** `cbotel` does W3C context
  *propagation* only. FusionReactor supports BoxLang (v12.1+); FR→OTLP export is
  roadmap (Q2 2026).

### FusionReactor (the power to match)
- **Transaction tree:** auto-typed transactions (Web Request → JDBC → CFHTTP →
  custom) in a parent/child relationship tree; "Slowest" (over threshold, the
  curated list) vs "Longest" (just sorted); per-query `EXPLAIN` plan.
- **★ Production Profiler (the killer feature):** a **threshold-gated sampling
  profiler**. Default: any transaction running > **3s** starts being profiled;
  it then **samples the call stack every ~200ms** (not per-method
  instrumentation), with a sample cap. Overhead is ~constant regardless of how
  much code runs, and the fast majority of requests are never profiled. Presented
  as an inverted call tree with self-time %.
- **★ Production debugger safety model:** breakpoints with a trigger/handler
  split (conditional gate vs action); **only the triggering thread is
  suspended**, never the whole app; **fire-count** auto-disable; **max paused
  threads** cap; **auto-resume timeout** so a forgotten breakpoint can't hang
  prod. Event Snapshot bounds capture (depth 5, ≤500 vars, first 5 collection
  elements).
- **Why it's prod-viable:** in-process agent + its own web UI; sampling over
  instrumentation; threshold-gating everywhere; thread-scoped + time-bounded
  debugging; bounded state capture.

We can reproduce **all** of the profiler and debugger behaviour natively, and
get the transaction tree for free from OTel spans (§4).

---

## 3. Layer 0 — the hook bus (the one shared foundation)

A thin, trait-object event bus dispatched from the VM at a fixed set of points.
Every other layer (B, C, profiler, debugger) is a subscriber. When no subscriber
is registered, dispatch is a single branch on an `Option`/atomic → effectively
free, and the whole thing is behind a Cargo feature so non-serve / wasm builds
pay nothing.

### Hook points (all anchors already found in the codebase)

| Hook | Site | Carries |
|---|---|---|
| `request_start` / `request_end` | `cli/src/lib.rs:1473` `handle_request` (+ `compile_and_run_with_session`) | method, route, url, status, duration |
| `function_enter` / `function_exit` | `cfml-vm/src/lib.rs:7409` `call_function`; method dispatch `:13625` `call_member_function` | name, kind (udf/method/closure), depth, duration |
| `template_enter` / `template_exit` | include / component-render dispatch | template path, duration |
| `query_execute` | `cfml-vm/src/lib.rs:10348` `queryexecute` intercept (already timed at `:10415`/`:10496`) | sql, datasource, rowcount, duration |
| `transaction_*` | `cfml-vm/src/lib.rs:10664` `__cftransaction_start` (+ commit/rollback) | depth, datasource, outcome |
| `bif_invoke` | builtin dispatch in `lib.rs`/`builtins.rs` | name (metrics-only — see §4) |
| `error` | `cfml-vm/src/lib.rs:2245` `wrap_error` | type, message, stack trace, line |
| `on_error_handled` | `cfml-vm/src/lib.rs:2200` `invoke_onerror` | handled vs re-thrown |
| `cflog` | `cfml-vm/src/lib.rs:10798` `__cflog` | text, type, file |

### Where it lives
- A `Vec<Arc<dyn VmObserver>>` (or a single composed observer) on **`ServerState`**
  (`cfml-vm/src/lib.rs:671`) so it persists across requests in serve mode and is
  reachable from every request via `AppState.server_state` (`cli/src/lib.rs:868`).
- Per-request accumulation uses the existing **`request_scope`**
  (`cfml-vm/src/lib.rs:839`) — a live `Arc<RwLock<IndexMap>>` that already
  survives the whole request including includes. The debug-footer layer reads it
  back from CFML; the OTel layer reads it in Rust.

### Cost discipline
- `function_enter/exit` is the hot one. It must be: *if no observer wants
  function events, do nothing.* Implement as an atomic `enabled` flag checked
  before borrowing the observer list; subscribers declare an interest mask so the
  VM can skip whole categories. Mirrors `tracing`'s callsite interest cache.

Expose a CFML-facing API over the same bus (BoxLang's `boxRegisterInterceptor` /
FusionReactor's FRAPI analogue): `registerInterceptor(component, events)`,
`announce(event, data)`, and a `transaction(name)` helper so app code can open
custom spans/metrics.

---

## 4. Layer B — OpenTelemetry observability (the "live, no degradation" answer)

This is the direct answer to *"how could OpenTelemetry let us debug in live
environments without runtime degradation."* The design is a **three-tier signal
model**; the degradation answer is that each tier is cheap-when-healthy and the
expensive decisions are pushed off-host.

### Crate stack (feature-gated, host-only)
```
tracing (instrument macros)                ← we annotate the hook bus, not the execute loop
  → tracing-subscriber Registry + OpenTelemetryLayer
  → opentelemetry_sdk (ParentBased(TraceIdRatioBased) sampler + BatchSpanProcessor)
  → opentelemetry-otlp (gRPC) → local OTel Collector → backend (Tempo/Jaeger/…)
```
Pin versions together (`opentelemetry` ↔ `tracing-opentelemetry` ↔
`opentelemetry-otlp`; current line ~0.32 / 0.33). `tracing` 0.1.x is the mature
standard — that's fine. For Axum, `axum-tracing-opentelemetry`'s
`OtelAxumLayer`/`OtelInResponseLayer` extracts inbound W3C `traceparent`, opens
the root SERVER span, and sets HTTP semantic-convention attributes.

### Tier 1 — always-on RED metrics (every request, ~free)
Pre-aggregated in-process (an atomic add / histogram bucket bump): request
counter, error counter by `error.type`, duration histogram (exponential
buckets), per-route. Plus DB-query count/duration and BIF-call counters fed from
the hook bus. Enable **TraceBased exemplars** so a p99 latency bucket carries the
`trace_id` of a sampled trace → click from "p99 spiked" straight into a real
trace. This is the always-true SLO/alert signal and costs effectively nothing.

### Tier 2 — sampled traces (the transaction tree, for free)
Map the hook bus to spans — **this reproduces FusionReactor's transaction tree**
as a standard OTel trace:

| Interpreter concept | OTel span | Kind |
|---|---|---|
| HTTP request | root span (Axum layer) | SERVER |
| Controller action / named CFC method | child span — **gated by depth cap + allow-list** | INTERNAL |
| `.cfm` / view render | child span | INTERNAL |
| real `queryExecute` / `<cfquery>` | child span + `db.*` (`db.system.name`, `db.query.text`, rowcount) | CLIENT |
| Query-of-Queries / in-memory SQLite | child span + `db.*` | INTERNAL |
| outbound `cfhttp` | child span (HTTP client semconv) | CLIENT |
| `cfthread` | child span / span link, context propagated | INTERNAL |
| uncaught CFML exception | `exception` span event **+** status=Error + `error.type` | — |

**The granularity rule (load-bearing for overhead): span-per-function-call is a
non-starter.** A page render is tens of thousands of internal calls. Policy:
- **SPAN** only: the request; named user CFC methods up to a **frame-depth cap
  (~3)** and/or an allow-list; template renders; every real DB query; every
  `cfhttp`; lock waits / `cfthread` boundaries.
- **METRIC, never span:** bytecode ops, BIF invocations, loop iterations,
  sub-cap helper calls, scope lookups.
- Rule of thumb: **if it can happen >a few hundred times per request, it's a
  metric, not a span.**

Errors: only **uncaught** CFML exceptions get the `exception` event + Error
status (map `cfcatch.type`→`exception.type`, tag-context→`exception.stacktrace`).
A `try/catch`-recovered exception gets neither (optionally a counter). Hook at
`wrap_error` (`:2245`).

### Tier 3 — keep only the interesting traces (tail sampling, off-host)
SDK sampler = **`ParentBased(TraceIdRatioBased(p))`** with small `p` (honor
upstream `traceparent`, else a low deterministic ratio). The **OTel Collector**
runs `tailsamplingprocessor` with policies `latency{threshold_ms: 3000}` OR
`status_code:[ERROR]` plus a small probabilistic baseline — i.e. **exactly
FusionReactor's 3s-threshold capture, computed off the application host.** When
nothing is wrong you keep almost nothing; when a request is slow or errors you
keep its *complete* trace.

### Why this is "no degradation"
1. **Compile-time:** debug/trace instrumentation removed in release via `tracing`
   `release_max_level_info`; whole subsystem behind a Cargo feature.
2. **Sampling short-circuit:** most traces are `NotRecording` → span start is an
   atomic load + branch; attribute work guarded by `is_recording()`.
3. **Allow-list + depth cap** → bounded span count per request regardless of CFML
   call depth.
4. **Hot data is metrics** (atomic ops), not spans.
5. **Async batched export** (`BatchSpanProcessor`, thread-based — the
   production-proven variant, not the experimental async-runtime one): export
   never touches the request path; on overload it sheds *telemetry*, not traffic.
6. **The expensive "is this trace interesting" decision lives in the Collector**,
   not in the hot path.

Net when healthy: a few atomic metric updates + a tiny fraction of head-sampled
traces. When something's wrong: tail sampling guarantees the full trace was kept.

---

## 5. Layer 3 — the threshold-gated sampling profiler (FusionReactor's killer feature, natively)

Tracing tells you *which request* was slow; this tells you *which CFML code* was
hot — including the op-level / BIF-internal hot-spots we deliberately refuse to
span. This is the highest-value single feature and it's **easier for us than for
FusionReactor** because we own `call_stack`.

### Design
- A single **watchdog thread** holds a registry of in-flight requests, each
  publishing `{request_id, started_at, atomic *const VM-call-stack-snapshot}`.
- When a request's age crosses a configurable **threshold (default 3s)**, the
  watchdog begins **sampling that request's CFML call stack every ~200ms** (both
  configurable; plus a max-sample cap). A "sample" = snapshot the
  `Vec<CallFrame>` (function/template/line) — cheap, and *constant cost
  regardless of how much code runs*, exactly like FusionReactor.
- Aggregate samples into an inverted call tree with self-time %. Attach as a span
  event / dump to the request's trace, or expose at an admin endpoint.
- The fast majority of requests never cross the threshold → zero profiling cost.

### Two flavours
- **CFML-frame sampler (recommended, portable):** samples `self.call_stack`. Maps
  directly to CFML functions/templates/lines; works in serve mode; no native
  dependency; safe with the JIT (JIT'd frames still push `CallFrame`s — verify).
- **Native CPU sampler (optional, host-only):** `pprof`/`pprof-rs` (TiKV) — a
  `SIGPROF` timer at ~100Hz with a malloc-free, `try_lock` signal handler (drops
  the sample rather than block — never deadlocks the VM), emitting pprof
  protobuf + flamegraph SVG. Use **wall-clock mode** (samples blocked/sleeping
  threads too) to attack our known **allocator-contention / IO-bound serial
  `/posts`** path. Caveat: process-wide timer — filter to the VM thread in
  analysis. Grafana **Pyroscope Rust SDK** (wraps pprof-rs, route-tagged) is the
  continuous-profiling path. **Both are host-only — never compiled into the wasm
  worker.**

---

## 6. Layer A — the interactive / production step debugger (DAP)

We implement DAP ourselves (no JDI to lean on), but we skip BoxLang's hardest
parts: no bytecode→source map (we already have line/col on every op) and no
native locals reader (scopes are plain `IndexMap`s we own).

### Mechanism
- A **breakpoint table** keyed by (template, line) and (template, function-entry),
  consulted at `BytecodeOp::LineInfo` (`:7218`) and at `call_function`/
  `call_member_function` entry. Cost when empty: one `is_empty()` check.
- On a hit, **suspend only the executing request** (its `spawn_blocking` thread /
  task — `cli/src/lib.rs:1692`), never the server. Variable inspection reads the
  live `local`/`arguments`/`variables`/scope maps directly; the call stack is
  `self.call_stack`. Step in/out/over = resume-until-next-LineInfo /
  resume-until-depth-changes, using the depth tracking already in
  `execute_function_with_args`.
- A DAP server (TCP, like BoxLang's `BoxLangRemoteDebugger`) speaks
  `setBreakpoints`/`stackTrace`/`scopes`/`variables`/`continue`/`next`/`stepIn`/
  `stepOut`/`evaluate` to VS Code / any DAP client.

### Production-safety model (copy FusionReactor wholesale)
- **Single-request suspension** — other requests keep serving.
- **Conditional breakpoints** — a CFML expression gate evaluated in the frame.
- **Fire-count** auto-disable after N hits.
- **Max paused requests** cap.
- **Auto-resume timeout** — a paused request resumes itself after T seconds
  unless a developer has attached to it. This is what makes a forgotten
  breakpoint safe in prod.
- **Bounded capture** for the non-interactive "event snapshot" mode (depth/var/
  collection-element caps), triggered on the 2nd occurrence of an exception —
  hangs off the `error` hook.

---

## 7. Layer C — classic CF debug output (cheapest win, do first)

Subscribe the hook bus to accumulate into `request_scope`: template execution
times, the query list (sql/rowcount/duration — query timing already exists), BIF
counts, scope sizes, total time. Render a footer/panel when an app-level
`this.debug`/`showdebugoutput` flag is on (dev only; never in production). This
is the Adobe/Lucee-familiar experience, validates the hook bus end-to-end, and is
a day-or-two of work. (BoxLang doesn't even ship this in core — easy parity win.)

---

## 8. RustCFML-specific constraints (do not skip)

- **wasm32 builds (`cfml-worker`, `rustcfml-wasm`) must stay green.** OTLP/gRPC,
  `pprof`, OS timers, and background threads **do not exist on
  `wasm32-unknown-unknown`**. Every observability dependency and code path must
  be behind a Cargo feature that is **off for wasm targets**, and verified with
  `cargo build -p cfml-worker -p rustcfml-wasm --target wasm32-unknown-unknown`
  *and* `wasm-pack build crates/wasm --target web` before tagging (per CLAUDE.md
  release gate). On the worker, the observability story is W3C context
  propagation + OTLP/HTTP export only (à la `cbotel`), not in-process profiling.
- **CLI vs serve mode.** The bus, profiler watchdog and DAP server only matter in
  serve mode; the CLI path should default everything off. The debug footer (C)
  works in both.
- **JIT interaction.** Confirm JIT'd / OSR'd frames still maintain `CallFrame`
  push/pop so the sampler and debugger see them; if a hot JIT'd function elides
  frames, the sampler under-attributes it. Track explicitly.
- **Verification gate.** New hook calls in the VM hot path can regress the JIT
  admission analyser (cf. the v0.137 codegen regression that silently killed 11
  JIT tests). Any hook touching `call_function`/codegen MUST be checked with
  `cargo test --workspace` (the JIT integration suite), not just
  `tests/runner.cfm`.
- **Don't double-instrument.** Query timing already exists at `:10415`; reuse it
  rather than adding a second `Instant`.

---

## 9. Suggested phasing (value vs effort)

1. **Layer 0 hook bus** + **Layer C debug footer** — foundation + visible win,
   low risk, exercises every hook point. *(small)*
2. **Layer 3 CFML-frame sampling profiler** — highest-value APM feature, native,
   no external deps, works without a collector. *(medium)*
3. **Layer B Tier-1 RED metrics + Tier-2 traces** via `tracing`/OTLP, allow-list
   + depth-capped spans, batched export. Adds the transaction tree and the
   "no-degradation live debugging" answer. *(medium–large; feature-gated)*
4. **Tier-3 tail sampling** Collector config + docs (ops-side, not engine code).
   *(small, off-host)*
5. **Layer A DAP step debugger** with the FusionReactor safety model. *(large)*
6. Optional: native `pprof`/Pyroscope continuous profiler (host-only). *(medium)*

### The headline answer
The reason we can "debug in live environments without runtime degradation" is the
combination of: **always-on cheap aggregated metrics** + **head-sampled traces
that are near-free when not recording** + **threshold-gated sampling** (both the
OTel tail-sampler off-host and the in-process stack sampler) so the expensive
work only happens on the few requests that are actually slow or broken — plus the
fact that, because we own the interpreter, every hook is a branch we control and
can compile out entirely.
