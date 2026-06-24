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
