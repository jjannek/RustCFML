# RustCFML Observability & Debugging — Implementation Plan

> Companion to [`observability-design.md`](./observability-design.md). That doc is
> the *what and why*; this is the *how, in what order, and how we know each step is
> done*. Status: **plan, not yet started** (2026-06-23).
>
> The end-state: RustCFML ships best-in-class, production-safe debugging and
> observability — a classic CF debug panel, a FusionReactor-class threshold
> sampling profiler, full OpenTelemetry tracing + metrics, and a native step
> debugger over DAP — all behind feature flags that cost nothing when off and
> never break the wasm/worker builds.

---

## How to read this plan

Each phase is self-contained and has: **Goal**, **Approach** (with code sketches
and real `file:line` anchors), **CFML-facing surface**, **Config knobs**,
**Tests**, **Verification gate**, **Definition of done (DoD)**, and a **size**
estimate (S ≈ 1–2 days, M ≈ 3–5 days, L ≈ 1–2 weeks of focused work).

Phases are ordered by dependency and value. Phase 0 is a hard prerequisite for
everything. After that the order is a recommendation, not a constraint — see
[Ordering & milestones](#ordering--milestones).

---

## Guiding principles (non-negotiable)

These are the constraints every phase must honour. If a phase can't, it doesn't ship.

1. **Zero-cost when off.** A build/run with observability disabled must show no
   measurable regression on `tests/runner.cfm`. Hot-path hooks are a branch on an
   interest bitmask; the whole subsystem is behind a Cargo feature.
2. **wasm stays green.** `cfml-worker` / `rustcfml-wasm` target
   `wasm32-unknown-unknown` and have no threads, OS timers, gRPC, or `pprof`.
   Every heavy dependency and every thread/timer code path is feature-gated
   **off** for wasm. Gate check (`cargo build -p cfml-worker -p rustcfml-wasm
   --target wasm32-unknown-unknown` **and** `wasm-pack build crates/wasm --target
   web`) is part of every phase's DoD.
3. **Sampling over instrumentation** for anything that scales with code volume.
   We never put a span or a timer on every bytecode op or BIF call. Hot, frequent
   things become **metrics** (atomic counters); only "interesting" boundaries
   become **spans**.
4. **Single-request suspension.** The debugger and any blocking operation pause
   **only the one request's thread**, never the server. Forgotten breakpoints
   auto-resume.
5. **Don't regress the JIT.** Any change to `call_function` / codegen hot paths is
   verified with `cargo test --workspace` (the JIT admission suite), not just the
   CFML runner. (The v0.137 codegen change that silently disqualified 11 JIT tests
   for ~20 releases is the cautionary tale.)
6. **Reuse what exists.** Query timing already lives at
   `cfml-vm/src/lib.rs:10415`; stack traces at `:2058`; line/col at `:7218`. Build
   on them, don't duplicate.

---

## Architecture at a glance

```
                         ┌──────────────────────────────────────────┐
                         │  Phase 0: VM hook bus (the one foundation) │
                         │  trait VmObserver + interest bitmask       │
                         │  fired at request/fn/template/query/txn/   │
                         │  error/log/line boundaries                 │
                         └──────────────────────────────────────────┘
                            ▲          ▲           ▲          ▲
            ┌───────────────┘     ┌────┘      ┌────┘     ┌────┘
   ┌────────┴────────┐  ┌─────────┴───────┐ ┌─┴────────┐ ┌┴─────────────┐
   │ Ph1 DebugFooter │  │ Ph2 Profiler    │ │ Ph3 OTel │ │ Ph5 DAP      │
   │ (classic CF     │  │ threshold stack │ │ traces + │ │ step debugger│
   │  debug panel)   │  │ sampler         │ │ metrics  │ │ (breakpoints)│
   └─────────────────┘  └─────────────────┘ └────┬─────┘ └──────────────┘
                                                  │
                                          ┌───────┴────────┐
                                          │ Ph4 Collector  │  (ops config,
                                          │ tail-sampling  │   not engine code)
                                          └────────────────┘
   Ph6 (optional): native pprof / Pyroscope continuous profiler — host-only
```

**Dependency edges:** everything depends on **Phase 0**. Phase 4 depends on
Phase 3. Phases 1, 2, 3, 5 are mutually independent once Phase 0 lands and can be
built in parallel or reordered to taste.

---

## Cross-cutting: Cargo features & configuration (define once, in Phase 0)

### Feature flags
Add to the workspace, off by default, enabled by the `cli` crate, **never** by the
wasm crates:

| Feature | Pulls in | Enables |
|---|---|---|
| `observability` | (none heavy) | The hook-bus call sites in the VM + the footer + the in-process sampling profiler. Pure-Rust, thread-only-on-host. |
| `obs-otel` | `opentelemetry`, `opentelemetry_sdk`, `opentelemetry-otlp`, `tracing-opentelemetry`, `axum-tracing-opentelemetry` | OTLP traces + metrics export. Implies `observability`. |
| `obs-dap` | `serde_json` (already present), a small DAP codec | The Debug Adapter Protocol server. Implies `observability`. |

- The **hook-bus call sites** in `cfml-vm` are `#[cfg(feature = "observability")]`.
  When the feature is off the branches don't exist → provably zero cost and wasm
  unaffected. `cli`'s default features include `observability` + `obs-otel` +
  `obs-dap`; the wasm crates depend on `cfml-vm` with `default-features = false`.
- The watchdog thread, OS timers, gRPC and `pprof` live only under
  `#[cfg(all(feature = "...", not(target_arch = "wasm32")))]` as a belt-and-braces
  guard (mirrors the existing pattern at `cli/src/lib.rs:614`).

### Configuration
Extend the existing `RustCfmlConfig` (`cfml-config`, surfaced on `ServerState` at
`cfml-vm/src/lib.rs:671`) with config, overridable by env vars and CLI flags
(precedence: CLI flag > env > `.cfconfig.json` > default), read once at server start
and stored on `ServerState`. Two top-level homes:
- **`debugging`** — the classic footer (Phase 1). Reuses **Lucee's `debugging` key**
  for drop-in `.cfconfig.json` compatibility; we also read Lucee/CFConfig's flat keys.
- **`observability`** — the new RustCFML-native subsystems (profiler, OTel, DAP).

See the [Config reference](#appendix-b-config-reference) appendix for the full key list.

---

## Phase 0 — The VM hook bus *(the one foundation)*  ·  size: **M**

### Goal
A single, near-zero-cost event bus inside the VM that every later layer subscribes
to. No subscriber registered ⇒ a predictable branch per hook and nothing else.

### Approach

**1. Interest bitmask + observer trait** — new module `cfml-vm/src/observe.rs`:

```rust
bitflags::bitflags! {
    #[derive(Clone, Copy)]
    pub struct Interest: u32 {
        const REQUEST     = 1 << 0;
        const FUNCTION    = 1 << 1;  // hot — only set when someone truly needs per-call events
        const TEMPLATE    = 1 << 2;
        const QUERY       = 1 << 3;
        const TRANSACTION = 1 << 4;
        const BIF         = 1 << 5;  // metrics only; never a span
        const ERROR       = 1 << 6;
        const LOG         = 1 << 7;
        const LINE        = 1 << 8;  // hottest — only the DAP debugger + per-line profiler set this
    }
}

/// One observer = one subscriber (footer, profiler, OTel, debugger). The VM holds
/// a single composed observer; `Composite` fans out and OR-combines interests.
pub trait VmObserver: Send + Sync {
    fn interest(&self) -> Interest;

    // Boundaries. Default no-ops so each subscriber implements only what it wants.
    fn on_request_start(&self, _r: &RequestInfo) {}
    fn on_request_end(&self, _r: &RequestInfo, _o: &RequestOutcome) {}
    fn on_fn_enter(&self, _f: &FnEnter) {}
    fn on_fn_exit(&self, _f: &FnExit) {}        // fired on normal return AND exception unwind
    fn on_template_enter(&self, _t: &TemplateEnter) {}
    fn on_template_exit(&self, _t: &TemplateExit) {}
    fn on_query(&self, _q: &QueryEvent) {}
    fn on_transaction(&self, _t: &TxnEvent) {}
    fn on_bif(&self, _name: &str) {}
    fn on_error(&self, _e: &ErrorEvent) {}
    fn on_log(&self, _l: &LogEvent) {}
    /// Returns a directive so the debugger can pause and the profiler can sample.
    fn on_line(&self, _ctx: &LineCtx) -> LineAction { LineAction::Continue }
}

pub enum LineAction { Continue, /* Phase 5 adds Pause, etc. */ }
```

**2. Wire onto the VM.** The VM gains two fields:

```rust
// cfml-vm/src/lib.rs, on CfmlVirtualMachine
#[cfg(feature = "observability")] observer: Option<std::sync::Arc<dyn VmObserver>>,
#[cfg(feature = "observability")] interest: Interest,   // cached OR of observer.interest()
```

Set from `ServerState` at VM construction. `interest` is a plain copy so the hot
path reads a local field, not through the `Arc`.

**3. Hook sites** — each guarded by an interest check so it's a bitand+branch when
nobody cares. Anchors are exact:

| Hook | Site | Notes |
|---|---|---|
| request start/end | `cli/src/lib.rs:1473` `handle_request` → wrap `compile_and_run_with_session` (`:516`/`:1692`) | root of everything |
| fn enter/exit | `cfml-vm/src/lib.rs:7409` `call_function`; `:13625` `call_member_function` | **also fire exit on the exception-unwind pop path** (the v0.235 frame-leak site) so spans/profiler stacks never leak |
| template enter/exit | include + component-render dispatch | |
| query | `cfml-vm/src/lib.rs:10348` `queryexecute` — reuse the `Instant` at `:10415`/`:10496` | carries sql/datasource/rowcount/ms |
| transaction | `cfml-vm/src/lib.rs:10664` `__cftransaction_start` (+ commit/rollback) | |
| bif | builtin dispatch in `lib.rs`/`builtins.rs` | **metrics only** — counter, no span |
| error | `cfml-vm/src/lib.rs:2245` `wrap_error` | reuses `build_stack_trace` (`:2058`) |
| on_error handled | `cfml-vm/src/lib.rs:2200` `invoke_onerror` | handled vs re-thrown |
| log | `cfml-vm/src/lib.rs:10798` `__cflog` | |
| line | `cfml-vm/src/lib.rs:7218` `BytecodeOp::LineInfo` | **only compiled-active when `interest & LINE`** — see below |

Hot-path discipline for the two hot hooks:

```rust
// LineInfo handler — the hottest. The whole block vanishes without the feature,
// and at runtime costs one bitand+branch unless a debugger/per-line profiler is attached.
#[cfg(feature = "observability")]
if self.interest.contains(Interest::LINE) {
    if let Some(obs) = &self.observer {
        match obs.on_line(&LineCtx { line, col, depth: self.call_stack.len() }) {
            LineAction::Continue => {}
        }
    }
}
```

```rust
// fn enter — guarded by FUNCTION; exit symmetric and ALSO on the unwind path.
#[cfg(feature = "observability")]
if self.interest.contains(Interest::FUNCTION) {
    if let Some(obs) = &self.observer { obs.on_fn_enter(&FnEnter { /* name, kind, depth */ }); }
}
```

**4. Enter/exit correlation.** Store an optional per-frame token on `CallFrame`
(`cfml-vm/src/lib.rs:1275`): `obs_token: Option<u64>` (and/or `started: Option<Instant>`
on host). The observer returns/records a token on enter; exit reads it from the
frame being popped. This makes exit self-contained and correct even when the VM
pops multiple frames during exception unwind.

**5. CFML-facing registration bus.** Expose a minimal interceptor API (model on
BoxLang's `boxRegisterInterceptor` / FusionReactor's FRAPI) so app code and modules
can subscribe and open custom transactions. These are VM-intercepted builtins:
`registerInterceptor(component|closure, events)`, `announce(event, data)`,
`transaction(name, closure)` (opens a custom span around a closure). A CFML
interceptor is wrapped in an adapter that implements `VmObserver` and dispatches to
the named CFC methods (`preFunctionInvoke`, `postQueryExecute`, …).

### CFML-facing surface
`registerInterceptor()`, `announce()`, `transaction()` (above). No behaviour change
unless used.

### Config knobs
`observability.enabled` (master switch; default off in CLI, off in serve unless set).

### Tests
- Rust unit tests in `observe.rs`: a `CountingObserver` asserts each hook fires the
  right number of times for a fixture script (incl. exception unwind firing
  `on_fn_exit` for each popped frame).
- CFML test `tests/observe/test_interceptors.cfm`: `registerInterceptor` sees
  `preFunctionInvoke`/`postQueryExecute`; `announce`/`transaction` round-trip. Add to
  `tests/runner.cfm`.
- A "no observer" micro-benchmark (criterion or a timed `runner.cfm` loop)
  confirming the disabled path is within noise.

### Verification gate
`cargo test --workspace` (JIT!) · `cargo run -- tests/runner.cfm` (CLI + serve) ·
wasm build + `wasm-pack` · no-observer benchmark within ±1%.

### DoD
Bus fires at all 10 sites with correct enter/exit pairing (incl. unwind); CFML
interceptor API works; feature-off build is byte-identical hot path; all gates green.

---

## Phase 1 — Classic CF debug output (footer/panel)  ·  size: **S**

### Goal
The Adobe/Lucee-familiar "debugging information" appended to a page, with a data
model faithful to **Lucee 6/7** so it feels native to CFML developers: total +
per-template times, the query list (SQL, datasource, recordcount, cached, ms, issuing
template+line), exceptions, `cftimer`/`cftrace` points, dumps, unscoped-variable
(implicit) access, and a generic-data extensibility channel. Cheapest visible win;
validates the bus end-to-end. **BoxLang ships no core footer** (its `debugMode` flag
only raises log verbosity; the de-facto BoxLang panel is the `cbdebugger` module), so
this also lands us ahead of BoxLang out of the box.

### Approach

**Standards alignment.** We follow the **Lucee / Adobe (CFConfig) lineage**, not
BoxLang — BoxLang ships no core footer to mirror (its `debugMode` flag only raises log
verbosity; the BoxLang-world panel is the `cbdebugger` module), and RustCFML already
reads `.cfconfig.json`, so Lucee's model is the least-surprising and gives Adobe-compat
on the basics for free. We mirror Lucee's two-gate enable model, its IP whitelist (the
security gate), the `<cfsetting showDebugOutput>` page override, its **`DebugData`
schema and template-CFC contract** (below), and its **seven section toggles**. The
configurable URL trigger is a **RustCFML enhancement, not a Lucee feature** — Lucee
core matches *by IP range only*; the familiar `?debug=true` is merely an application
convention (`if(url.keyExists("debug")) cfsetting(showDebugOutput=true)`), which we
promote to a first-class, configurable gate (see *Activation*).

**Collection.** A `DebugCollector` observer (`interest = REQUEST|TEMPLATE|QUERY|BIF|ERROR`)
accumulates into a per-request `DebugData` struct, stored on the VM as
`request_debug: Option<Box<DebugData>>` and allocated at `request_start` **only when
the activation check (below) passes** — so a request that won't show debug output
collects nothing and allocates nothing. Reuse the query timing already captured at
`:10415`; template times from the template enter/exit hook; total from request
start/end.

**Activation — the four-gate model (fail-closed; render only if all four pass, evaluated cheapest/most-secure first):**

1. **Enabled** — `debugging.enabled` (default `false`). When `production_mode` is
   false, localhost may be treated as enabled even if unset (dev convenience).
2. **Viewer allowed** = **(client IP ∈ `debugging.showFromIPs`)** *OR* **(URL trigger
   matches)**. This is the production-safety gate — debug output leaks SQL, scope
   contents and file paths, so it is default localhost-only and we **never** render
   for a viewer that passes neither rule. The IP whitelist applies **identically in
   production** (it is *not* tied to `production_mode`), so the first-class way to
   debug a live site is `enabled: true` with `showFromIPs` restricted to your
   office/VPN/ops addresses (optionally plus a secret URL trigger) — every other
   visitor gets a normal page and never sees the panel or any timing. Supporting
   multiple addresses/CIDR ranges in `showFromIPs` is part of this gate.
   - **URL trigger (RustCFML enhancement) — fully configurable, including the variable
     name itself**, which enables security-by-obscurity (Lucee core has no `?debug`;
     this supersets it):
     - `debugging.urlTrigger.enabled` (default `true`),
     - `debugging.urlTrigger.param` — **the URL variable name** (default `"debug"`;
       set to e.g. `"myhiddenvarname"` to hide the trigger entirely),
     - `debugging.urlTrigger.value` — the required value (default `"true"`; set to an
       unguessable secret so that `?myhiddenvarname=s3cr3t-9f2a` becomes the gate;
       empty string = presence-only, i.e. any value triggers).
     - Matching is on the resolved **`url`/`form` scope** so both `?param=value` and a
       posted field work; comparison is case-insensitive on the value unless a
       secret is configured (then exact).
     - In `production_mode`, a presence-only trigger (`value=""`) and a bare
       `?debug=true` are **refused** — production forces a non-empty secret value.
   - **Client IP resolution (reverse proxies) — `debugging.trustForwardedFor`.** By
     default the gate matches the raw socket peer (`addr.ip()` at
     `cli/src/lib.rs:1480`), which is what Adobe/Lucee do. Behind a load balancer the
     peer is always the proxy, so this knob controls header-based resolution, designed
     to be **spoof-resistant**:
     - `false` (default) — ignore forwarded headers; match the socket peer.
     - `true` — trust `X-Forwarded-For` (then `X-Real-IP`) unconditionally. Only safe
       if your edge always overwrites the header on ingress; documented as the foot-gun
       option, never the default.
     - a **list of trusted proxy IPs/CIDRs** (recommended for production) — honour the
       forwarded header **only when the socket peer is itself in this trusted set**,
       then walk `X-Forwarded-For` right-to-left skipping trusted hops to find the
       first untrusted address (the real client). A direct visitor who forges
       `X-Forwarded-For` is ignored, because *their* peer isn't a trusted proxy — so
       the whitelist can't be bypassed by a spoofed header.

     The resolved client IP is the same value OTel records as `client.address`
     (Phase 3) — implement it once as a shared `resolve_client_ip(peer, headers,
     trust_cfg)` helper and reuse it; don't fork the logic.
3. **Not suppressed by the page** — honour `<cfsetting showDebugOutput="false">`,
   which (like Adobe) can only turn it **off**, never bypass gates 1–2. Auto-suppress
   when the response Content-Type isn't HTML/text (JSON/binary/redirect) and on AJAX
   requests (`X-Requested-With`) so we never corrupt a non-page response.
4. **Renderable** — there is an `output_buffer` to append to and the body is HTML/text.

**Rendering — the Lucee template-CFC contract.** At `request_end`, if all gates pass,
render `DebugData` through a **debug-template CFC** implementing
`output(struct custom, struct debugging)` — exactly Lucee's contract (`custom` =
config options for that template; `debugging` = the runtime data struct). We ship four
built-ins matching Lucee — `modern` (default, interactive HTML panel), `classic`
(static tables), `simple` (no-JS), `comment` (HTML `<!-- -->` block for view-source) —
plus `none`. Because renderer and data are decoupled and the contract is identical,
**existing Lucee debug-template CFCs can render our data**, and users can drop their
own template CFC in a configured directory. `getDebugData()` is available whenever
gates 1–2 pass (even with `template="none"` or the footer suppressed), so a JSON/AJAX
panel or a separate debug endpoint can consume the structured data directly.

### Data model — the `DebugData` schema (Lucee 6/7-faithful)
The struct passed to the template as `debugging`. We emit Lucee 6/7's shape (not 5.x's
removed top-level `times` key — page timing lives in `pages`). Each section is a
collection of rows with fixed columns:

| Section | Columns | Source |
|---|---|---|
| `queries` | `name, time, sql, src, line, count, datasource, usage, cacheType` | `query` hook (reuse timing at `:10415`); `count`=recordcount, `cacheType`=cached indicator, `usage`=per-column used/unused (when `queryUsage` on) |
| `pages` | `id, count, min, max, avg, app, load, query, total, src` | `template` enter/exit; `app`=exec−query, `total`=load+exec |
| `pageParts` | `id, count, min, max, avg, total, path, start, end, startLine, endLine, snippet` | optional finer breakdown (cap ~100 parts) |
| `timers` | `label, time, template` | `<cftimer type="debug">` |
| `traces` | `type, category, text, template, line, action, varname, varvalue, time` | `<cftrace>` / `trace()` |
| `dumps` | `output, template, line` | `<cfdump output="debug">` / `debug()` |
| `implicitAccess` | `template, line, scope, count, name` | scope-resolution hook (see distinctive features) |
| `genericData` | `category, name, value` | `debugAdd()` (app/module-injected panels) |
| `exceptions` | array of `{type, message, detail, tagContext:[{template,line}]}` | `error` hook; reuse `build_stack_trace` (`:2058`) |
| `datasources` | per-DS `{name, openConnections, connectionLimit}` | pool stats |
| `abort` | `{template, line}` (only if aborted) | `cfabort` path |
| `starttime`, `id` | request meta | request start |

Scope dump ("Scope Variables") follows the universal contract: render **Application,
CGI, Client, Cookie, Form, Request, Server, Session, URL** — **never `variables`/`local`**.

### Distinctive Lucee features worth baking in (beyond the basic ACF footer)
- **`implicitAccess` — unscoped-variable access tracking** (`scope, name, template,
  line, count`). Surfaces scope leaks and the scope-search-chain cost; given our hot
  `get_ci`/scope-resolution path it doubles as a perf-debugging aid. Gated by the
  `implicitAccess` toggle (off by default — it adds per-access bookkeeping).
- **Query `usage` — per-column used/unused** (`queryUsage` toggle): flags columns you
  `SELECT`'d but never read.
- **`cacheType`** on queries — cached-vs-live indication.
- **`debugAdd(category, struct)` → `genericData`** — the supported channel for app code
  / frameworks (Wheels, Preside) to inject their own debug panel; pairs directly with
  the Phase 0 interceptor bus.
- **The seven section toggles** (Lucee's flags): `database, exception, tracing, timer,
  implicitAccess, queryUsage, dump` — plus `maxRecords` (≈ Lucee `debugMaxRecordsLogged`,
  default 10) so a runaway page can't balloon the buffer, and `highlightMs` (slow-row
  red threshold, the universal 250 ms default).

### CFML-facing surface
- `getDebugData()` → the `DebugData` struct above (gated by activation 1–2). Lucee
  exposes this via `getPageContext().getDebugger().getDebuggingData()`; we provide a
  direct BIF and may also mirror the pagecontext path for compat.
- `debugAdd(category, data)` → append a row to `genericData` (custom panels).
- Feed the data from the standard tags/BIFs: `<cftimer type="debug" label=…>`,
  `<cftrace>` / `trace()`, `<cfdump output="debug">` / `debug(var)`.
- `isDebugMode()` → the two-gate conjunction (enabled AND viewer-allowed), matching Lucee.
- Honour `<cfsetting showDebugOutput="true|false">` — page-level **suppress only**, matching Adobe/Lucee.
- Optionally honour `this.debug = true` in `Application.cfc` as an additional source for gate 1 — the viewer/IP gate 2 is **still** enforced, so an app turning it on can never leak to the public.

### Config knobs (Lucee-compatible `debugging` block — see Appendix B)
`debugging.enabled`, `debugging.showFromIPs`, `debugging.trustForwardedFor`,
`debugging.urlTrigger.{enabled,param,value}` (the RustCFML enhancement),
`debugging.template` (`modern`|`classic`|`simple`|`comment`|`none`, or a custom
template-CFC path), `debugging.templateDir` (where user template CFCs live),
`debugging.highlightMs` (slow-row threshold, default 250), `debugging.maxRecords`
(≈ Lucee `debugMaxRecordsLogged`, default 10), and the **seven section toggles**
`debugging.fields.{database, exception, tracing, timer, implicitAccess, queryUsage,
dump}` plus the scope-dump selection. We also read Lucee/CFConfig's flat keys
(`debuggingEnabled`, `debuggingDBEnabled`, `debuggingTemplate`, …) for drop-in
compatibility. The footer lives under Lucee's `debugging` key; the *new* subsystems
(profiler, OTel, DAP) live under `observability`.

### Tests
`tests/observe/test_debug_output.cfm` (serve-oriented): with `enabled` + localhost
allowed, run a page with two queries + an include; assert `getDebugData()` reports 2
queries with rowcounts and 1 template and timings ≥0. Plus activation tests (Rust,
driving the gate logic directly so they don't depend on a real client IP):
- IP not in `showFromIPs` and no trigger → **no footer, no collection allocated**.
- Custom `urlTrigger.param="myhiddenvarname"` + `value="s3cr3t"`:
  `?myhiddenvarname=s3cr3t` renders; `?myhiddenvarname=wrong` and `?debug=true` do not.
- `<cfsetting showDebugOutput="false">` suppresses even when gates 1–2 pass.
- Non-HTML / AJAX response → auto-suppressed.
- `production_mode` + presence-only trigger (`value=""`) → refused.
- **Proxy / spoofing** (the `resolve_client_ip` helper): with `trustForwardedFor`
  as a trusted-CIDR list, a request from a trusted proxy carrying
  `X-Forwarded-For: <whitelisted-ip>` resolves to the whitelisted client and renders;
  the **same header from an untrusted peer is ignored** (resolves to the peer) and
  does **not** render. With `trustForwardedFor:false`, forwarded headers never affect
  the decision.
- **Data-model fidelity:** a page issuing 2 queries + an include + a
  `<cftimer type="debug">` + a `<cftrace>` + an unscoped var read + a `debugAdd()`
  produces the matching `queries` / `pages` / `timers` / `traces` / `implicitAccess` /
  `genericData` sections with the documented columns; `queryUsage` on flags an unused
  selected column; `cacheType` reflects a cached query.
- **Template-CFC contract:** the same `DebugData` renders through `classic` / `modern`
  / `comment` / `none` **and** through a user-supplied template CFC implementing
  `output(custom, debugging)`.
- **Section toggles:** disabling `database` omits `queries`; `maxRecords` caps each
  section's row count; the scope dump never includes `variables`/`local`.
Cross-engine note: assert structure/counts, not exact ms.

### Verification gate
Full gate (as Phase 0). Explicitly confirm: the footer never renders for a
non-whitelisted IP without a matching trigger; a wrong/absent trigger value never
renders; collection doesn't even allocate when activation fails.

### DoD
Footer renders in serve mode through the four-gate model; emits the **Lucee 6/7
`DebugData` schema** and renders via the **`output(custom, debugging)` template-CFC
contract** (four built-in templates + user-supplied CFCs); ships `implicitAccess`,
`queryUsage`, `cacheType`, `debugAdd()`/`genericData`, the seven section toggles, and
`cftimer`/`cftrace`/dump integration; the URL trigger's **param name and value are
both configurable** (security-by-obscurity supported); `getDebugData()` / `isDebugMode()`
work; honours `<cfsetting showDebugOutput>`; off + localhost-only by default; **can be
left enabled in production scoped to specific IPs/CIDRs** (and/or a secret trigger)
with no leakage to other visitors; production refuses a keyless trigger; never renders
for a disallowed viewer; gates green.

---

## Phase 2 — Threshold-gated sampling profiler *(FusionReactor's killer feature)*  ·  size: **M**

### Goal
When a request runs longer than a threshold (default 3s), start sampling **its CFML
call stack** every ~200ms and aggregate into an inverted call tree with self-time
%. Constant cost on the slow request; **zero cost on every fast request**; the
single highest-value APM feature, and easier for us than for FusionReactor because
we own `self.call_stack`.

### Approach — cooperative self-sampling (safe, no cross-thread stack access)

The call stack lives in the VM owned by the request's `spawn_blocking` thread
(`cli/src/lib.rs:1692`). A separate watchdog thread must not read it directly. So
the watchdog **requests** a sample and the VM **provides** it at its next safe
point:

1. **Registry.** A `ProfilerHub` on `ServerState` (host-only) holds, per in-flight
   request: `{ id, started: Instant, want_sample: Arc<AtomicBool>, sink:
   Arc<Mutex<SampleAggregator>> }`. The VM clones the `Arc<AtomicBool>` into a field
   `profile_flag` at request start (one Arc clone, cheap).
2. **Watchdog thread** (single, `#[cfg(not(wasm))]`): every `tick` (e.g. 50ms) scans
   the registry; for any request whose age > `thresholdMs`, sets `want_sample` at
   the configured `intervalMs` cadence, up to `maxSamples`.
3. **VM provides the sample** at the `LineInfo` hook (interest `LINE` is set only
   while profiling is armed for *this* request):
   ```rust
   if let Some(f) = &self.profile_flag {
       if f.load(Relaxed) {            // cheap relaxed load; almost always false
           if f.swap(false, Relaxed) { self.capture_self_sample(); }
       }
   }
   ```
   `capture_self_sample()` snapshots `self.call_stack` (function/template/line per
   frame) + `current_line` into the request's `SampleAggregator`. This is the VM
   sampling **itself**, so no locking of live `IndexMap`s across threads.
4. **Aggregate & expose.** On `request_end`, fold samples into a call tree (self vs
   total time by frame signature). Attach to the request's trace as a span event
   (Phase 3) and/or expose at an admin endpoint `/__rustcfml/profiler` and via a
   `getRequestProfile()` BIF.

**Overhead control:** a fast request never has `LINE` interest set, so it pays
nothing. A slow request pays one relaxed atomic load per CFML line plus a stack
snapshot every 200ms — constant regardless of how much code runs. If even the
per-line load proves measurable, fall back to sampling at **function-entry**
boundaries only (coarser but cheaper); benchmark and decide.

**JIT caveat (must verify):** confirm JIT'd / OSR'd functions still push/pop
`CallFrame`s so the sampler sees them; a JIT path that elides frames would
under-attribute. Add a JIT-on profiler test.

### CFML-facing surface
`getRequestProfile()` → the call tree for the current request (if profiled);
`profileNow()` to force-start profiling the current request (FusionReactor's
"Profile now").

### Config knobs
`observability.profiler.enabled`, `…thresholdMs` (3000), `…intervalMs` (200),
`…maxSamples` (e.g. 500), `…watchdogTickMs` (50).

### Tests
- Rust: feed the aggregator synthetic stacks, assert the inverted tree + self-time %.
- CFML `tests/observe/test_profiler.cfm` (serve-only / skipped on CLI): a route that
  `sleep()`s past the threshold inside a known call chain; assert the profile
  contains the expected hot frame. Mark timing-tolerant.
- JIT-on profiler test (separate, `cargo test --workspace`).

### Verification gate
Full gate. Plus: a sub-threshold request shows **no** profiler allocation/flag
churn (assert `LINE` interest unset).

### DoD
Slow requests produce a usable call tree; fast requests pay nothing; `profileNow()`
works; JIT'd frames attributed correctly; gates green.

---

## Phase 3 — OpenTelemetry traces + metrics *(the "live, no-degradation" core)*  ·  size: **L**

### Goal
Always-on RED metrics + sampled distributed traces that reproduce FusionReactor's
transaction tree as standard OTel. Traces export async over OTLP; **metrics export
either by OTLP push to a collector OR via a native Prometheus `/metrics` scrape
endpoint (config-selected, or both)** so teams that already run Prometheus need no
collector. This is the direct answer to "debug in production without runtime
degradation."

### Approach

**Setup (cli, `obs-otel`, host-only).** At server start build an
`SdkTracerProvider` with `ParentBased(TraceIdRatioBased(p))` sampler + a
**thread-based `BatchSpanProcessor`** (the production-proven variant — *not* the
experimental async-runtime one) exporting OTLP/gRPC to a local collector. Build a
meter provider for metrics. Register a `tracing_subscriber` Registry with
`EnvFilter`, fmt, and the `OpenTelemetryLayer`. Pin crate versions together
(`opentelemetry` ↔ `tracing-opentelemetry` ↔ `opentelemetry-otlp`).

**Root span (HTTP).** Use `axum-tracing-opentelemetry`'s `OtelAxumLayer` /
`OtelInResponseLayer` on the router (`cli/src/lib.rs:1402`) to extract inbound W3C
`traceparent`, open the root SERVER span, set HTTP semantic-convention attributes,
and write context into the response. Span name = `{method} {http.route}` using the
**matched route template** (derive from the rewrite/controller mapping; never the
raw path — cardinality).

**VM child spans — raw OTel API, not `tracing` macros.** The VM emits spans through
an `OtelObserver: VmObserver` that holds an explicit OTel **context stack**
(per-request, on the VM thread — safe because one request = one blocking thread).
On request start it bridges the active context from the Axum root span (via
`OpenTelemetrySpanExt`) and pushes it; each `on_fn_enter`/`on_template_enter`/
`on_query` starts a child span with the top-of-stack as parent and pushes it; the
matching exit ends it and pops. Why raw API not `tracing` macros: the VM's manual
enter/exit doesn't fit `tracing`'s RAII-guard model across bytecode dispatch;
explicit start/end with an explicit parent stack is clean and correct.

**The span allow-list (load-bearing for overhead).** Per the design doc's
granularity rule — *span-per-call is a non-starter*:
- **SPAN:** the request (root); named user CFC methods **gated by frame-depth cap
  (~3) and/or an allow-list**; template renders; every real DB query (CLIENT,
  `db.*`); QoQ/in-memory SQLite (INTERNAL, `db.*`); outbound `cfhttp` (CLIENT);
  lock waits / `cfthread` boundaries.
- **METRIC, never span:** bytecode ops, BIF calls, loop iterations, sub-cap helper
  calls, scope lookups. Rule: **>a few hundred/request ⇒ metric.**

So `on_fn_enter` consults `depth <= cap && allow_listed(name)` before spanning;
`on_bif` only bumps a counter.

**Semantic conventions (emit stable names — greenfield, skip dual-emit).**
- HTTP server: `http.request.method`, `url.path`, `url.scheme`,
  `http.response.status_code`, `http.route`, `server.address`, `client.address`.
- DB client: `db.system.name`, `db.query.text` (sanitised), `db.operation.name`,
  `db.namespace`, rowcount; span name `{operation} {target}`; kind CLIENT for real
  drivers, INTERNAL for QoQ/in-memory.
- Errors (**uncaught only**, at `wrap_error` `:2245`): record an `exception` span
  **event** (`exception.type`←`cfcatch.type`, `exception.message`,
  `exception.stacktrace`←tag-context) **and** set span status = Error + `error.type`.
  A `try/catch`-recovered exception gets neither (optional counter).

**Tier-1 RED metrics (always-on, ~free).** OTel instruments on the `OtelObserver`:
request counter, error counter (by `error.type`), duration histogram (exponential
buckets) per `http.route`; DB query count + duration histogram per datasource; BIF
call counter. Enable **TraceBased exemplars** so histogram buckets carry the
`trace_id` of sampled traces → click from "p99 spiked" to a real trace.

**Metrics export — push (OTLP) or pull (Prometheus), the same instruments either way.**
The OTel metric instruments live on the `OtelObserver`; *where* they go is a
`MetricReader` choice, config-selected via `observability.otel.metrics.exporter`:
- `otlp` — a `PeriodicExportingReader` pushes metric snapshots to the collector on an
  interval (default 60s); the collector then feeds Prometheus / Mimir / Datadog / etc.
- `prometheus` — a `PrometheusExporter` reader renders the in-memory aggregation
  on-demand at a namespaced **`/__rustcfml/metrics`** endpoint that Prometheus scrapes
  directly — **no collector required**. Metrics-only: Prometheus stores no traces, so
  the *tracing* half still uses OTLP regardless.
- `both` — register both readers off the one set of instruments.

OTel's dotted instrument names (`http.server.request.duration`) auto-translate to
Prometheus convention (underscores + `_seconds`/`_total` suffixes) via the bridge.
Pull mode needs no `BatchSpanProcessor`-style queue — the aggregation is held in
memory and serialised on scrape. **Is the Prometheus endpoint *necessary*?** No — a
collector's `prometheusexporter` already turns OTLP metrics into a scrape target one
hop downstream. We offer it because it's the lowest-friction, most-deployed metrics
path and removes the collector as a *required* dependency for the common
"just give me dashboards + alerts" case. Crate caveat: the `opentelemetry-prometheus`
bridge is 0.x and has lagged the core line — either pin it carefully or back the pull
endpoint with `metrics` + `metrics-exporter-prometheus` while keeping
`opentelemetry-otlp` for the push/trace path; settle this when building.

**Why no degradation:** (1) `release_max_level_info` compiles out debug/trace; (2)
low head-sample ratio ⇒ most traces `NotRecording` ⇒ span start is an atomic
load+branch, attribute work guarded by `is_recording()`; (3) allow-list + depth cap
bound spans/request; (4) hot data is metrics; (5) `BatchSpanProcessor` exports off
the request path and sheds *telemetry* (not traffic) on overload; (6) the
"interesting trace" decision is Phase 4, off-host.

### CFML-facing surface
`transaction(name, closure)` from Phase 0 now opens a real OTel INTERNAL span. A
BIF to read the current `traceId`/`spanId` (for log correlation). Outbound `cfhttp`
auto-injects `traceparent`.

### Config knobs
`observability.otel.enabled`, `…endpoint` (OTLP), `…protocol` (`grpc`|`http`),
`…sampleRatio` (head, e.g. 0.05), `…serviceName`, `…spanDepthCap` (3),
`…spanAllowList` (component/method globs), `…export.timeoutMs`, batch knobs
(`OTEL_BSP_*` env honoured). Metrics: `…metrics.enabled`,
**`…metrics.exporter` (`otlp` | `prometheus` | `both`)**,
`…metrics.prometheus.path` (default `/__rustcfml/metrics`),
`…metrics.pushIntervalMs` (OTLP periodic reader, default 60000).

### Tests
- Rust integration test with an **in-memory span exporter**: run a fixture page,
  assert the span tree shape (root → method → query), the `db.*` attributes, the
  exception event + Error status on an uncaught throw, and that a deep call below
  the depth cap produced **no** span.
- Metrics test: assert request/error counters and the duration histogram recorded
  (OTLP in-memory reader). With `exporter:"prometheus"`, scrape `/__rustcfml/metrics`
  and assert the text exposition contains `rustcfml_http_requests_total` and a
  `rustcfml_http_request_duration_seconds` histogram with the expected labels.
- A `traceparent` propagation test: inbound header continues the trace; outbound
  `cfhttp` carries a child `traceparent`.
- **wasm gate is critical here** — confirm the OTLP/gRPC/thread deps are fully
  excluded from the wasm crates.

### Verification gate
Full gate, with extra attention to the wasm exclusion. JIT suite. Confirm
`sampleRatio=0` (everything NotRecording) is within noise of observability-off.

### DoD
Real OTLP traces with the transaction tree + correct semconv attributes reach a local
collector; **RED metrics export via both OTLP push and a native Prometheus
`/__rustcfml/metrics` scrape endpoint, selected by `metrics.exporter`**; head sampling
+ batch export verified; uncaught errors recorded, caught ones not; wasm builds
untouched; gates green.

---

## Phase 4 — Tail sampling at the Collector *(ops, not engine)*  ·  size: **S**

### Goal
Keep only the **interesting** traces — slow or errored — exactly like
FusionReactor's threshold capture, but computed **off the application host** so the
hot path stays cheap.

### Approach
No engine code. Ship a reference OTel Collector config + docs:
- `tailsamplingprocessor` with policies: `latency{threshold_ms: 3000}` OR
  `status_code:[ERROR]` OR an attribute match, plus a small `probabilistic`
  baseline. Tune `decision_wait` (~30s) and `num_traces` (watch
  `sampling_trace_dropped_too_early`).
- For multi-instance: a two-tier deployment — tier-1 `loadbalancingexporter`
  (routes by trace-id) → tier-2 tail processor — so all spans of a trace land on one
  instance. Document the single-gateway simple case as the default.
- A `docker-compose` example (Collector + Tempo/Jaeger + Grafana) under
  `examples/observability/` so a user gets the full picture locally in one command.

### Tests / DoD
Manual: run the example stack, drive slow + errored + fast requests, confirm only
slow/errored full traces are retained. Documented in `docs/observability-ops.md`.

---

## Phase 5 — Native step debugger over DAP  ·  size: **L** (largest)

### Goal
Set breakpoints, step (in/out/over), inspect and edit variables, evaluate
expressions — from VS Code or any DAP client — including a **production-safe** mode
that pauses only one request and auto-resumes. We implement DAP ourselves but skip
BoxLang's two hardest parts: **no bytecode→source map** (we have line/col on every
op) and **no native locals reader** (scopes are `IndexMap`s we own).

### Approach

**DAP server** (`obs-dap`, host-only): a TCP server thread (à la BoxLang's
`BoxLangRemoteDebugger`) speaking the DAP wire protocol (`initialize`, `launch`/
`attach`, `setBreakpoints`, `setExceptionBreakpoints`, `stackTrace`, `scopes`,
`variables`, `setVariable`, `continue`, `next`, `stepIn`, `stepOut`, `pause`,
`evaluate`, `disconnect`). It owns a shared **breakpoint table** keyed by
`(template, line)` and `(template, fn-entry)` and a registry of paused requests.

**A `DebuggerObserver: VmObserver`** with `interest = LINE | FUNCTION | ERROR`
(only attached when a client connects → normal runs never set `LINE`). At the
`LineInfo` hook (`:7218`) `on_line` consults the breakpoint table:
- **Miss:** return `Continue` (the common case — table lookup is the only cost while
  attached).
- **Hit:** evaluate the optional condition in the current frame; if it fires, the
  **VM thread parks itself** in a command loop, servicing inspect/eval/step/set
  requests *on its own thread* (so all scope access stays on the owning thread — no
  cross-thread `IndexMap` access), and only returns to bytecode on `continue`/`step`.

**State exposure while paused:** the call stack is `self.call_stack`; scopes are the
live `local`/`arguments`/`variables`/`this` maps; `evaluate` compiles+runs a small
expression against the current frame (we already have the compiler + VM on this
thread); `setVariable` writes the scope map directly. Source mapping is trivial —
`setBreakpoints` maps file+line straight to the table; stack frames already carry
line via `current_line` + `CallFrame.line`.

**Stepping** uses the call-depth tracking already in `execute_function_with_args`:
step-in = break at the next `LineInfo`; step-over = next `LineInfo` at depth ≤
current; step-out = next `LineInfo` at depth < current.

**Production-safety model (copy FusionReactor wholesale):**
- **Single-request suspension** — only the parked thread blocks; others serve.
- **Conditional breakpoints** — CFML expression gate.
- **Fire-count** — breakpoint auto-disables after N hits.
- **Max paused requests** — a counter; at cap, a new hit just `Continue`s (never
  exceed the cap, never starve the pool).
- **Auto-resume timeout** — the park is a `recv_timeout`; on timeout the request
  auto-continues *unless* a client has attached to that paused frame. This is what
  makes a forgotten breakpoint safe in production.
- **Exception breakpoints** + a bounded, non-interactive **Event Snapshot** (depth
  5 / ≤500 vars / first-5 collection elements) hung off the `error` hook, triggered
  on the 2nd occurrence of an exception type — for capture without pausing.

**Editor integration:** a thin VS Code launch config (`attach` to the DAP port).
Optionally a small extension later; DAP-over-TCP works with generic DAP clients now.

### Config knobs
`observability.debugger.enabled` (off by default; serve-only), `…port`,
`…maxPausedRequests`, `…autoResumeMs`, `…allowProduction` (must be explicitly true
to permit pausing when `production_mode` is on).

### Tests
- A scripted DAP client (Rust integration test, `obs-dap`): connect, `setBreakpoints`
  on a fixture, drive a request, assert the breakpoint hits, `stackTrace`/`scopes`/
  `variables` return expected values, `next`/`stepIn`/`stepOut` move correctly,
  `evaluate` works, `setVariable` mutates, `continue` finishes.
- Safety tests: fire-count disables; auto-resume fires after the timeout; max-paused
  cap causes pass-through; `allowProduction=false` refuses to pause in production
  mode.
- Confirm a non-attached run never sets `LINE` interest (no overhead).

### Verification gate
Full gate. JIT suite (a breakpoint inside a would-be-JIT'd function must
de-optimise or be honoured — decide and test). wasm excluded.

### DoD
VS Code (or scripted DAP client) can set breakpoints, step, inspect/edit, evaluate;
single-thread suspension + fire-count + auto-resume + max-paused all enforced;
production pausing gated behind explicit opt-in; gates green.

---

## Phase 6 — Native CPU/wall-clock profiler *(optional, host-only)*  ·  size: **M**

### Goal
Code-level (Rust + VM-internal) hot-spot profiling for the things tracing/CFML-frame
sampling can't see — bytecode dispatch, BIF internals, allocator pressure. Targets
the known **allocator-contention / IO-bound serial `/posts`** path.

### Approach
- Ad-hoc: integrate `pprof` (pprof-rs, TiKV) behind a `--profile` flag —
  `SIGPROF`/`setitimer` at ~100Hz, malloc-free `try_lock` signal handler (drops
  rather than blocks), emit pprof protobuf + flamegraph SVG. **Wall-clock mode** to
  catch blocked/IO time. Process-wide timer → filter to the VM thread in analysis.
- Continuous: Grafana **Pyroscope Rust SDK** (wraps pprof-rs, route-tagged) for
  serve mode. Call `shutdown()` on exit.
- **Host-only**: `#[cfg(all(feature="obs-pprof", not(target_arch="wasm32")))]`.
  Never in the worker.
- Future: migrate to OTel's native profiling signal when it leaves Alpha (pprof
  convertibility makes the switch cheap).

### DoD
`--profile` produces a flamegraph + pprof file for a CLI run; optional continuous
mode documented; wasm untouched.

---

## Ordering & milestones

**Recommended sequence:** `0 → 1 → 2 → 3 → 4`, with **5 (DAP)** startable in
parallel right after **0** (it only needs the `LINE`/`FUNCTION` hooks), and **6**
whenever. If *interactive debugging* is the top user ask, do `0 → 1 → 5` first and
slot 2/3 after.

| Milestone | Phases | What users get |
|---|---|---|
| **M1 — Foundations + visible win** | 0, 1 | The hook bus + a classic CF debug panel + a CFML interceptor API. Ships fast, low risk. |
| **M2 — Production profiling** | 2 | FusionReactor-class threshold sampling profiler. Highest value/effort; no external deps. |
| **M3 — Full observability** | 3, 4 | OTel traces (transaction tree) + RED metrics + tail-sampled "keep the interesting ones" — the live-debugging-without-degradation story, end to end with a local Grafana/Tempo stack. |
| **M4 — Interactive debugging** | 5 | Step debugger over DAP in VS Code, with the production-safe pause model. |
| **M5 — Deep profiling** | 6 | Native CPU/wall-clock flamegraphs + continuous profiling. |

Each milestone is independently shippable and tagged (respect the CLAUDE.md release
gate every time).

---

## Performance acceptance criteria (gate each phase)

- **Observability disabled (feature off):** 0% measurable change vs baseline on
  `tests/runner.cfm` and the wheels-perf-bench `/posts` p50.
- **Bus on, no subscriber:** ≤1% on `runner.cfm`.
- **Profiler armed, request *under* threshold:** ≤1% (must prove `LINE` interest
  unset on fast requests).
- **Profiler actively sampling a slow request:** ≤2% added to that request only.
- **OTel on, `sampleRatio` low (~5%), metrics on:** ≤3% on `/posts` p50; export must
  never appear on the request critical path (verify spans flush on the batch
  thread).
- **DAP attached, no breakpoint hit:** ≤2% (table lookup per line).

If a phase blows its budget, it doesn't ship until fixed — and we `git bisect`
rather than shrug (the v0.137 JIT lesson).

---

## Risks & open questions

1. **Per-line atomic load in the profiler** — may be measurable in hot loops.
   Mitigation/fallback: sample at function-entry granularity instead of per-line.
   *Decide via benchmark in Phase 2.*
2. **JIT vs frames** — JIT'd/OSR'd functions must still push `CallFrame`s for the
   sampler and debugger to see them. *Verify in Phase 2/5; add a JIT-on test.*
3. **DAP breakpoint inside a JIT-compiled function** — does the JIT honour the
   breakpoint table, or must we de-opt that function while debugging? *Decide in
   Phase 5.*
4. **`tracing` ↔ raw-OTel context bridging** — getting VM child spans correctly
   parented under the Axum root span across the bridge needs care. *Prototype early
   in Phase 3.*
5. **wasm dependency creep** — easy to accidentally pull an OTLP/gRPC/thread
   transitive dep into a shared crate. *The wasm build gate in every DoD is the
   guardrail; keep heavy deps in `cli` only.*
6. **Crate version churn** — the OTel Rust crates are 0.x with tight coupling. *Pin
   exact versions; treat upgrades as their own PRs.*
7. **Security / data leakage** — `db.query.text` and the debug footer can leak PII;
   spans/footer must respect sanitisation + IP whitelist + production gating by
   default. **The IP whitelist is only as trustworthy as client-IP resolution:** if
   `trustForwardedFor` blindly trusts `X-Forwarded-For`, a direct attacker can spoof a
   whitelisted IP and unlock the footer. Default `false`; the trusted-proxy-list mode
   is the safe production setting; document the `true` foot-gun loudly.

---

## Appendix A — Event taxonomy (the bus contract)

| Event | When | Key payload |
|---|---|---|
| `request_start` / `request_end` | per HTTP request (serve) | method, route, url, status, total ms |
| `fn_enter` / `fn_exit` | UDF / method / closure call + return/unwind | name, kind, depth, ms, token |
| `template_enter` / `template_exit` | include / component render | path, ms |
| `query` | `queryExecute` / `<cfquery>` | sql, datasource, system, rowcount, ms |
| `transaction` | begin / commit / rollback / savepoint | depth, datasource, outcome |
| `bif` | builtin invocation | name (counter only) |
| `error` | `wrap_error` (any raised) + uncaught flag | type, message, stack trace, line |
| `on_error_handled` | `invoke_onerror` | handled vs re-thrown |
| `log` | `cflog` | text, type, file |
| `line` | every `LineInfo` (only while LINE-interested) | line, col, depth → `LineAction` |

## Appendix B — Config reference

### `debugging` (classic footer — Lucee/CFConfig-compatible, Phase 1)

```jsonc
{
  "debugging": {
    "enabled": false,                          // Lucee: debuggingEnabled. dev-mode may default-on for localhost.
                                               // Set true + restrict showFromIPs to run live in production.
    "showFromIPs": ["127.0.0.1", "::1"],       // Adobe "Debugging IP Addresses" / Lucee IP rules — the security gate.
                                               // Honoured in production too; supports multiple IPs / CIDR ranges.
    "trustForwardedFor": false,                // reverse-proxy client-IP resolution. false = use socket peer (default);
                                               //   true = trust X-Forwarded-For/X-Real-IP (foot-gun); or a list of
                                               //   trusted proxy IPs/CIDRs (recommended) = honour the header only when
                                               //   the socket peer is a trusted proxy, then walk XFF right-to-left.
    "urlTrigger": {                            // RustCFML enhancement — Lucee core matches by IP only; ?debug is an app convention
      "enabled": true,
      "param": "debug",                         // the URL VARIABLE NAME itself — rename for obscurity, e.g. "myhiddenvarname"
      "value": "true"                           // required value; set an unguessable secret for security-by-obscurity.
                                               //   "" = presence-only (any value) — REFUSED when production_mode is on.
    },
    "template": "modern",                       // modern (default) | classic | simple | comment | none | <path to a template CFC>
    "templateDir": "",                          // optional dir of user template CFCs implementing output(custom, debugging)
    "highlightMs": 250,                         // slow-row red-highlight threshold (Adobe/Lucee/cbdebugger universal default)
    "maxRecords": 10,                           // ≈ Lucee debugMaxRecordsLogged — rolling cap per section
    "fields": {                                 // the seven Lucee section toggles
      "database": true, "exception": true, "tracing": true, "timer": true,
      "implicitAccess": false, "queryUsage": false, "dump": true,
      "scopes": ["cgi", "url", "form"]          // which scopes to dump (never variables/local)
    }
  }
}
```
Viewer is allowed if the client IP is in `showFromIPs` **OR** the URL trigger matches
(so you can scope a live site to office/VPN IPs, hand a teammate a secret
`?myhiddenvarname=…` link, or both). `<cfsetting showDebugOutput="false">` always
suppresses; non-HTML/AJAX responses auto-suppress. Flat Lucee/CFConfig keys
(`debuggingEnabled`, …) are also read for drop-in compatibility. Overridable by
`RUSTCFML_DEBUG_*` env vars and CLI flags.

### `observability` (profiler / OTel / DAP — RustCFML-native)

```jsonc
{
  "observability": {
    "enabled": false,
    "profiler": { "enabled": false, "thresholdMs": 3000, "intervalMs": 200,
                  "maxSamples": 500, "watchdogTickMs": 50 },
    "otel": {
      "enabled": false, "endpoint": "http://localhost:4317", "protocol": "grpc",
      "serviceName": "rustcfml", "sampleRatio": 0.05,
      "spanDepthCap": 3, "spanAllowList": ["*"],
      "metrics": { "enabled": true, "exporter": "otlp",      // otlp | prometheus | both
                   "prometheus": { "path": "/__rustcfml/metrics" }, "pushIntervalMs": 60000 },
      "export": { "timeoutMs": 30000 }
    },
    "debugger": { "enabled": false, "port": 9898, "maxPausedRequests": 4,
                  "autoResumeMs": 60000, "allowProduction": false }
  }
}
```
Overridable by `RUSTCFML_OBS_*` env vars and the corresponding CLI flag (precedence:
CLI > env > `.cfconfig.json` > default).

## Appendix C — New / touched files (forecast)

- **New:** `cfml-vm/src/observe.rs` (bus + traits + events), `cfml-vm/src/profiler.rs`
  (hub + aggregator), `cli/src/otel.rs` (provider/exporter setup),
  `cli/src/dap/` (server + protocol codec), `cfml-stdlib/` additions (interceptor
  BIFs), `examples/observability/` (collector + compose), `docs/observability-ops.md`.
- **Touched (hook sites):** `cfml-vm/src/lib.rs` (`call_function` :7409,
  `call_member_function` :13625, `queryexecute` :10348, `__cftransaction_start`
  :10664, `wrap_error` :2245, `invoke_onerror` :2200, `LineInfo` :7218, `__cflog`
  :10798, `CallFrame` :1275, `ServerState` :671); `cli/src/lib.rs` (`handle_request`
  :1473, router :1402, `compile_and_run_with_session` :516); `cfml-config`
  (config block); `Cargo.toml`s (features + deps, host-only).
