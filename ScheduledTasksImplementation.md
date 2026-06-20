# Scheduled Tasks (`cfschedule`) — Implementation Review & Plan

Status: **Design / not started.** This document reviews how Lucee and BoxLang
implement scheduled tasks, maps the RustCFML infrastructure we can reuse, and
proposes a phased implementation. We have no Administrator UI, so configuration
is driven by the `cfschedule` tag/BIF at runtime plus `.cfconfig.json`
declarative registration (our cfconfig-as-application-config model).

---

## 1. Goal & scope

Support `cfschedule` / `schedule()` so that a CFML app can register a named task
that fires a target on an interval (once / every-N-seconds / daily / weekly /
monthly, plus cron), within an optional start/end window, surviving across
requests in serve mode.

Two non-negotiable architectural facts drive everything below:

1. **Scheduled tasks must live on `ServerState`** (cross-request), not
   per-request. This is the exact trap the JIT fell into (per-request
   `JitEngine` did nothing). A per-request scheduler is a no-op.
2. **CLI vs serve mode have different semantics.** A one-shot CLI process exits;
   long-lived periodic tasks only make sense in `--serve` mode. CLI mode should
   support `action="run"` (fire now, synchronously) and registration, but a
   registered periodic task simply won't fire after the process exits.

---

## 2. Reference: how Lucee does it

Primary sources: [Lucee `<cfschedule>`](https://docs.lucee.org/reference/tags/schedule.html),
[cfdocs](https://cfdocs.org/cfschedule),
[`ScheduledTaskThread.java`](https://github.com/lucee/Lucee/blob/master/core/src/main/java/lucee/runtime/schedule/ScheduledTaskThread.java),
[CFConfig task docs](https://cfconfig.ortusbooks.com/using-the-cli/command-overview/manage-scheduled-tasks),
[Quartz recipe](https://docs.lucee.org/recipes/scheduler-quartz.html).

### 2.1 Tag surface (classic scheduler)

`action` ∈ `update` (create-or-update), `run`, `delete`, `list`, `pause`,
`resume`. `task` is the identity key.

| Attribute | Type | Default | Notes |
|---|---|---|---|
| `action` | string | — | required |
| `task` | string | — | required (all but `list`) |
| `operation` | string | `HTTPRequest` | only HTTPRequest supported |
| `url` | string | — | required on `update`; the page requested |
| `startDate` / `startTime` | date / time | — | required on `update`; `startTime` stored as seconds-of-day |
| `endDate` / `endTime` | date / time | — | optional stop window |
| `interval` | string | `3600` | `Once`, `Daily`, `Weekly`, `Monthly`, **or N seconds (min 10)** |
| `paused` | boolean | `false` | create in paused state |
| `hidden` / `readonly` | boolean | `false` | admin-UI flags (irrelevant to us) |
| `autoDelete` | boolean | `false` | remove task once no future run possible |
| `unique` | boolean | inconsistent across sources — verify | if true, skip a fire while the prior run is still alive |
| `publish` + `file` + `path` | bool + strings | `false` | save the HTTP response to a file |
| `resolveURL` | boolean | `false` | rewrite relative links in saved output |
| `username` / `password` | string | — | HTTP basic auth on the target |
| `port` | numeric | `80` | target port |
| `requestTimeOut` | numeric (sec) | server default | per-task timeout |
| `proxyServer`/`proxyPort`/`proxyUser`/`proxyPassword` | — | — | proxy for the fire |
| `userAgent` | string | — | added Lucee 6.0.0.172 |
| `result` / `returnVariable` | string | — | only with `action="list"` |

**ColdFusion-only (NOT classic Lucee):** `group`, `priority`, `retryCount`,
`eventHandler`, `onException`, `cronTime`, `cluster`, `mode`. Lucee's modern
analogue for these is the Quartz extension (below). We should **not** try to
match the ACF Enterprise surface in v1.

### 2.2 Execution model (classic)

- **Fires an HTTP request to `url`** — it does *not* include a template or call a
  CFC in-process. Basic auth / proxy / port / timeout / resolveURL all apply to
  that HTTP call.
- Each task owns a long-lived `ScheduledTaskThread`: loop → compute next fire →
  sleep until then → spawn a separate `ExecutionThread` to do the HTTP work
  (scheduling isolated from execution).
- Interval kinds internally: `ONCE`, `EVEREY` (numeric N-seconds), `DAY`,
  `WEEK`, `MONTH`. `calculateNextExecution()` advances a `Calendar` by the unit,
  clamps to the start/end window, applies a ~1s buffer to avoid immediate
  re-fire.
- `Once` past its time → marks itself invalid; with `autoDelete` it's removed.
- `unique=true` + prior run still alive → **skip** this fire. Non-unique tasks
  may overlap (it reaps dead execution threads).
- **No retry / no `onException` in classic Lucee** — exceptions are logged; next
  fire proceeds on schedule.

### 2.3 Persistence & config

- **Lucee 5 and earlier:** XML at
  `<lucee-context>/WEB-INF/lucee/scheduler/scheduler.xml`, **web-context-scoped**
  (not server-level). Each `<task>` element mirrors the tag; numeric intervals
  stored as seconds (`interval="86400"`), dates as `{d '...'}` / times as
  `{t '...'}`.
- **Lucee 6:** config moved to JSON (`.CFConfig.json`).
- **Lucee 7:** classic scheduler extracted into the optional **"Scheduler
  Classic" extension** (bundled in full distro, absent in `-light`).
- **CFConfig (Ortus):** dedicated `task` namespace, defaults to `luceeWeb`
  because tasks are web-scoped. The exact `.CFConfig.json` key name/casing for
  classic tasks is **unconfirmed** (placeholder `scheduledTasks`); confirm with
  `cfconfig task list --JSON` against a real server before mirroring it.

### 2.4 Modern Lucee: Quartz extension

JSON-configured, built on Quartz. Adds **Component jobs** (run a CFC with an
`execute()` method, `transient`/`singleton`), **cron expressions**
(`"0 0 9-17 ? * MON-FRI"`), and cluster run-once coordination. This is the
closest Lucee equivalent to ACF's `eventHandler`/`cronTime`/`cluster`. Useful as
a direction-of-travel reference; **not** a v1 target.

---

## 3. Reference: how BoxLang does it

Primary sources: [Scheduled Tasks](https://boxlang.ortusbooks.com/boxlang-framework/asynchronous-programming/scheduled-tasks),
[Executors](https://boxlang.ortusbooks.com/boxlang-framework/asynchronous-programming/executors),
[boxlang.json (dev)](https://github.com/ortus-boxlang/BoxLang/blob/development/src/main/resources/config/boxlang.json),
[Ortus RC3 blog](https://www.ortussolutions.com/blog/boxlang-100-rc3-has-landed),
[Raymond Camden](https://www.raymondcamden.com/2025/04/04/scheduling-code-in-boxlang).

BoxLang does **not** center on a tag. Its native, first-class mechanism is a
**Scheduler class + fluent `ScheduledTask` DSL**, in core (not a module), backed
by a Java `ScheduledExecutorService` pool (the `scheduled-tasks` executor).
`bx-compat-cfml` does **not** implement `cfschedule`. (A `tasksFile`
=`${boxlang-home}/config/tasks.json` is referenced as persisting "`bx:schedule`
tasks," implying some tag/persistence facility, but it is undocumented — treat as
uncertain.)

### 3.1 Fluent DSL highlights

- **Frequency:** `every(period, unit)`, `everySecond/Minute/Hour`,
  `everyHourAt(min)`, `everyDay`, `everyDayAt("HH:mm")`, `everyWeek`,
  `everyWeekOn(day, time)`, `everyMonth`, `everyMonthOn(day, time)`,
  `everyYear[On]`, `onWeekends/onWeekdays(time)`, `onMondays…onSundays(time)`,
  `onFirst/LastBusinessDayOfTheMonth(time)`, `cron(expr)` (5-field Unix or
  6-field Quartz), `spacedDelay(delay, unit)`.
- **Constraints:** `when(closure)`, `between(start,end)`,
  `startOnTime/endOnTime`, `startOn/endOn(date[,time])`, `delay(n,unit)`,
  `withNoOverlaps()`.
- **Lifecycle (per task):** `call(closure|obj[,method])`, `before`, `after`,
  `onSuccess`, `onFailure`. Plus `enable/disable`, `run(force)`, `getStats()`.
- **Scheduler-level hooks:** `configure()`, `onStartup`, `onShutdown`,
  `beforeAnyTask`, `afterAnyTask`, `onAnyTaskSuccess`, `onAnyTaskError`.

### 3.2 Registration & config

- Global via `boxlang.json` → `scheduler.schedulers` (array of class paths),
  per-app via `this.schedulers` in `Application.bx`, CLI via
  `boxlang schedule MyScheduler.bx`, or programmatic `schedulerStart(...)`.
- Management BIFs: `schedulerNew`, `schedulerStart`, `schedulerGet[All]`,
  `schedulerList`, `schedulerShutdown`, `schedulerRestart`, `schedulerStats`.
- In-memory by default; optional cache-backed server-fixation for clustering.

### 3.3 Takeaway for us

BoxLang's model is the modern, code-first design and is where the ecosystem is
heading — closures + lifecycle hooks + cron. But it's a large surface. The
pragmatic plan: **ship the `cfschedule` tag/BIF for Lucee/ACF compatibility
first** (this is what existing apps and Preside/Wheels use), then layer a
BoxLang-style fluent Scheduler later if demand exists. Crucially, we should make
the v1 execution engine able to run **either an HTTP fire (Lucee classic) or an
in-process CFC/closure call**, because the in-process path is what a future
BoxLang-style API needs and it avoids the HTTP round-trip for local tasks.

---

## 4. Existing RustCFML infrastructure (what we reuse)

Mapped from the codebase. Line numbers approximate; grep the named symbols.

| Capability | Where | Reuse |
|---|---|---|
| **cfthread** lowering | `crates/cfml-compiler/src/tag_parser.rs:~1734` → `__cfthread_run/_join/_terminate` | Pattern for VM-intercepting a tag |
| cfthread VM exec, `ThreadSeed`/`ThreadHandle` | `crates/cfml-vm/src/lib.rs:~1125` | **Background VM execution off the request thread** — clones bytecode + scopes, std::thread w/ 64MB stack, cancel flags |
| `_schedule()` (one-shot delayed) | `crates/cfml-vm/src/lib.rs:~11058` | Relay-thread sleep→spawn→forward; cooperative cancel (50ms poll). v1 = one-shot only |
| `FutureNative` async handle | `crates/cfml-vm/src/async_kernel.rs:~43` | `.get/.isDone/.cancel`; `Arc<RwLock>` survives request boundary |
| **`ServerState`** | `crates/cfml-vm/src/lib.rs:~624` | Holds app scopes, sessions, named locks, bytecode cache. **Where the task registry lives.** |
| Session reaper background task | `crates/cli/src/lib.rs:~1199` | **The template to copy**: `tokio::spawn` + `tokio::time::sleep` at server startup, drains/queues work |
| Shared tokio runtime | DB pools (MSSQL/PG) | Reuse for the scheduler loop |
| Axum request → template exec | `crates/cli/src/lib.rs:~1381` | How an HTTP request maps to template execution (for the "fire a URL" path) |
| cfconfig schema | `crates/cfml-config/src/schema.rs` | No `scheduledTasks` section yet — wire one in |

### Key constraints surfaced by the explore pass

1. **The VM runs inside `spawn_blocking`** — the tokio async context isn't
   directly reachable from CFML code. The session reaper shows the workaround:
   the async background task runs *outside* VM context and either (a) does the
   work itself via a fresh VM execution, or (b) queues work for a request to
   drain. For scheduled tasks we want (a) — fire independently of any request.
2. **In-process invocation needs a synthesized scope.** A task firing outside an
   HTTP request has no natural `cgi`/`url`/`form`. For the **HTTP-fire path** we
   make a real loopback HTTP request (cleanest, matches Lucee exactly). For a
   future **CFC/closure path** we build a minimal request-like scope, reusing the
   `ThreadSeed` machinery cfthread already uses.
3. **Periodic scheduling needs an outer loop** — `_schedule()` is one-shot.
   Periodic = a background task that recomputes next-fire and respawns.
4. **No persistence across restart today** — purely in-memory. Persistence comes
   from `.cfconfig.json` (declarative) + an optional writable task store.

**No blockers found.** Everything builds on proven patterns (session reaper +
cfthread + ServerState).

---

## 5. Crate recommendation

cfschedule's `interval` is mostly **named periods + N-seconds**, not raw cron —
so we do not strictly need a cron engine for Lucee-classic parity. But cron is
where both Lucee (Quartz) and BoxLang have gone, so plan for it.

**Recommendation: build a lightweight scheduler loop on the existing tokio
runtime, using the [`cron`](https://crates.io/crates/cron) crate only for cron
expression parsing / next-occurrence math.** Do **not** pull in
`tokio-cron-scheduler`.

Rationale:
- `tokio-cron-scheduler`'s job model is "register an async closure"; our jobs are
  **blocking VM executions** (or loopback HTTP). We'd be fighting its model and
  adding a dependency for a loop we already have a proven template for (the
  session reaper).
- The named-period / N-second next-fire math (§2.2) is ~50 lines and we need
  full control over the start/end window clamping and `unique`/overlap behavior
  anyway.
- `cron` (the crate) is small, well-maintained, gives us `Schedule::from_str` +
  `.upcoming(tz)` for the cron path when we add it. Pairs with `chrono` which is
  already in the tree.

Alternative if we later want many tasks with minimal bespoke code:
`tokio-cron-scheduler` — revisit only if the hand-rolled loop becomes a
maintenance burden.

---

## 6. Proposed architecture

### 6.1 Tag → script lowering

In `tag_parser.rs`, lower `<cfschedule action="..." ...>` (and the script
statement form `schedule action="..." ;` / `cfschedule(...)`) to a VM-intercepted
builtin call `__cfschedule(attributesStruct)`, exactly as cfthread/cfquery tags
are lowered. Boolean/expression attrs must go through the
`format_attr_value`/expression path (see the PR #124 boolean-attr lesson) so
`#dynamicSecure#`-style values evaluate.

Register a stub `__cfschedule` in `builtins.rs`; add it to the intercept list in
`lib.rs call_function()`; implement the handler there (needs `&mut self` +
`ServerState`).

### 6.2 The registry on `ServerState`

```rust
// on ServerState
scheduled_tasks: Arc<RwLock<IndexMap<String /*lowercased name*/, ScheduledTask>>>,
scheduler_tx: Option<...>,   // control channel to the background loop
```

```rust
struct ScheduledTask {
    name: String,
    target: TaskTarget,            // Url{..} | Component{..} (future) | Closure (future)
    schedule: TaskSchedule,        // Once | EverySeconds(u64) | Daily | Weekly | Monthly | Cron(String)
    start: Option<DateTime>, end: Option<DateTime>,
    start_time_secs: Option<u32>, end_time_secs: Option<u32>,
    unique: bool, paused: bool, auto_delete: bool,
    publish: Option<PublishSpec>,  // file/path/resolveURL
    timeout_secs: Option<u64>,
    auth: Option<(String,String)>, proxy: Option<ProxySpec>, port: Option<u16>, user_agent: Option<String>,
    // runtime state
    next_fire: Option<DateTime>, running: Arc<AtomicBool>, last_result: ...,
    stats: TaskStats,              // totalRuns/success/failures/lastRun (BoxLang-style, cheap to add)
}
```

### 6.3 The background scheduler loop (serve mode)

At server startup (next to the session reaper in `crates/cli/src/lib.rs`),
`tokio::spawn` a single scheduler driver:

```
loop {
    let now = now();
    let due = registry.read().filter(|t| !t.paused && t.next_fire <= now && in_window(t,now));
    for task in due {
        if task.unique && task.running.load() { recompute_next(task); continue; }
        task.running.store(true);
        spawn_fire(task);            // tokio::spawn → spawn_blocking VM exec OR loopback HTTP
        recompute_next(task);        // advance next_fire by interval, clamp to window
        if task.schedule == Once || past_end(task) {
            if task.auto_delete { remove(task) } else { mark_invalid(task) }
        }
    }
    sleep(min(time_to_next_fire, 1s)).await;   // wake on the soonest next_fire, cap at 1s for responsiveness to add/delete
}
```

A control channel lets `__cfschedule` actions (update/delete/pause/resume) poke
the loop to recompute immediately rather than waiting out the sleep.

### 6.4 Firing a task

**v1 — HTTP fire (Lucee parity):** Make a loopback HTTP GET to `url` using the
engine's existing HTTP client (the one behind cfhttp), honoring auth/proxy/
port/timeout/userAgent. If `publish`, write the response body to `path/file`
(apply `resolveURL` if set). This is the simplest correct semantics and matches
Lucee exactly, including "task hits a real URL on this server."

**v1.5 — in-process (optional, enables BoxLang-style later):** If `url` resolves
to a local template, or a future `component=`/closure target is given, run it
via a fresh VM execution reusing `ThreadSeed` (cfthread machinery), with a
synthesized minimal request scope. Avoids the HTTP round-trip.

Each fire records into `stats` and logs to a scheduler log (mirror BoxLang's
`scheduler.log`); uncaught exceptions are logged, not retried (Lucee-classic
behavior). `unique`/overlap handled via the `running` flag.

### 6.5 CLI mode semantics

- `action="run"` → fire synchronously, now (works in CLI).
- `action="update"/"delete"/"pause"/"resume"/"list"` → mutate the registry;
  registration succeeds but periodic tasks won't fire after the process exits
  (document this clearly — same as Lucee CLI/one-shot reality).
- Declarative `.cfconfig.json` tasks are loaded into the registry at startup in
  both modes; they only *fire* under `--serve`.

### 6.6 Persistence & cfconfig

We have no Administrator, so two registration channels:

1. **Runtime:** the `cfschedule` tag/BIF (creates/updates entries in the
   in-memory registry; serve mode fires them).
2. **Declarative:** a `scheduledTasks` array in `.cfconfig.json`, loaded at
   startup into the registry. This is our equivalent of Lucee's persisted
   `scheduler.xml`. Wire into `crates/cfml-config/src/schema.rs` (per-folder
   auto-discovery + `--cfconfig` baseline already exist — see
   `project_cfconfig_scoping`).

Proposed `.cfconfig.json` shape (Lucee-attribute-aligned; we own the schema since
Lucee's exact key is unconfirmed):

```json
{
  "scheduledTasks": [
    {
      "task": "nightly-import",
      "url": "http://localhost:8500/scheduled/import.cfm",
      "interval": "Daily",
      "startDate": "2026-01-01", "startTime": "02:00",
      "endDate": "", "endTime": "",
      "unique": true, "paused": false, "autoDelete": false,
      "requestTimeOut": 300,
      "publish": false, "file": "", "path": "", "resolveURL": false,
      "username": "", "password": "",
      "port": 8500
    }
  ]
}
```

(Optionally also accept a `cron` field and a future `component` field.) Whether
to *write back* runtime-created tasks to a file is a v2 decision — start with
read-only declarative + in-memory runtime tasks.

---

## 7. Implementation phases

**Phase 1 — Tag/BIF plumbing + registry (no firing).**
- Lower `<cfschedule>` + script forms in `tag_parser.rs` → `__cfschedule(struct)`.
- Stub in `builtins.rs`; intercept + handler in `lib.rs`.
- `ScheduledTask`/`TaskSchedule` types; registry on `ServerState`.
- Implement `action` = update/delete/pause/resume/list/run (run = synchronous
  HTTP fire). Next-fire math for Once/N-seconds/Daily/Weekly/Monthly with
  start/end window clamp.
- Tests: register, list, delete, pause/resume, next-fire computation (pure-fn
  unit tests in Rust + CFML behavioral tests).

**Phase 2 — Background scheduler loop (serve mode).**
- Spawn the driver loop at server startup beside the session reaper; control
  channel for poke-on-mutate.
- HTTP-fire path via existing HTTP client; auth/proxy/port/timeout/userAgent.
- `unique`/overlap via `running` flag; `autoDelete` on spent Once/past-end.
- `publish` → write response to file (+ `resolveURL`).
- Scheduler log + per-task `stats`.
- Tests: serve-mode integration — register a short-interval task hitting a local
  echo endpoint (reuse `tests/tags/http_statements_target.cfm`), assert it fired
  N times in a window; assert `unique` prevents overlap; assert pause stops it.

**Phase 3 — Declarative cfconfig + cron.**
- `scheduledTasks` in the config schema; load at startup.
- Add `cron` interval via the `cron` crate.
- Cross-engine: verify the tag forms parse/run on Lucee 7 (Scheduler Classic
  extension) via the standard `box server start` harness; guard engine-specific
  bits with `isRustCFML()`.

**Phase 4 (optional, later) — in-process targets + BoxLang fluent API.**
- `component=`/closure targets via `ThreadSeed`; minimal synthesized scope.
- Consider a `schedulerNew`/`schedulerStart` BIF surface + fluent `task()` DSL.

---

## 8. Testing

- **Rust unit tests:** next-fire computation across all interval kinds + window
  clamping + DST edge (use fixed input times; no `Date::now`).
- **CFML behavioral (`tests/tags/test_cfschedule*.cfm`):** registration, list
  shape, run-now, delete, pause/resume. Gate periodic-firing subtests on
  serve-mode availability (discover port from `cgi.server_port`, skip under
  CLI) — same pattern as `test_tags_cfscript_statements.cfm`.
- **Serve-mode integration:** the verification gate requires serve-mode cold+warm
  green. A short-interval task hitting the local echo target, asserting fire
  count, is the key new coverage.
- **Cross-engine:** run the suite against Lucee 7 (Scheduler Classic ext
  installed). Pin lucee@7 (`feedback_lucee_pin_7_not_be`).
- Add `cargo test --workspace` coverage and the wasm32 build check (the registry
  types touch shared `CfmlValue` only via the struct payload — verify wasm).

---

## 9. Open decisions (need a call before/while building)

1. **HTTP-fire vs in-process for v1.** Recommendation: HTTP-fire only in v1
   (exact Lucee parity, simplest), in-process deferred to Phase 4.
2. **Write-back of runtime-created tasks to a file.** Recommendation: no in v1 —
   declarative `.cfconfig.json` is read-only-at-startup; runtime tasks are
   in-memory only. Revisit if users expect persistence of admin-style tasks.
3. **`.cfconfig.json` key name/casing.** We own it (`scheduledTasks`), but should
   we *also* accept Lucee's actual key for import compat? Defer until we've
   confirmed Lucee's real key via `cfconfig task list --JSON`.
4. **`unique` default.** Sources disagree (false vs true). Pick `false` (Lucee
   tag-doc default) and document it.
5. **Clustering / shared-session interplay.** Out of scope for v1; note that with
   shared sessions / multi-node a task would fire on every node (Lucee Quartz
   solves this with cache coordination — future work).

---

## 10. References

Lucee: [cfschedule tag](https://docs.lucee.org/reference/tags/schedule.html) ·
[ScheduledTaskThread.java](https://github.com/lucee/Lucee/blob/master/core/src/main/java/lucee/runtime/schedule/ScheduledTaskThread.java) ·
[CFConfig tasks](https://cfconfig.ortusbooks.com/using-the-cli/command-overview/manage-scheduled-tasks) ·
[Quartz recipe](https://docs.lucee.org/recipes/scheduler-quartz.html) ·
[Lucee 7](https://docs.lucee.org/guides/lucee-7.html)

BoxLang: [Scheduled Tasks](https://boxlang.ortusbooks.com/boxlang-framework/asynchronous-programming/scheduled-tasks) ·
[Executors](https://boxlang.ortusbooks.com/boxlang-framework/asynchronous-programming/executors) ·
[boxlang.json](https://github.com/ortus-boxlang/BoxLang/blob/development/src/main/resources/config/boxlang.json) ·
[Raymond Camden walkthrough](https://www.raymondcamden.com/2025/04/04/scheduling-code-in-boxlang)

Crate: [`cron`](https://crates.io/crates/cron) (parsing) · alternative
[`tokio-cron-scheduler`](https://crates.io/crates/tokio-cron-scheduler) (not
recommended for v1).
