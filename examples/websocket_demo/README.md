# RustCFML Realtime — kitchen-sink demo

An interactive, single-page demo that drives **(nearly) the whole RustCFML
WebSocket surface** from one channel CFC, so you can see each feature on the
wire as you use it. For the minimal "just chat" version, see
[`../websocket_chat`](../websocket_chat). For the full reference, see
[`docs/websockets.md`](../../docs/websockets.md).

## Run

```bash
cargo run --release -- --serve examples/websocket_demo
# then open http://localhost:8500/ in two or three browser tabs
```

Give each tab a name, hit **Connect**, and watch the **Wire log** panel — it
prints every frame you send (`▶ out`) and every frame the engine pushes back
(`◀ in`), so the JSON envelope behind each button is visible.

## What it demonstrates

Everything below lives in `websockets/demo.cfc` (≈90 lines).

### Lifecycle (convention methods, all optional)

| Method | Fires when | Demo uses it to |
|---|---|---|
| `onConnect( socket )` | a client connects | set `socket.data`, join `lobby`, `track()` presence, send `welcome` + broadcast a join notice. Return `false` to reject, or an array of room names to auto-join. |
| `onMessage( socket, msg )` | a frame with no matching `on=` event | echo back + ack; `{boom:true}` throws to show `onError`. |
| `onError( socket, err )` | a handler throws | emit an `errored` event (the connection survives). |
| `onDisconnect( socket, reason )` | a client leaves | broadcast a leave notice (reads the name from `socket.data`). |

### Event routing — `function … on="event"`

A JSON frame `{ ev:"chat", d:{…}, id:"…" }` is routed to the annotated handler
instead of `onMessage`; the handler's **return value becomes the client's ack**
(its `ref` echoes the inbound `id`).

- `chat()  on="chat"`  → `io().to(room).emit("chat", …)`
- `joinRoom()  on="join"` / `leaveRoom()  on="leave"`  → `socket.join` / `socket.leave`

### The `socket` object

`socket.id()`, `socket.emit(event,data)`, `socket.broadcast(event,data)`
(everyone *except* the sender), `socket.send(data)`, `socket.join(room)` /
`socket.leave(room)`, `socket.rooms()`, `socket.to(room).except(id).emit(…)`,
`socket.track([key],meta)` / `socket.untrack()`, `socket.data` (a **live
per-connection struct**), `socket.param(name)` / `socket.params()`,
`socket.sessionId()`, `socket.close()`.

### Presence — `socket.track()`

`track({ name })` in `onConnect` publishes to the channel's roster. Clients get a
`presence_state` snapshot on join and `presence_diff` join/leave deltas after —
the **Presence roster** panel renders them live. The roster is cluster-wide.

### Authorization — `canJoin`

`canJoin( socket, room )` gates room joins. Here it refuses `"private"`, so the
**join "private"** button trips the gate: `socket.join` throws → `onError` →
you see an `⚠ onError` line. (Method-level `secured="role"` annotations are
documented in `docs/websockets.md`; not shown here.)

### Whisper / client events — `clientEvents="typing"`

As you type, the page sends a `client-typing` **whisper**. The hub relays it to
the other clients with **no CFML running** (no handler, no ack, not in history) —
that's why the *typing indicator* updates with zero server code. `clientEvents`
declares the relay event so it also works over the socket.io transport. See the
[whisper docs](../../docs/websockets.md#whisper--client-events).

### Resumability — `history="50"` + `lastEventId`

The channel retains its last 50 channel-wide frames. **Reconnect (replay)**
reconnects passing `?lastEventId=<last id seen>`, and the engine replays what you
missed before live traffic resumes.

### Emit from anywhere — `broadcast.cfm`

`broadcast.cfm` is a plain HTTP page (no socket, no channel instance) that calls
`wsPublish(channel="/demo", event="system", data={…}, to="lobby")`. A
`cfthread`, scheduled task, or queue worker would push the same way. The
**announce** button hits it with `fetch()`.

## Files

```
websockets/demo.cfc   the channel — all the server logic
broadcast.cfm         emit-from-anywhere (wsPublish from an HTTP request)
index.html            the interactive client (vanilla JS, no dependencies)
```
