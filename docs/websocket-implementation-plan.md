# RustCFML WebSocket / Realtime — Implementation Plan

Actionable companion to [`websocket-design.md`](websocket-design.md). Design rationale and the cross-ecosystem principle catalog live there; this doc is the build order, file map, wire spec, and acceptance gates.

**Locked-in decisions** (see design doc *Decisions*): (1) fluent core **+** socket.io-lucee compat layer; (2) wires = socket.io (`socketioxide`) **+** raw WebSocket, **no Pusher**; (3) **cluster-ready from day one** (`Broker` seam, node-qualified ids, message-id wire); (4) this is design-only — no engine code until the plan is approved.

---

## File map

| Where | What |
|---|---|
| `crates/cfml-vm/src/websocket.rs` *(new)* | `WebSocketRegistry`, `Broker` trait + `LocalAdapter`, `ConnId`, `ConnEntry`, `WireEnvelope`, `SocketHandle`/`ServerEmitter` (`CfmlNative`). Pure data + trait-bounded senders — **no axum/tokio types**; `#[cfg(target_arch="wasm32")]` no-op impls so `cfml-worker`/`rustcfml-wasm` still build. |
| `crates/cfml-vm/src/lib.rs` | Add `websocket: Arc<WebSocketRegistry>` to `ServerState` (next to `sessions`/`named_locks`). VM intercepts for `wsPublish`/`io`/socket methods. A `dispatch_ws_event` helper reusing `call_lifecycle_method` (`lib.rs:18719`). Channel-CFC discovery reusing the `app_cfc_path_cache` pattern + `getMetadata`/`__funcmeta` reflection (`lib.rs:9781`). |
| `crates/cfml-stdlib/src/builtins.rs` | Register stubs (return-error) for `wsPublish`, `io`, `wsSubscribe`/`wsUnsubscribe`, `wsPresence`, and the test BIFs; real handlers are VM-intercepted in `lib.rs`. |
| `crates/cli/src/lib.rs` | axum `GET /ws/:channel` → `WebSocketUpgrade` (Phase 1); `socketioxide` layer on `/socket.io/` (Phase 3). Per-connection driver task: bounded outbound `mpsc`→sink pump, inbound→`spawn_blocking` dispatch, engine auto-ping, auto-cleanup on drop. |
| `crates/cli/Cargo.toml` | `bytes` (binary frames, Phase 1); `socketioxide` (Phase 3). |
| `crates/cfml-compiler` | **Expected: no work.** Verify the parser already stores `socket=`/`encoding=` (component attrs) and `on=`/`secured=` (function attrs) — it does for the analogous `taffy:uri`/`access=`/`skip` cases (`parser.rs:3490-3552`, `:3649-3943`). Add a test if a gap surfaces. |
| `tests/websocket/*.cfm` + `crates/cli/tests/websocket_*.rs` | CFML test-harness suite (via `wsTest*`/`assertBroadcast`) + Rust integration tests (`tokio-tungstenite` raw client; socket.io test client in Phase 3). |
| `examples/websocket_chat/` | Demo chat app (SocketBox/Preside parity) for manual + Playwright verification. |

---

## Wire envelope (cluster-safe from day one)

Raw-WS frames are JSON; the socket.io transport maps the same fields onto Engine.IO/Socket.IO packets. Designed once so ids never have to change when the distributed `Broker` switches on.

```jsonc
{
  "t":  "msg|ack|join|leave|presence|err|ping|pong",  // frame type
  "ch": "/chat",            // channel (handler + auth boundary)
  "ev": "message",          // event name (routes to on="message" / onMessage)
  "d":  { },                // payload (auto-(de)serialized when encoding="json")
  "id": "n3:01J...ULID",    // node-qualified, monotonic per channel → resumability + routing
  "ref":"01J...",           // ack correlation (set when a reply is expected)
  "ex": ["n1:01J..."]       // optional exclude list (self-echo control)
}
```

`ConnId` = `{nodeId}:{ulid}` so "send to connection X" routes to the owning node; `id` gives gap-free reconnect (client sends `lastEventId` at connect).

---

## Phase 1 — raw-WS core + cluster-ready foundation

Covers the Lucee-extension and Ortus-SocketBox raw-WS styles.

**Build:**
1. `WebSocketRegistry` behind the `Broker` trait; `LocalAdapter` delivers to local `tx` senders. `ConnId` node-qualified; `WireEnvelope` with `id`/`ref`/`ex`. Put it on `ServerState`.
2. `SocketHandle` (`CfmlNative`): `id`, live `data` struct, `session`, `send(text)`, `emit(event,data)`, `broadcast(event,data)`, `to(room)`→`ServerEmitter`, `join`/`leave`/`rooms`, `close(code,reason)`. `ServerEmitter` backs `io(channel)` and the flat `wsPublish(channel,event,data,except=)` BIF.
3. axum `GET /ws/:channel` upgrade; per-connection tokio task with bounded outbound `mpsc` (drop/close on overflow), engine ping/pong (never dispatched to CFML), auto-cleanup (leave all rooms + fire `onDisconnect`) on drop.
4. Channel-CFC discovery (`/websockets/*.cfc` + `component socket="…"`), bytecode-cached. Dispatch via `spawn_blocking` → fresh VM → `dispatch_ws_event` (`call_lifecycle_method`) with live application scope + session attached from CFID.
5. Lifecycle: `onConnect(socket)` **reject gate** (return `false`/throw rejects; may return rooms to auto-join), `onMessage(socket,msg)`, `onDisconnect(socket,reason)`, `onError(socket,err)`. **Ack-by-return** (non-null handler return → client ack). Text **+ binary** frames; `encoding="json"` → `onMessage` gets a parsed struct.
6. CFML test harness: `wsTestConnect()/wsTestSend()` + `assertBroadcast(channel,event,predicate)` via a broadcast-BIF VM-intercept (mirror `writeOutput`/`cfthread`).

**Acceptance:** Rust integration test connects with `tokio-tungstenite`, sends, receives an echo and a broadcast to a second client; `onConnect` rejection closes the handshake; disconnect auto-removes from rooms; a `.cfm` page calling `wsPublish` reaches a connected client; CFML `assertBroadcast` suite green in `tests/runner.cfm`.

## Phase 2 — rooms + distributed Broker + presence + auth

Covers emit-from-anywhere at multi-node scale.

- `join/leave/to`, `on="event"` annotation dispatch (`__funcmeta` lookup), `socket.broadcast` = sugar for "others in my rooms".
- **Distributed `Broker`** over the existing shared-session cluster (memberlist + Memcached/Automerge): publishes broadcasts + replicates room membership/presence across nodes. socketioxide's Redis adapter wired as an alternative later.
- Presence: `socket.track(meta)` / `io(channel).presence()`; `presence_state` snapshot + `presence_diff` deltas; cluster-correct via the Broker.
- Authorization: `canJoin(socket,room)` hook; method/component annotation `secured="role"` checked via `getMetadata` before dispatch. Clients never self-join.
- Resumability replay: per-message `id` already in the wire (Phase 1); add `lastEventId` handling + best-effort in-memory history, opt-in `{history=N}`.

**Acceptance:** two server instances share room fan-out (broadcast on node A reaches a client on node B); presence roster + join/leave diffs correct across nodes; unauthorized `canJoin` rejected; reconnect with `lastEventId` replays missed messages.

## Phase 3 — socket.io transport + socket.io-lucee compat

Covers Preside / socket.io-lucee.

- Add `socketioxide` as a tower layer on the existing axum router (`/socket.io/`): Engine.IO handshake, namespaces, acks, binary, polling↔ws fallback. One connect handler adapts `SocketRef` → `SocketHandle`/registry — fluent CFML API unchanged.
- **Compat layer**: emulate the socket.io-lucee CFML surface (`new SocketIoServer()`, `io.on("connect",fn)`, `socket.on/emit/send`, `socket.joinRoom/leaveRoom/leaveAllRooms`, `ns.emit(rooms=…)`, `socket.broadcast(rooms=[…])`) over the same registry/`Broker`. Preside's own `isWebUser()`/`getWebsiteLoggedInUserId()` stay in the extension, reading the attached session.

**Acceptance:** a stock socket.io JS client connects, joins a room, emits/receives, and gets an ack; a minimal `preside-ext-socket-io`-style handler runs against the compat layer unchanged.

## Phase 4 — declarative conveniences

Wheels-model / domain-event auto-broadcast (`component broadcast=true` / model mixin on save·delete), whisper/client-events relayed by the hub (kept out of the CFML request path), optional `/topic`·`/user` naming convention.

## Phase 5 *(future)*

Cloudflare Durable Objects + WebSocket Hibernation — separate design; steal only auto-response ping/pong + persisted-attachment recovery.

---

## Verification gates (every phase, per `CLAUDE.md`)

- `cargo test --workspace` (Rust + JIT integration) green; new `crates/cli/tests/websocket_*.rs` green.
- `cargo run -- tests/runner.cfm` (CLI **and** serve cold+warm) green incl. the new `tests/websocket/*.cfm`.
- `cargo build --target wasm32-unknown-unknown -p cfml-worker -p rustcfml-wasm` green — **the one to watch**: the registry-on-`ServerState` change touches shared types; the `Broker`/sender traits must have `wasm32` no-op impls.
- `wasm-pack build crates/wasm --target web` green before pushing to `main`.
- A red **or** skipped test in any suite is a release blocker — `git bisect` to root cause, never "flaky".

## Implementation risks / watch-items

- **Shared-type churn on `ServerState`** → wasm build breakage. Gate the registry behind traits; keep `cfml-vm/src/websocket.rs` axum/tokio-free.
- **JIT lease per dispatch** — each `spawn_blocking` message borrows/returns the `JitLease` like a normal request; long-lived connections must not hold it between messages.
- **Write-safety** — per-connection serialized dispatch + single outbound `mpsc` consumer; never expose a frame receive-loop to CFML (engine owns it).
- **`socketioxide` footguns** (from the survey): one handler per event name (replaces silently); `Vec<u8>` serializes as a JSON number array unless `bytes::Bytes` — handle in the adapter.
- **Case-insensitivity** — lowercase-normalize channel/room/event keys (consistent with the engine's `IndexMap` scope-key policy).

---

## As-built status (v0.299.0 — Phase 1 shipped) & Phase 2 handoff

> This section records what actually landed vs the plan above, and the exact
> entry points for Phase 2. Written so a fresh session can continue without
> prior chat context. **Where this differs from the plan above, this section is
> authoritative** (the plan was written pre-implementation).

### What shipped (Phase 1, v0.299.0)

| Area | As-built location |
|---|---|
| Registry, rooms, wire, NativeObjects | `crates/cfml-vm/src/websocket.rs` — `WebSocketRegistry`, `WireEnvelope`, `FrameSink` trait, `SocketHandle`, `ServerEmitter`. Single `parking_lot::RwLock<Inner>`; `NativeObject` returns wrap `std::sync::RwLock`. |
| Registry on server | `ServerState.websocket: Arc<WebSocketRegistry>` (`lib.rs`, node_id = a uuid minted in `with_config`). |
| Emit BIFs + dispatch | `lib.rs` `call_function`, in the `name_lower` special-case block right after `writeOutput`: `io`, `wsPublish`, `assertBroadcast`, no-op `wsSubscribe/wsUnsubscribe/wsPresence`. Named args via the `pending_ws_named` stash (set in the CallNamed path next to `pending_dump_named`). `current_ws_channel` makes `io()` no-arg resolve to the channel under dispatch. `ws_test_log` backs `assertBroadcast`. `ws_arg()` helper resolves named-or-positional. |
| Lifecycle dispatch | `lib.rs` `dispatch_ws_event(channel, cfc_path, method, args)` — `resolve_component_template` + `resolve_inheritance` (+ `attach_native_parent`), then calls the lifecycle method **preserving the return value** (ack-by-return / `onConnect` reject gate). Fresh instance per message. |
| Builtin stubs | `cfml-stdlib/src/builtins.rs` `fn_ws_stub` (registered so names resolve; real behaviour is VM-intercepted). `fn_serialize_json`/`fn_deserialize_json` made `pub` for the driver. |
| axum upgrade + driver | `crates/cli/src/websocket.rs` — `GET /ws/{channel}` (route added in `lib.rs` router), per-connection bounded-mpsc outbound pump (binary frames for raw `send`), inbound→`spawn_blocking` `run_dispatch`, `encoding="json"` parse, handshake-param capture, `onError` on throw, auto-cleanup on drop. cli deps: axum `ws` feature, `bytes`, `futures-util`; dev `tokio-tungstenite`. |
| Tests / example | `crates/cli/tests/websocket_raw.rs` (5 live tests + `tests/fixtures/ws_app/`), `tests/websocket/test_ws_harness.cfm` (in `tests/runner.cfm`), `examples/websocket_chat/`, `docs/websockets.md`. |

**Already working that the plan lists under Phase 2:** rooms + `join`/`leave`/`rooms`/`socket.to(room)` + `io().to(room).except(id).emit()` + the self-id room. So Phase 2 is really: `on="event"` routing, distributed Broker, presence, auth, replay.

### Phase 2 progress — `on="event"` routing + ack `ref` (landed post-v0.299.0, NOT yet tagged)

- **`on="event"` annotation routing** shipped. `VM::resolve_ws_handler(struct, method, event)` (`lib.rs`, just above `dispatch_ws_event`) scans the instantiated channel CFC's `__funcmeta_<name>` structs for one whose metadata `on` equals the inbound event (case-insensitive) and dispatches there; missing/blank event falls back to `onMessage`. `dispatch_ws_event` gained an `event: Option<&str>` arg (only external caller is the driver). No parser/compiler changes — `on="say"` already parses into `func.metadata` → `__funcmeta_say`.
- **Driver** (`cli/src/websocket.rs`): `route_inbound(parsed)` splits a JSON channel's inbound object into `(event, payload, ack-ref)` — an object with a non-empty `ev` routes to the `on=` handler with `d` as the payload and its `id` echoed back as the ack's `ref`; everything else keeps the old whole-value-→-`onMessage` path. `dispatch`/`run_dispatch`/`handle_message_result`/`deliver_ack` thread the event + reply-ref through.
- **Tests:** `event_routing_and_ack_ref` in `crates/cli/tests/websocket_raw.rs` (6 live tests now) + `handleSay` `on="say"` handler in the `ws_app/echo.cfc` fixture. Asserts on= dispatch, `d`-payload unwrap, ack `ref` correlation, and the no-`ev` fallthrough.
- **Docs:** `docs/websockets.md` gained an "Event routing" section + `ref` in the wire/ack docs; roadmap updated.
- **Gates:** all green — `cargo test --workspace` (0 fail), `cargo run -- tests/runner.cfm` (4779/4779), serve cold+warm (4829/4829), wasm32 build, `wasm-pack build crates/wasm --target web`.

**Presence (P11)** shipped (committed as part of v0.300.0 / a follow-up). Phoenix-style track/list + state-snapshot + join/leave diffs, **channel-scoped, single-node** (cluster-correct for free once the distributed Broker lands — no API change):
- Registry (`websocket.rs`): `Inner.presence: HashMap<(channel,key), BTreeMap<ConnId, meta>>` + `ConnEntry.presence_keys`. Methods `track(conn,key,meta)` (emits `presence_state` snapshot to self, `presence_diff` join to others), `untrack(conn,key)`, `presence_state(channel) -> {key:{metas:[…]}}`, `presence_diff_frame(...)`. `unregister` now auto-untracks + broadcasts leave diffs (locks released before broadcast — parking_lot RwLock is non-reentrant). Frame type `t="presence"`, `ev="presence_state"|"presence_diff"`.
- CFML surface: `socket.track(meta)` (key=conn id) / `socket.track(key,meta)` (group tabs) / `socket.untrack([key])`; `io([channel]).presence()`; `wsPresence([channel])` flat BIF (was a no-op stub) — all return `{key:{metas:[…]}}`.
- Tests: `presence_state_diffs_and_roster` (websocket_raw.rs, now 7 live tests) + `presence_track_snapshot_and_leave_on_disconnect` unit test (websocket.rs, 5 unit tests) + `presence.cfc` fixture. Docs: `docs/websockets.md` "Presence" section + socket/io tables + roadmap.

### Deviation from the plan (decided, not an oversight)

- **No literal `Broker` trait object yet.** Decision 3's *substance* shipped — node-qualified `ConnId` (`{nodeId}:{uuid}`), monotonic message-id wire, and **all** fan-out funnels through `WebSocketRegistry::{emit_to,broadcast,to_room}`. A `Broker` trait with only a single-node `LocalAdapter` would be empty ceremony; introduce it **in Phase 2** when the distributed adapter gives it a second implementation. `FrameSink` already abstracts per-connection delivery, so the cross-node seam is: add remote routing inside those three registry methods (local id → local `sink.send`; remote nodeId → broker publish).

### Phase 2 entry points (verified)

- **`on="event"` annotation routing** — ✅ DONE (see "Phase 2 progress" above).
- **Distributed Broker** — extend the three registry fan-out methods to publish/subscribe across the existing shared-session cluster (memberlist + Memcached/Automerge, feature-gated in `crates/cli`); replicate room membership + presence. `socketioxide` Redis adapter is an alternative for the Phase-3 transport.
- **Presence** — ✅ DONE single-node (see "Presence (P11)" above). Cluster-correctness comes with the distributed Broker.
- **Auth** — `canJoin(socket, room)` hook gating `join`; `secured="role"` method/component annotation checked via the same `__funcmeta`/`getMetadata` reflection before dispatch.
- **Resumability replay** — wire already carries per-message `id`; accept `lastEventId` at the handshake (query param, already captured) + best-effort in-memory history, opt-in `{history=N}`.

### Verification gates (unchanged, all green at v0.299.0)

`cargo test --workspace` · `cargo run -- tests/runner.cfm` (CLI) · serve cold+warm · `cargo build -p cfml-worker -p rustcfml-wasm --target wasm32-unknown-unknown` · `wasm-pack build crates/wasm --target web`. **Known-flaky:** `stdlib/test_cfhttp.cfm` hits httpbin.org — its env failure flips the suite count 585↔586; not a regression.
