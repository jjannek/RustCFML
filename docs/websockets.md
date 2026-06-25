# WebSockets / Realtime

RustCFML has native WebSocket support: a long-lived, full-duplex connection
served from the **same process and port** as your HTTP traffic — no servlet
container, no embedded Jetty, no Node sidecar. You write a **channel component**
(one CFC = one channel) with convention-named lifecycle methods, and the engine
bridges each inbound frame to a fresh VM exactly as it does an HTTP request.

> **Status:** Phase 1 (raw WebSocket). Rooms, `join`/`leave`, the fluent `io()`
> emitter, ack-by-return, binary + JSON codecs, and emit-from-anywhere are all
> live. The socket.io transport, presence, multi-node fan-out, and the
> socket.io-lucee compatibility layer are on the roadmap — see
> [Roadmap](#roadmap). Design rationale lives in
> [`websocket-design.md`](websocket-design.md).

## Quick start

Create `websockets/chat.cfc` under your web root:

```cfml
component socket="/chat" encoding="json" {

    function onConnect( socket ) {
        socket.join( "lobby" );
        socket.emit( "welcome", { id = socket.id() } );
        socket.broadcast( "userJoined", { id = socket.id() } );   // everyone else
    }

    function onMessage( socket, message ) {          // `message` is a parsed struct
        io().to( "lobby" ).emit( "message", { from = socket.id(), text = message.text } );
        return { delivered = true };                  // becomes the client's ack
    }

    function onDisconnect( socket, reason ) {
        io().to( "lobby" ).emit( "userLeft", { id = socket.id() } );
    }
}
```

Connect from the browser at `ws://host/ws/chat`:

```js
const ws = new WebSocket(`ws://${location.host}/ws/chat`);
ws.onmessage = (e) => {
  const frame = JSON.parse(e.data);          // { t, ch, ev, d, id }
  if (frame.ev === "message") console.log(frame.d.from, frame.d.text);
};
ws.onopen = () => ws.send(JSON.stringify({ text: "hello" }));
```

A complete runnable demo (two-tab chat) lives in
[`examples/websocket_chat/`](../examples/websocket_chat/):

```bash
rustcfml --serve examples/websocket_chat   # then open http://localhost:8500/ in two tabs
```

## Channels & discovery

- A **channel** is one CFC under `/websockets/`. `websockets/chat.cfc` is
  reachable at `ws://host/ws/chat`.
- The optional `socket="/chat"` component attribute names the **wire channel**
  (the `ch` field in frames, and the target for `io("/chat")` / `wsPublish`).
  Without it the channel id defaults to `/<filename>`.
- `encoding="json"` makes `onMessage` receive a **parsed value** instead of raw
  text (see [Codec](#codec)).

## Lifecycle methods

All are optional and matched by convention (case-insensitive), like
`Application.cfc`.

| Method | When | Notes |
|---|---|---|
| `onConnect( socket )` | After the handshake | **Reject gate** — see below. |
| `onMessage( socket, message )` | Each inbound frame | Return value → client ack. |
| `onDisconnect( socket, reason )` | Connection closed | Always fires; rooms are auto-left regardless. |
| `onError( socket, err )` | A handler threw | `err` is `{ message, type }`. The connection survives. |

### The connect reject gate

`onConnect` decides whether the connection is allowed and what rooms it joins:

```cfml
function onConnect( socket ) {
    if ( socket.param( "token" ) != getSecretToken() ) {
        return false;                      // reject → handshake closed
    }
    return [ "lobby", "user:" & socket.param( "userId" ) ];   // auto-join these rooms
}
```

- Return `false` (or throw) → the connection is **rejected and closed**.
- Return an **array of room names** → the socket auto-joins them.
- Return nothing → the connection is accepted with no extra rooms.

Clients can **never** join rooms themselves — only server code calls
`socket.join()` — so authorization is enforced at join time.

## Authorization

Two declarative gates, both reading the identity you attach to `socket.data` at
connect (a real app resolves this from the session — see `socket.sessionId()`):

```cfml
function onConnect( socket ) {
    var user = lookUpUser( socket.sessionId() );   // your own resolution
    socket.data.authenticated = !isNull( user );
    socket.data.roles = isNull( user ) ? [] : user.roles;   // e.g. ["admin"]
}
```

**`secured` on a handler** gates it *before it runs*:

```cfml
function purge( socket, data ) on="purge" secured="admin" {  // needs the admin role
    ...
}
function whoami( socket, data ) on="whoami" secured {        // any authenticated socket
    return io().presence();
}
```

- bare `secured` → requires a truthy `socket.data.authenticated`.
- `secured="admin,editor"` → requires `socket.data.roles` (array) or
  `socket.data.role` (string) to include one of the listed roles (case-insensitive).
- `secured="false"` → explicitly opts out.

A denied call doesn't run the handler — it surfaces through `onError(socket, err)`
(an `onConnect` denial rejects the handshake), so the client gets a clean error
rather than silence.

**`canJoin( socket, room )`** gates room joins. Whenever server code calls
`socket.join( room )`, the channel's `canJoin` (if defined) is consulted first; a
falsey return (or a throw) rejects the join *loudly* — so a join derived from
client-supplied input can't slip a user into a room they shouldn't see:

```cfml
function canJoin( socket, room ) {
    return room.startsWith( "public-" ) || socket.data.roles.contains( "admin" );
}
```

## Event routing (`on="event"`)

For an `encoding="json"` channel, an inbound frame can name an **event** and be
routed to a dedicated handler instead of the catch-all `onMessage`. Annotate a
function with `on="<event>"`:

```cfml
component socket="/chat" encoding="json" {

    function say( socket, data ) on="say" {
        socket.broadcast( "said", data );
        return { delivered = true };          // → ack (see below)
    }

    function typing( socket, data ) on="typing" {
        socket.to( data.room ).emit( "typing", { who = socket.id() } );
    }

    function onMessage( socket, message ) {   // fallback: any frame with no matching event
        // ...
    }
}
```

A client sends an event frame matching the wire shape — `ev` is the event name,
`d` the payload, and an optional `id` rides back on the ack's `ref` so the
client can correlate the reply:

```js
ws.send(JSON.stringify({ ev: "say", d: { text: "hi" }, id: "req-1" }));
// → handler `say(socket, { text:"hi" })` runs; ack frame comes back with ref:"req-1"
```

Routing is case-insensitive. A frame with **no** `ev` (or whose event matches no
`on=` handler) falls through to `onMessage`, so un-annotated channels behave
exactly as before. The handler receives the `d` payload as its second argument.

## The `socket` object

The live handle passed to every lifecycle method:

| Method | Description |
|---|---|
| `socket.id()` | Stable per-connection id (node-qualified). |
| `socket.send( data )` | Raw frame to this client. A binary value is sent as a binary frame; anything else as JSON text. |
| `socket.emit( event, data )` | Named event to this client. |
| `socket.broadcast( event, data )` | To everyone in the channel **except** the sender. |
| `socket.join( room )` / `socket.leave( room )` | Room membership. |
| `socket.rooms()` | Array of rooms this socket is in. |
| `socket.to( room )` | Returns a scoped emitter → `.emit()` / `.send()` / `.except()`. |
| `socket.track( [key], meta )` / `socket.untrack( [key] )` | Presence — see below. |
| `socket.close( [code], [reason] )` | Close this connection. |
| `socket.data` | A **live, mutable struct** for per-connection state (see below). |
| `socket.param( name )` / `socket.params()` | Handshake query parameters (`?userId=42`). |
| `socket.sessionId()` | The `CFID` captured at the handshake. |

### Per-connection state: `socket.data`

`socket.data` is a plain CFML struct the engine keeps alive for the connection's
lifetime. Because structs are reference types, you just read and write it — no
`get`/`set`, no write-back ceremony:

```cfml
function onConnect( socket )            { socket.data.joinedAt = now(); }
function onMessage( socket, message )   { socket.data.lastSeen  = now(); }
```

## Rooms

A **room** is an ad-hoc fan-out group inside a channel. Joining an unknown room
creates it; emptying it destroys it — no registration. Every connection also
auto-joins a room named after **its own id**, so "send to one user" and "send to
a room" are the same primitive.

```cfml
socket.join( "room-42" );
io().to( "room-42" ).emit( "update", payload );      // everyone in the room
io().to( socket.id() ).emit( "private", payload );   // just one connection
```

On disconnect the engine removes the connection from **every** room
automatically, whether or not you define `onDisconnect` — the most common
realtime leak is impossible by default.

## Presence

Presence answers "who's here?" — *set my bit of state, read everyone else's*.
A connection **tracks** itself with some metadata; the engine keeps a per-channel
roster and ships **diffs** as people come and go (the Phoenix Presence model).

```cfml
function onConnect( socket ) {
    socket.track( { user = socket.param( "userId" ), status = "online" } );
}
```

- **`socket.track( meta )`** — adds this connection to the roster keyed by its
  own id. **`socket.track( key, meta )`** groups several connections under one
  key (e.g. a user's multiple tabs/devices → one roster entry with many `metas`).
  Re-tracking under the same key updates the meta.
- **`socket.untrack()`** / **`socket.untrack( key )`** — removes it. Disconnect
  untracks automatically, so the roster never leaks a ghost.

When a connection tracks, **it** receives the full roster as a `presence_state`
frame, and **everyone else** gets a `presence_diff` join. Leaves (including
disconnects) broadcast a `presence_diff` leave. Read the roster anytime:

```cfml
io().presence();            // inside a handler — the current channel's roster
io( "/chat" ).presence();   // by channel, from anywhere
wsPresence( "/chat" );      // flat-BIF equivalent
```

The roster shape (also the payload of a `presence_state` frame):

```jsonc
{
  "user-42": { "metas": [ { "user": "user-42", "status": "online" } ] },
  "user-99": { "metas": [ { "user": "user-99", "status": "away" } ] }
}
```

A `presence_diff` frame carries `{ "joins": { … }, "leaves": { … } }` in the same
per-key shape. Presence is channel-scoped; multi-node correctness arrives with the
distributed broker (later in Phase 2) with no API change.

## Emit from anywhere

You don't need a `socket` handle to push to clients. Any ordinary `.cfm` page,
`cfthread`, or scheduled task can publish — this is the primary realtime
ergonomic for a request/response engine.

**`wsPublish()`** — the flat BIF, the canonical path:

```cfml
wsPublish( channel="/chat", event="announcement", data={ text="Maintenance in 5m" } );
wsPublish( channel="/chat", event="ping", data=now(), to="lobby" );    // a single room
wsPublish( channel="/chat", event="x", data=d, except=excludeConnId ); // self-echo control
```

**`io()`** — the fluent accessor (sugar over the same registry):

```cfml
io( "/chat" ).emit( "ping", now() );                       // whole channel
io( "/chat" ).to( "lobby" ).emit( "announcement", data );  // a room
io( "/chat" ).to( "lobby" ).except( cid ).emit( "x", d );  // exclude one connection
io( "/chat" ).in( "lobby" ).count();                       // member count
io( "/chat" ).sockets();                                   // connection ids
io( "/chat" ).presence();                                  // the presence roster
```

Inside a channel handler, `io()` with **no argument** refers to the current
channel.

## Codec

By default `onMessage` receives the raw text (or a `Binary` value for binary
frames). Declaring `encoding="json"` on the channel parses inbound text once, so
the handler gets a struct/array directly:

```cfml
component socket="/chat" encoding="json" {
    function onMessage( socket, message ) {   // already deserialized
        io().to( "lobby" ).emit( "chat", message.text );
    }
}
```

Malformed JSON degrades to the raw string rather than dropping the message, so
your handler can validate it.

## Acknowledgements

The return value of `onMessage` (and any `on="event"` handler) is shipped back
to the sending client as an `ack` frame (`ev:"ack"`). Return nothing for no ack:

```cfml
function onMessage( socket, message ) {
    var id = store( message );
    return { ok = true, id = id };     // client receives this as the ack
}
```

When the inbound frame carried an `id`, the ack echoes it back as `ref` so the
client can match the reply to its request.

## Resumability (`history` / `lastEventId`)

A client that briefly drops off the network can reconnect and **replay the
messages it missed**, in order, before live traffic resumes. Opt in per channel
with the `history="N"` attribute — the engine then retains the last `N`
channel-wide frames:

```cfml
component socket="/feed" encoding="json" history="100" {
    function publish( socket, data ) on="post" {
        io().emit( "post", data );   // channel-wide → retained in history
    }
}
```

Every outbound frame carries a monotonic `id` (`{nodeId}:{seq}`). The client
remembers the `id` of the last frame it processed and, on reconnect, sends it as
a `lastEventId` query parameter:

```js
const last = localStorage.getItem("lastEventId");
const url  = "/ws/feed" + (last ? "?lastEventId=" + encodeURIComponent(last) : "");
const ws   = new WebSocket(url);
ws.onmessage = e => { const f = JSON.parse(e.data); if (f.id) localStorage.setItem("lastEventId", f.id); /* … */ };
```

The engine replays every retained frame newer than that cursor (keeping each
frame's original `id`, so the client keeps advancing). What gets retained is the
**channel-wide fan-out** — `io().emit()`, `io().to(room).emit()`,
`socket.broadcast()`. Per-connection sends (acks, a `presence_state` snapshot) are
not history and are never replayed.

If the cursor is older than the oldest frame still retained, the client has
provably lost messages: the engine first sends a `{ "t": "reset" }` frame so the
app knows to resync from its own source of truth, then replays what it still has.

```jsonc
{ "t": "reset", "ch": "/feed", "id": "node:512" }   // gap — you missed messages; resync
```

**Caveats (this phase).** History is **best-effort, in-memory, and per node**:
it is lost on restart, capped at `N` frames, and a `lastEventId` minted by a
*different* node (after a failover) is skipped (the client gets the `reset` hint).
Replay is channel-wide, not room-precise — the socket re-establishes its rooms via
`onConnect` auto-join anyway. Cluster-correct, durable resumability arrives with
the distributed broker (later in Phase 2) with no API change.

## Wire format

Raw-WS frames are JSON with a stable shape (designed once so ids never change
when clustering is enabled later):

```jsonc
{
  "t":  "msg",        // frame type: msg | ack | ...
  "ch": "/chat",      // channel
  "ev": "message",    // event name (routes to on="message"; absent for a raw send())
  "d":  { },          // payload
  "id": "node:42",    // node-qualified, monotonic message id
  "ref":"req-1"       // ack correlation — echoes an inbound frame's id (acks only)
}
```

## Clustering / multi-node

When the server runs as part of a **clustered session store** (`--features
cluster`, with `sessionStorage` pointing at a `provider: "cluster"` cache),
WebSocket fan-out automatically rides the same gossip cluster — no extra config.
A `wsPublish` / `io().broadcast()` / `socket.to(room)` on one node reaches
clients connected to **any** node, and the presence roster is cluster-wide. The
CFML API is identical to single-node; nothing in your channel code changes.

How it works: every channel-wide broadcast and room fan-out is published to peer
nodes over the cluster, and each node re-delivers to its own connected clients
(the Socket.IO Redis-adapter model — no sticky sessions needed). Connection ids
are node-qualified, so a send to a specific connection routes to its owning node.
Presence `track`/`untrack` is replicated, and when a node leaves the cluster its
roster entries are evicted (a leave diff is emitted to remaining clients).

Caveats (best-effort, in line with the realtime contract):

- **Fan-out is best-effort.** A cross-node frame dropped under load is a missed
  realtime message, not state corruption — exactly like a single dropped frame to
  a slow client.
- **`lastEventId` replay stays node-local.** History is retained on the node that
  produced the frames; a client that reconnects to a *different* node receives a
  `reset` hint (no replay) rather than a gap. Pin reconnects to the same node, or
  treat `reset` as "resync from scratch".
- **Room `count()` / `sockets()` are node-local.** They report this node's members
  only — room *membership* is not replicated (delivery does not need it). For a
  cluster-wide "who's here", use **presence** (`io(channel).presence()`), which is
  replicated.

## Concurrency & safety

The engine owns the connection's event loop, so your handlers never see frame
loops or backpressure:

- Per-connection dispatch is **serialized**; each message runs on a fresh VM
  (like an HTTP request), so handlers may safely block.
- Outbound frames go through a **bounded queue** — a slow client cannot stall a
  handler; on overflow the connection is closed.
- Ping/pong is answered by the engine and **never** dispatched to CFML.

## Testing without a live socket

Realtime logic is testable from `tests/runner.cfm` with no WebSocket client.
`wsPublish` records every broadcast, and `assertBroadcast()` inspects it:

```cfml
wsPublish( channel="/chat", event="message", data={ text="hi" } );

assertBroadcast( "/chat", "message" );                                  // true
assertBroadcast( "/chat", "message", function( d ){ return d.text == "hi"; } );  // predicate
assertBroadcast( "/chat", "nope" );                                     // false
```

(Live-socket behaviour — echo, broadcast, rooms, reject, binary — is covered by
the Rust integration suite in `crates/cli/tests/websocket_raw.rs`.)

## Roadmap

Phase 1 (this page) covers the raw-WebSocket core. **Phase 2 is complete:**
`on="event"` annotation routing, ack `ref` correlation, **presence**,
**authorization** (`secured=` / `canJoin`, above), **`lastEventId` resumability**
(`history=`, above), and **multi-node fan-out** over the shared-session cluster
([Clustering / multi-node](#clustering--multi-node), above) have all landed.
Planned:

- **Phase 3** — a **socket.io** transport (Engine.IO handshake, namespaces, acks, polling fallback) plus a **socket.io-lucee compatibility layer** so existing socket.io CFML apps run with minimal change.
- **Phase 4** — declarative conveniences: model/domain-event auto-broadcast, whisper/client events, optional `/topic`·`/user` naming conventions.

See [`websocket-implementation-plan.md`](websocket-implementation-plan.md) for
the full build order.
