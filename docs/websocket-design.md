# RustCFML WebSocket / Realtime — Design

> **Status:** design (no engine code yet). Companion: [`websocket-implementation-plan.md`](websocket-implementation-plan.md).
> Informed by a survey of ~12 realtime stacks outside CFML; four shaping decisions are locked in (see *Decisions* below).

---

## Context

Three reference projects do realtime in the CFML world, each shaped by how its host engine is deployed:

| Project | Protocol | CFML API shape | Why it exists |
|---|---|---|---|
| **Lucee WebSocket extension** | raw WebSocket (JSR-356) | one listener CFC per channel, `onOpen/onMessage/onClose/onError`, a `wsClient` with `send()/broadcast()/close()`, `static` scope for cross-connection state, `websocketInfo()` | servlet-container WS, shares the HTTP port |
| **Ortus SocketBox** | raw WebSocket | `WebSocketCore` base CFC, `onConnect/onMessage(message, channel)`, `sendMessage(msg, channel)`, `broadcastMessage()`; cross-instance relay (no sticky sessions) | needs CommandBox / BoxLang MiniServer's low-level HTTP listener |
| **Preside socket.io ext** → **socket.io-lucee** | **socket.io** (Engine.IO framing, namespaces, rooms, acks, polling↔ws fallback) | `io = new SocketIoServer(); io.on("connect", fn(socket))`, `socket.on/emit/send`, `socket.joinRoom/leaveRoom`, `ns.emit(rooms=…)`, `socket.broadcast(rooms=[…])`; Preside adds `socket.isWebUser()/getWebsiteLoggedInUserId()` | embeds netty-socketio on Jetty inside Lucee |

All three solve the **same hard problem**: bridge a long-lived async connection to an engine that executes synchronously. RustCFML's advantage: it **owns its axum/tokio server**, so it needs no servlet container, no embedded Jetty, no Node sidecar — the WebSocket lives natively in the same process and port as HTTP.

**Directive:** do not clone any one of these APIs — design the most fluent, elegant CFML interface, then back it with the cleanest Rust architecture.

### What the codebase already gives us (verified)

- **axum + tokio** are hard deps; `run_server` builds a multi-thread runtime; requests already use `tokio::task::spawn_blocking` to run the sync VM. axum has built-in `extract::ws::WebSocketUpgrade`. (`crates/cli/src/lib.rs`)
- **`ServerState`** (`crates/cfml-vm/src/lib.rs:672`) is the per-server `Arc` shared across all requests — already holds `applications`, `sessions`, `bytecode_cache`, `named_locks`, the session reaper. **This is where the connection registry belongs.**
- **`CfmlNative` / `NativeObject`** (`crates/cfml-common/src/dynamic.rs:633`) + `register_native_class` — the exact vehicle for exposing a live `socket` handle to CFML.
- **Lifecycle dispatch** `call_lifecycle_method` (`crates/cfml-vm/src/lib.rs:18719`) already does case-insensitive method lookup + `this`/`variables` binding + writeback — reuse verbatim for socket callbacks (same pattern as `onApplicationStart`).
- **Function-level annotations already parse and reflect**: `function chat(socket,msg) on="message" {}` is captured into `Function::metadata` and surfaced as `__funcmeta_<name>` structs + via `getMetadata().functions[].metadata` (`compiler.rs:2860`, `lib.rs:9781`). **No parser work needed for the elegant API.**
- **Async kernel** (`crates/cfml-vm/src/async_kernel.rs`) + `spawn_cfthread` (`cli/src/lib.rs:589`) show the established `FutureNative`-style NativeObject pattern — the socket handle follows it.
- **Shared-session/cluster infra already exists** (Memcached + memberlist/Automerge behind features) — the natural backbone for SocketBox-style cross-instance relay.

---

## Decisions (locked in)

1. **Both a fluent core *and* a socket.io-lucee compat layer.** The greenfield fluent API (below) is the primary surface for new apps; a compatibility shim emulates the `socket.io-lucee` CFML API (`new SocketIoServer()`, `io.on("connect", fn)`, `socket.on/emit/send`, `socket.joinRoom/leaveRoom`, `ns.emit(rooms=…)`) so the existing **preside-ext-socket-io** runs with minimal change during transition.
2. **Two wire protocols: socket.io (via `socketioxide`) + raw WebSocket.** socket.io is the primary (Preside / socket.io-lucee parity + mature client); raw WS covers the Lucee-extension and Ortus-SocketBox styles. **Pusher protocol is *not* in scope** for now (revisit only if the Laravel Echo ecosystem becomes a goal).
3. **Cluster-ready from day one.** Multi-node fan-out is a first-class constraint throughout, not a bolt-on: the registry sits behind a `Broker` abstraction, connection ids are node-qualified, and the wire format carries what cross-node routing/resumability need — so the single-node `LocalAdapter` and the distributed backend are the same seam with no later rework.
4. **Deliverable: repo docs + project memory** (`docs/websocket-design.md` + `docs/websocket-implementation-plan.md`), design-only, no engine code yet.

---

## The CFML interface (the heart of this design)

### Decision: a **channel component** with **convention lifecycle + annotation event routing**

One CFC = one channel. Lifecycle is by convention (zero ceremony, like `Application.cfc`); socket.io-style named events route via the **function attribute annotation already supported by the parser**. Most fluent option *and* cheapest to build — no new syntax.

```cfml
// /websockets/Chat.cfc      ← auto-discovered; path → channel "/ws/chat"
component socket="/chat" {

    // ── lifecycle: convention method names, all optional ──
    function onConnect( socket ) {
        socket.join( "lobby" );
        socket.emit( "welcome", { id = socket.id(), users = io().in("lobby").count() } );
        socket.broadcast( "userJoined", socket.id() );   // everyone else in channel
    }

    function onDisconnect( socket, reason ) {
        io().to( "lobby" ).emit( "userLeft", socket.id() );
    }

    // ── named events: the elegant annotation (function attribute, parses TODAY) ──
    function say( socket, message ) on="message" {
        io().to( "lobby" ).emit( "message", { from = socket.id(), text = message } );
    }

    function enter( socket, room ) on="join" {
        socket.join( room );
        socket.emit( "joined", room );
    }
}
```

**Why this is the elegant choice (alternatives considered):**

- `on="message"` is a *function attribute* — RustCFML already parses, stores, and reflects it. Reads like a route declaration, keeps the handler name free, and is Wheels/TestBox-idiomatic (`function x() skip {}`). **Zero parser changes.**
- Convention lifecycle (`onConnect/onMessage/onDisconnect/onError`) means the simplest raw-WS echo server is ~4 lines — beats Lucee's six-callback surface and SocketBox's base-class requirement.
- The **`socket` object is identical** whether raw WS or socket.io; socket.io just adds `emit(event,…)` / `on=` routing on top of raw `send()`.
- **Rejected** — line-prefix `@on("message")` (BoxLang style): prettier but needs new lexer/parser/AST work not yet in the engine. Noted as *optional future sugar* that desugars to the same `__funcmeta`, so adopting it later changes nothing downstream.
- **Rejected** — `io = new SocketIoServer(); io.on(...)` (socket.io-lucee imperative style): forces boilerplate wiring into every app, no natural home in a request/response engine. We expose `io()` as an ambient accessor instead.

### The `socket` object (NativeObject — same surface for raw WS and socket.io)

| Method | Raw WS | socket.io | Notes |
|---|---|---|---|
| `socket.id()` | ✓ | ✓ | stable per-connection id |
| `socket.send(data)` | ✓ | ✓ | raw text/binary frame to this client |
| `socket.emit(event, data)` | — | ✓ | named event to this client |
| `socket.broadcast(event, data)` | ✓ | ✓ | to everyone in channel **except** sender |
| `socket.join(room)` / `leave(room)` / `rooms()` | ✓ | ✓ | room membership |
| `socket.to(room)` | ✓ | ✓ | returns a scoped emitter → `.emit()/.send()` |
| `socket.close([reason])` | ✓ | ✓ | |
| `socket.set(key,val)` / `get(key)` | ✓ | ✓ | per-connection state (replaces Lucee's `static`) |
| `socket.data()` / `socket.param(name)` | ✓ | ✓ | handshake query params (`?userId=42`) |
| `socket.session` / `socket.isWebUser()` … | ✓ | ✓ | session attached from CFID at handshake (enables Preside-style auth helpers) |

### Emit from anywhere (scheduled task, REST endpoint, another request)

An ambient accessor, no wiring required — backed by the registry on `ServerState`:

```cfml
io( "/chat" ).to( "lobby" ).emit( "announcement", "Maintenance in 5m" );
io( "/chat" ).emit( "ping", now() );          // whole channel
io( "/chat" ).in( "lobby" ).count();          // members
io( "/chat" ).sockets();                       // connection ids
```

`io()` (no arg, inside a handler) = the current channel. Replaces Lucee's "stash `wsClients` in `static`" idiom and SocketBox's `new WebSocket().broadcastMessage()` with one obvious call.

### Config / discovery

- `this.websocket.enable = true` in `Application.cfc` (or cfconfig `websocket.enabled`).
- Channels auto-discovered from `/websockets/*.cfc` (Lucee convention) **and/or** the explicit `component socket="/chat"` attribute (wins when present).
- Client connects to `ws://host/ws/chat` (raw) or the socket.io client to `/socket.io/` with namespace `/chat`.

---

## Rust architecture (how we implement it)

### Connection lifetime & the sync/async bridge

```
axum router
 ├─ raw WS:  GET /ws/:channel        → WebSocketUpgrade            (Phase 1–2)
 └─ socket.io: socketioxide layer on /socket.io/                  (Phase 3)
        │
        on_upgrade → one tokio task per connection
        │   holds split (sink, stream) + an outbound mpsc::Sender
        │   registers ConnEntry in WebSocketRegistry (on ServerState)
        │
   inbound frame ──► tokio::task::spawn_blocking:
        build fresh VM (reuse compile_and_run path + bytecode cache),
        load listener CFC, construct `socket` NativeObject bound to (connId, registry),
        dispatch via call_lifecycle_method (onMessage) OR __funcmeta `on=` lookup,
        with live application scope + attached session  (exactly like an HTTP request)
        │
   outbound (socket.emit / io().to(room).emit) ──►
        NativeObject.call_method routes through registry → pushes frame onto the
        target connection(s) mpsc::Sender(s); the conn task drains sender → ws sink
```

The fresh-VM-per-message model is **already how HTTP works** — nothing changes about VM isolation, JIT leasing, or app-scope live handles. A message is just a request whose "response" is zero or more outbound frames.

### New pieces

1. **`WebSocketRegistry`** on `ServerState` (`crates/cfml-vm/src/lib.rs`), **behind a `Broker` trait from day one** (decision 3):
   - `connections: DashMap<ConnId, ConnEntry>` — `ConnEntry { channel, tx, rooms, data, session_id }` — holds only this node's *local* sockets.
   - `rooms: DashMap<(channel, room), HashSet<ConnId>>` — local membership index.
   - `channels: HashMap<channel, CfcPath>`.
   - **`ConnId` is node-qualified** (`{nodeId}:{ulid}`) so "send to connection X" routes to the owning node, and message envelopes carry a monotonic per-channel id (for resumability, P12).
   - Methods (`emit_to`, `broadcast(channel, except)`, `to_room`, `join/leave`, `members`, `presence`) dispatch through a **`Broker`**: `LocalAdapter` (single node) delivers to local `tx` senders; a distributed adapter additionally publishes/​subscribes the fan-out across nodes and merges remote room membership + presence. Same call sites, swappable backend — **no rework to go multi-node**.
   - Pure data + channel sends — **no axum/tokio types leak into cfml-vm**, so the wasm build stays green (sender + broker behind traits; no-op impls on `wasm32`).

2. **`SocketHandle`** implementing `CfmlNative` (`crates/cfml-vm/src/websocket.rs` or in `cli`, registered via the registrar): wraps `(ConnId, Arc<WebSocketRegistry>)`, maps `emit/send/broadcast/join/leave/to/close/set/get/id/data` onto registry ops. `to(room)` returns a second small NativeObject (scoped emitter). Mirrors the `FutureNative` pattern.

3. **Connection driver** in `crates/cli` (axum-side, `#[cfg(not(wasm))]`): the per-connection tokio task; inbound→`spawn_blocking` dispatch; outbound mpsc→sink pump; cleanup (leave rooms, fire `onDisconnect`) on drop.

4. **Dispatcher**: resolves channel→CFC (bytecode-cached), instantiates listener, builds `socket`, calls the right method. For socket.io, decode packet → `(event, args)` → look up the function whose `__funcmeta` has `on == event` (fallback `onMessage`).

5. **socket.io transport (Phase 3)**: add **`socketioxide`** — a tower layer that mounts on the existing axum router and handles Engine.IO handshake, namespaces, rooms, acks, binary, and polling↔websocket fallback. We implement **one** connect handler adapting socketioxide's `SocketRef` to our `SocketHandle`/registry, so the CFML API above is unchanged. **Do not hand-roll Engine.IO.**

### Cross-instance relay — foundational, not deferred (decision 3)

The `Broker` seam above is designed in from Phase 1, so cluster fan-out is *the same code path* whether one node or fifty. The single-node `LocalAdapter` ships first; the **distributed adapter reuses the existing shared-session cluster** (memberlist + Memcached/Automerge, already feature-gated) to publish broadcasts and replicate room membership + presence across nodes, and lands as soon as the core is usable (Phase 2) rather than as a late add-on. `socketioxide` also ships a Redis adapter we can wire as an alternative for the socket.io transport. This delivers SocketBox's "scale without sticky sessions" headline. Because `ConnId` is node-qualified and the wire carries per-channel message ids, nothing about ids, acks, presence, or resumability needs to change when the distributed backend is switched on.

### wasm / Cloudflare worker

Out of scope for Phase 1–4. Native registry types are kept axum/tokio-free so `cfml-worker` / `rustcfml-wasm` keep compiling. The future worker path is **Durable Objects + the WebSocket Hibernation API** (a separate design), not this code.

---

## Phasing

Reshaped by the locked-in decisions: the `Broker` seam + cluster-safe ids/wire are in Phase 1, the distributed backend moves up to Phase 2, and a socket.io-lucee compat track joins Phase 3. No Pusher.

| Phase | Scope | Covers |
|---|---|---|
| **0** | this design doc + impl plan | — |
| **1** | raw WS core, **cluster-ready foundation**: axum upgrade route; `WebSocketRegistry` behind the `Broker` trait (`LocalAdapter`); node-qualified `ConnId` + message-id wire; `SocketHandle`; listener discovery + `onConnect` (reject gate) `/onMessage/onDisconnect/onError`; ack-by-return; live `socket.data`; text **+ binary** + `encoding="json"`; engine auto-cleanup + auto-ping; flat `wsPublish` + `io()` accessor; CFML test harness | Lucee ext + SocketBox (raw WS) |
| **2** | rooms + annotation routing + **distributed Broker** + presence: `join/leave/to`, `on="event"` dispatch, `canJoin`/annotation auth, cross-node fan-out over the shared-session cluster, presence track/list/diff, resumable wire ids | emit-from-anywhere, multi-node scale-out |
| **3** | socket.io transport (`socketioxide`: namespaces, acks, polling fallback) **+ socket.io-lucee compat layer** (`SocketIoServer`/`io.on`/`socket.emit`/`joinRoom`) | Preside ext / socket.io-lucee |
| **4** | polish: Wheels-model / domain-event auto-broadcast, whisper/client-events, optional `/topic`·`/user` naming convention, socketioxide Redis adapter option | declarative realtime conveniences |
| **5** *(future)* | worker / Durable-Objects path | Cloudflare |

---

## Verification strategy (testing is non-trivial)

CFML tests can't act as a WS client, so the runner harness can't cover this directly.

- **Rust integration tests** in `crates/cli/tests/` (pattern of `jit_numeric.rs`): spin up a server, connect with `tokio-tungstenite` (raw) and a socket.io test client (Phase 3), assert echo / broadcast / rooms / lifecycle.
- **Example app** under `examples/websocket_chat/` (demo-chat parity with SocketBox / Preside) for manual + Playwright verification.
- Optional `wsConnect()` test BIF for CFML-harness loopback self-test, but the Rust integration tests are the source of truth.
- **Release gate reminder:** all usual gates (`cargo test --workspace`, `tests/runner.cfm` CLI+serve, `--target wasm32` build, `wasm-pack build crates/wasm`) must stay green — the registry-on-`ServerState` change touches shared types, so the **wasm build is the one to watch**.

---

## Fluent design principles (cross-ecosystem)

Distilled from a survey of ~12 realtime stacks outside CFML — Socket.IO, **Phoenix Channels/Presence**, Rails **Action Cable**/Turbo, **ASP.NET SignalR**, Go **Centrifugo**/melody, python-socketio/**Django Channels**, **Spring STOMP**/Ktor/Vert.x, **socketioxide**/axum, **Laravel Reverb**/Echo, **Pusher/Ably/Supabase**, **Cloudflare DO/PartyKit**/Bun, and **tRPC/Convex/Liveblocks**. Ordered by how universally they recur and how much they move developer experience.

**P1 — Emit-from-anywhere is THE primitive, not a side feature.** *Every* mature stack makes "push to clients from ordinary code that holds no socket — a normal request, a `cfschedule` task, a DB hook" first-class and identical to in-handler sends: Socket.IO `io`, Phoenix `Endpoint.broadcast`, Action Cable `ActionCable.server.broadcast`, SignalR `IHubContext`, Centrifugo `node.Publish`, Spring `SimpMessagingTemplate`, socketioxide's clone-cheap `SocketIo`, Laravel `Broadcast` facade, Pusher `trigger`, PartyKit `getByName().broadcast`. For a request/response engine this is *the* defining ergonomic — most CFML pushes will originate outside any connection.

**P2 — Channels/rooms/topics are zero-ceremony, server-assigned strings; clients can never self-join.** Joining an unknown string creates it; emptying it destroys it; no registration. The server alone calls `join`, so authorization is enforced at join time (security-positive). A two-level model recurs: a *channel/namespace* = handler + auth boundary; a *room/topic* = ad-hoc fan-out group within it. Brilliant small trick (Socket.IO, python-socketio): **every connection auto-joins a room named after its own id**, so "send to a user" and "send to a room" are one primitive.

**P3 — One component = one channel, convention-named lifecycle methods.** The most-praised and most CFML-idiomatic shape (Phoenix, Action Cable, SignalR virtual overrides, Django Channels, Javalin, PartyKit, socketioxide). The handler set is discoverable; cleanup is just the close body; routing lives next to the handler, never in a distant config file.

**P4 — Name the audience, not one overloaded `send`.** Two loved schools: distinct verbs (Phoenix `push`/`broadcast`/`broadcast_from`/`reply`) and a fluent target chain (`io.to(room).except(id).emit(event,data)` — Socket.IO/socketioxide/SignalR `Clients.Group(...)`). The chain is the most widely recognized and transfers to the JS client verb-for-verb. Self-echo control (`broadcast_from`/`toOthers`/`skip_sid`/`except`) is the most common need and deserves a named form.

**P5 — Acknowledgements = request/response over the socket.** Near-universal (Socket.IO callbacks, Phoenix reply tuples, SignalR `InvokeAsync`, python-socketio, socketioxide `AckSender`, Vert.x request/reply, Centrifugo RPC). The recurring CFML-idiomatic form in every "CFML fit" note: **the handler's return value becomes the client's ack** — zero new syntax — plus a server→client `request(event,data,timeout)` that returns a value.

**P6 — Authenticate once at the handshake; identity is then ambient.** Action Cable `identified_by`, SignalR `Context.User`, Centrifugo `OnConnecting`, socketioxide connect-middleware + extensions, Bun attach-data-at-upgrade, Socket.IO `io.use`. A dedicated connect gate that returns false / throws to reject — and may return the set of rooms to auto-subscribe — beats per-message checks. Reuse the existing session/cookie.

**P7 — Per-connection state is a plain bag.** SignalR `Context.Items`, socketioxide extensions, python-socketio sessions, PartyKit `connection.state`, Bun `ws.data`. CFML structs are reference types, so a live mutable `socket.data` struct gives this with no get/set/write-back ceremony.

**P8 — Declare the codec once.** Starlette's `encoding="json"` (cited as "tiny but elegant"), Ktor `contentConverter`, Javalin `messageAsClass`: one declaration and the handler receives a parsed value instead of raw text, killing per-handler `deserializeJSON`. Malformed input becomes a catchable error, not a silent null.

**P9 — The engine owns concurrency; never surface loops/backpressure to the developer.** Centrifugo's "never block in a handler" is hostile to CFML; the lesson (melody/coder make writes concurrency-safe by default) is that the *runtime* must serialize per-connection dispatch, keep a bounded outbound queue, and keep the frame receive-loop in Rust — so CFML handlers can safely block. This validates the convention-method model over Ktor's exposed `for (frame in incoming)` loop.

**P10 — Cleanup is structural, not the developer's burden.** Connect/close are symmetric and named; the engine auto-removes a connection from all rooms on disconnect regardless of whether `onClose` exists, so the #1 realtime leak is impossible by default. Ping/pong is auto-answered in the engine and never dispatched into CFML (PartyKit/Cloudflare auto-response).

**P11 — Presence as track/list + join/leave/sync diffs.** Named one of the two hardest problems, solved declaratively everywhere it matters (Phoenix Presence, Ably/Supabase/Pusher presence channels, Liveblocks). "Set my bit of state, read everyone else's"; merge-on-update; ship deltas.

**P12 — Resumability is a flag, and must be designed into the wire from day one.** Socket.IO `connectionStateRecovery`, Centrifugo history+stream-position, tRPC `tracked()`+`lastEventId`, Ably resume/recover. Carry a per-message id so a generic client can auto-recover; offer `{history=N}` as opt-in.

**P13 — Wire-protocol compatibility imports a mature client for free.** Laravel Reverb/Pusher/Ably are interchangeable because they share the Pusher protocol, so Laravel Echo "just works." Speaking an existing protocol (socket.io via `socketioxide`, and/or Pusher) hands us battle-tested reconnection, presence, and backoff on the client with zero bespoke JS — the single biggest client-side lever.

**P14 — Design for connection-free testing.** Phoenix `assert_broadcast`/`assert_push`, Action Cable `assert_broadcasts`, Laravel `Event::fake`. Make the channel CFC directly invocable and intercept the broadcast BIF (the existing `writeOutput`/`cfthread` VM-intercept pattern) so realtime logic is testable from `tests/runner.cfm` with no live socket.

**Deliberately NOT adopted:** strongly-typed hubs / reversible TS interfaces / GraphQL schema typing (no generics in CFML — can't enforce; use named events + optional runtime validation); Spring's STOMP destination-broker (judged heavyweight — at most an *optional* `/topic` `/user` naming convention); exposed receive-loops / reactive-streams backpressure (engine owns it); Convex reactive-DB auto-invalidation and Replicache dual-mutator local-first (too large, vendor-shaped, client-runtime concerns); Cloudflare memory-eviction hibernation (serve mode keeps the VM resident — steal only auto-response ping/pong + the persisted-attachment idea for clustering).

---

## Refinements to the proposed design

Tagged `[adopt]` (fold into Phase 1–3), `[consider]` (later phase), `[defer]` (Phase 4+).

**[adopt] Elevate emit-from-anywhere to the headline (P1).** A flat BIF is the canonical path, callable from any `.cfm`, `cfthread`, or scheduled task; `io()` is sugar over it. Registry lives on `ServerState` so it crosses requests.
```cfml
// before: io("/chat").to("lobby").emit("announcement", data)   // only path shown
// after:  both — flat BIF is primary, fluent accessor is sugar
wsPublish( channel="/chat", event="announcement", data=payload, except=cid );
io( "/chat" ).to( "lobby" ).except( cid ).emit( "announcement", payload );
```

**[adopt] Acknowledgements via return value + server→client request (P5).** No new syntax: a handler's return value is shipped as the client's ack. Add a server-initiated `socket.request()` returning a value (back it with the existing `runAsync`/`FutureNative` shim). Carry a correlation ref in the wire format.
```cfml
function save( socket, doc ) on="save" {
    var id = store( doc );
    return { ok = true, id = id };          // delivered to the client's ack callback
}
// server asks one client and awaits:
var answer = socket.request( "confirm?", change, timeout=5000 );
```

**[adopt] Resolve open question — dedicated connect gate, not an `onRequestStart` overload (P6).** `onConnect(socket)` may reject (return `false` / throw) and may return rooms to auto-join. Session/cookie identity resolves here, then is ambient on `socket`.
```cfml
function onConnect( socket ) {
    if ( !socket.isWebUser() ) return false;            // reject handshake
    socket.data.userId = socket.getWebsiteLoggedInUserId();
    return [ "lobby", "user:#socket.data.userId#" ];     // server-decided subscriptions
}
```

**[adopt] Resolve open question — live `socket.data` struct, drop `set/get`, no `static` (P7).** CFML structs are reference types, so per-connection state is just a mutable struct held by the engine for the connection's life. `socket.id`, `socket.data`, `socket.session` stay.

**[adopt] Resolve open question — both text and binary in v1, plus declare-codec-once (P8).** axum gives binary for free; auto-detect `CfmlValue::Binary` on send, deliver inbound binary as `Binary` to `onMessage`. A channel attribute `encoding="json"` makes `onMessage` receive a parsed struct (malformed → catchable error).
```cfml
component socket="/chat" encoding="json" {
    function onMessage( socket, message ) {     // `message` is already a struct
        io().to( "lobby" ).emit( "chat", message.text );
    }
}
```

**[adopt] Engine-owned concurrency, write-safety, auto-cleanup, auto-ping (P9, P10).** Per-connection serialized dispatch + bounded outbound `mpsc` (drop/close on overflow); the frame receive-loop stays in Rust; the engine removes a connection from all rooms on disconnect regardless of `onClose`; ping/pong is auto-answered in the engine and never dispatched into CFML.

**[adopt] Declarative authorization (P2, P6).** A `canJoin(socket, room)` hook gates room joins; method/component annotations (`function kick(socket,id) secured="admin" {}`) are checked via `getMetadata` before dispatch — reusing the reflection machinery we already have. Clients can never self-join.

**[adopt] Two-level channel/room model + self-id room (P2).** Document channel (CFC + auth) vs room (ad-hoc group) explicitly; auto-join each connection to a room named after its id so `wsPublish("/chat", ..., to="user:42")` and room sends are one primitive.

**[adopt] Connection-free CFML test harness (P14).** `wsTestConnect()/wsTestSend()` + `assertBroadcast(channel, event, predicate)` by intercepting the broadcast BIF (mirrors `writeOutput`/`cfthread`); the channel CFC is designed to be directly invocable. Lands the realtime suite inside `tests/runner.cfm` despite CFML not being a WS client.

**[adopt] Commit to the fluent target chain as canonical, with self-echo control everywhere (P4).** `io().to(room).except(id).emit(event,data)`; `socket.broadcast(event,data)` = sugar for "others in my rooms"; every publish path accepts `except`/`exclude`.

**[adopt → Phase 2] Presence (P11)** — `socket.track(meta)` / `io(channel).presence()`, with `presence_state` snapshot + `presence_diff` deltas in the wire format; cluster-correct via the distributed `Broker` (decision 3).

**[adopt → Phase 1 wire / Phase 2 replay] Resumability (P12)** — the per-message id / stream position is in the wire from Phase 1 (so ids never have to change); accept `lastEventId` at connect; v1 best-effort in-memory history, opt-in `{history=N}` later.

**[consider → Phase 4] Whisper / client events relayed by the hub (perf)** — typing/cursor chatter never enters the CFML request path; the hub relays `client-`-prefixed events with no CFML code running.

**[adopt] socket.io-lucee compatibility layer (decision 1).** Alongside the fluent API, a thin shim emulates the `socket.io-lucee` CFML surface so **preside-ext-socket-io** runs with minimal change: `new SocketIoServer()`, `io.on("connect", fn)`, `socket.on/emit/send`, `socket.joinRoom/leaveRoom/leaveAllRooms`, `ns.emit(rooms=…)`, `socket.broadcast(rooms=[…])`. It rides the same socket.io transport (Phase 3) and delegates to the same registry/`Broker` as the fluent API — one engine, two surfaces. (Preside's own helpers — `isWebUser()`, `getWebsiteLoggedInUserId()` — stay in the extension, reading the session the engine attaches at handshake.)

**[adopt] Cluster-ready foundation (decision 3, supersedes the old "Phase 4 clustering" item).** The `Broker` seam, node-qualified `ConnId`, and message-id wire are designed in from Phase 1; the distributed backend (over the existing shared-session cluster) lands in Phase 2. Multi-node correctness for ids/acks/presence/resumability is a constraint from the start, not retrofitted.

**[not now] Pusher protocol (P13).** Deliberately out of scope — socket.io + raw WS are the chosen wires. Revisit only if inheriting the Laravel Echo / `pusher-js` client ecosystem becomes a goal; the `Broker`/registry seam leaves room to add it as a third transport later without disturbing the CFML API.

**[defer] Marker/annotation auto-broadcast of domain events & Wheels model CRUD (P3-adjacent)** — `component broadcast=true` / a model mixin that publishes on `save`/`delete` (Laravel `ShouldBroadcast` / Turbo `broadcasts_to`). Phase 4.

**[defer] Turbo-Streams-style bounded-verb declarative client** (push rendered `.cfm` fragments, near-zero JS) and a **Convex-style live-query tier** (`queryWatch(name, sql)` + `invalidate(name)` over QoQ) — differentiators for a later phase.

These refinements drive the **Phasing** table above: Phase 1 = raw-WS core + cluster-ready foundation + ack-by-return + `onConnect` reject gate + live `socket.data` + text/binary/`encoding` + engine auto-cleanup/auto-ping + CFML test harness; Phase 2 = rooms + distributed `Broker` + presence + `canJoin`/annotation auth + resumable wire ids; Phase 3 = socket.io transport + the socket.io-lucee compat layer; Phase 4 = declarative conveniences (auto-broadcast, whisper, naming conventions).
