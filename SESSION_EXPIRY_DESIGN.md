# Session Expiry — Background Reaper Design

> **Status:** proposal / design notes. NOT implemented. Uncommitted scratch.
> Supersedes the current request-driven sweep if accepted.

## 1. How it works today (v0.131.0)

There is **no timer**. Expiry is piggy-backed on request handling. In
`execute_with_lifecycle` step 8b, *every* request for an app with
`sessionManagement = true` calls:

```rust
let expired = server_state.sessions.take_expired(now_epoch_secs());
for (_, vars) in &expired {
    call_lifecycle_method("onSessionEnd", [sessionScope, applicationScope]);
}
```

Each `SessionStore` implements `take_expired` differently:

| store | `take_expired` | `onSessionEnd` | data eviction |
|---|---|---|---|
| memory | full `HashMap` scan, removes `now − last_accessed > timeout` | ✅ | by the scan |
| memcached | no-op `[]` | ❌ never | native TTL |
| cluster | scans local Automerge docs | ✅ per node | by the scan |
| datasource | `SELECT` expired + per-row `DELETE` claim | ✅ best-effort | by the delete |
| worker KV | no-op in request path | ❌ (separate scheduled handler) | native TTL |

### Problems

1. **Per-request cost.** Memory store does an O(n) scan of *all* live sessions
   on *every* request. The datasource store runs a `SELECT … WHERE expires_at
   <= now` (plus a `DELETE` per expired row) on *every* request. Cost scales
   with `request_rate × session_count`, not with the number of sessions that
   actually expire.
2. **No expiry without traffic.** A session only gets reaped — and
   `onSessionEnd` only fires — when *some* request arrives to trigger the
   sweep. On an idle server an expired session lingers and `onSessionEnd`
   never fires until the next hit. Lateness is unbounded.
3. **Read-path inaccuracy (memory store).** `MemoryStore::get` returns the
   session even when it is past `last_accessed + timeout`; only the sweep
   removes it. A request arriving after expiry but before a sweep sees a
   *live* expired session. (The datasource store does not have this bug —
   `get` filters `expires_at > now`.)

## 2. Goals

Split "accuracy" into two distinct guarantees with different mechanisms:

- **G1 — Read-path exactness (hard requirement).** A session is invisible the
  instant `now > expires_at`. `get()` must enforce this itself, independent of
  any sweep. This is what "the timeout must be accurate" means for app
  behaviour: code never sees a session that should have died.
- **G2 — `onSessionEnd` delivery timeliness (bounded best-effort).**
  `onSessionEnd` fires within a bounded lag of actual expiry, *including on an
  idle server*. Bound = the reaper tick (configurable), or exact with adaptive
  scheduling.
- **G3 — Off the request path.** A normal request pays ~zero expiry cost.

Non-goals: at-least-once `onSessionEnd` delivery (still best-effort, documented);
client-scope expiry.

## 3. Proposed architecture

### 3.1 Read-path exactness (G1) — do this regardless

Make every store's `get()` treat an expired record as absent:

- **memory:** in `get`, if `now − last_accessed > timeout` return `None` (and
  opportunistically remove). Cheap, single-key.
- **datasource:** already correct (`expires_at > ?` in the `SELECT`).
- **memcached / KV:** native TTL already guarantees this.
- **cluster:** check the doc's expiry in `get`.

This alone fixes correctness and is independent of the reaper. It should land
even if we keep a sweep.

### 3.2 Background reaper task (G2 + G3)

One `tokio` task per server, spawned in `async_run_server`, holding an
`Arc<ServerState>`. It wakes on a tick and drains expired sessions, firing
`onSessionEnd` per drained session. The per-request sweep (step 8b) is
**removed**.

```
async fn session_reaper(state: Arc<ServerState>, cfg: ReaperCfg) {
    loop {
        sleep(cfg.next_delay()).await;          // fixed tick, or adaptive
        let expired = state.sessions.take_expired(now());   // already grouped per store
        for (app_name, cfid, vars) in expired {
            run_on_session_end(&state, &app_name, vars).await;
        }
    }
}
```

#### Tick strategy — accuracy vs simplicity

- **Fixed interval (default, e.g. 60s).** Max `onSessionEnd` lateness = one
  tick. Simple, predictable. A 30–60s tick is plenty for a lifecycle hook
  whose own contract is best-effort.
- **Adaptive (optional).** Track the minimum `expires_at` across live sessions;
  sleep until `min(min_expires_at − now, max_tick)`. Fires `onSessionEnd`
  within ~ms of true expiry while still capping idle wakeups at `max_tick`.
  More moving parts; only worth it if a tight `onSessionEnd` SLA is needed.

Recommendation: ship fixed-interval first (config `session.reapIntervalSecs`,
default 60), leave adaptive as a follow-up.

### 3.3 The hard part — firing `onSessionEnd` without a request

`onSessionEnd(sessionScope, applicationScope)` is **per-application** CFML that
needs the app's `Application.cfc`, its `application` scope, and its mappings —
all of which the current code has on hand only because it runs inside a live
request. The reaper has no request context, so it must reconstruct one:

1. **Know each session's application.** `take_expired` currently returns
   `(cfid, vars)`. The reaper needs `(app_name, cfid, vars)`:
   - datasource store already has the `app_name` column → return it.
   - memory/cluster stores must start recording the app: add `app_name:
     String` to `SessionData` (written at create time from
     `current_application_name`). Small, backward-compatible (serde default
     `""`).
2. **Build a synthetic execution context per app.** For each distinct
   `app_name` in the expired batch, look up `ServerState.applications[app_name]`
   to get the `Application.cfc` path/template + the live `application` scope,
   compile (via the bytecode cache), construct a minimal
   `CfmlVirtualMachine`, set `application_scope`, and call
   `call_lifecycle_method("onSessionEnd", [sessionScope, appScope])` once per
   expired session.
   - This is essentially a headless request whose only job is the hook. Reuse
     `compile_file_cached`, mappings, and the existing lifecycle plumbing.
   - Guard against re-entrancy / cost: cap the batch, run apps sequentially.
3. **`ApplicationState` already stores `name`, `variables`, `config`,
   `app_function_table`** — enough to rebuild the app scope. The Application.cfc
   path needs to be retained (store it on `ApplicationState` if not already).

This per-app context reconstruction is the bulk of the work and the main risk.
A staged fallback: if rebuilding the app context is too invasive for v1, the
reaper can still do **cleanup only** (drain expired data off the request path,
satisfying G1+G3) and fire `onSessionEnd` *opportunistically on the next
request for that app* (bounded by traffic again, but at least cleanup is timely
and cheap). Document the trade-off rather than silently dropping the hook.

## 4. Per-store behaviour after the change

| store | read exactness (G1) | reaper sweep (G2/G3) | notes |
|---|---|---|---|
| memory | `get` checks expiry | scan once per tick, not per request | O(n) per tick |
| datasource | already filters | indexed `SELECT`+`DELETE` once per tick | per-request query removed |
| cluster | `get` checks expiry | per-node scan per tick | delete-as-claim unchanged |
| memcached | native TTL | n/a (no-op) | `onSessionEnd` still not delivered |
| worker KV | native TTL | n/a (Workers cron handler) | unchanged |

## 5. Concurrency & multi-node

- Delete-as-claim stays the cross-node guard: the node whose `DELETE` removes
  the row owns the `onSessionEnd` call, so N reaper nodes don't double-fire.
- Each node runs its own reaper; ticks need not be synchronised.
- For the cluster store, a session may be reaped on whichever node holds it;
  `onSessionEnd` fires there.

## 6. Config

```jsonc
{
  "session": {
    "reapIntervalSecs": 60,     // 0 = disable background reaper
    "reapAdaptive": false,      // sleep until next expiry (capped at reapIntervalSecs)
    "reapBatchMax": 1000        // cap sessions drained per tick
  }
}
```

CLI mode (single-shot, no server loop) spawns no reaper; expiry is irrelevant
there. The per-request sweep is removed in serve mode only.

## 7. Edge cases

- **Server shutdown:** by default the reaper does *not* flush `onSessionEnd`
  for all live sessions on shutdown (that's a different semantic — matches
  Lucee, where a hard stop drops pending session-ends). Could add an optional
  graceful drain.
- **`onSessionEnd` throws:** swallow + log per session, continue the batch
  (same as today's `let _ =`).
- **App never re-requested:** with the reaper, its sessions still expire and
  (if app context is reconstructable) `onSessionEnd` still fires — the idle
  bug is fixed. With the cleanup-only fallback, data is reaped but the hook
  waits for traffic.
- **Clock:** all stores use unix-epoch seconds; sliding `last_accessed` is set
  on each request that touches the session.

## 8. Phasing

1. **Phase 1 (correctness, low-risk):** read-path expiry in `get()` for memory
   + cluster. Add `app_name` to `SessionData`. Keep the per-request sweep.
2. **Phase 2 (the lever):** spawn the reaper task with a fixed tick; remove the
   per-request sweep; reaper does cleanup + `onSessionEnd` via reconstructed
   app context (or cleanup-only fallback, documented).
3. **Phase 3 (optional polish):** adaptive scheduling; graceful shutdown drain;
   metrics (sessions reaped/tick, reap duration).

## 9. Open questions

- Is reconstructing the per-app context for `onSessionEnd` worth the
  complexity in v1, or do we ship cleanup-only + opportunistic-hook and revisit?
- Default tick: 60s reasonable? Preside/Lucee operators expect ~minutes.
- Should `reapIntervalSecs = 0` fully disable cleanup (rely on read-path
  exactness + store TTL) for memcached/KV-style deployments?
