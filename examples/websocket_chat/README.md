# RustCFML WebSocket Chat (demo)

A minimal multi-tab chat over a raw WebSocket, demonstrating the RustCFML
realtime engine (Phase 1). Parity with the SocketBox / Preside socket.io demo
chats, in ~25 lines of CFML.

## Run

```bash
cargo run --release -- --serve examples/websocket_chat
# then open http://localhost:8500/ in two browser tabs
```

## How it works

- `websockets/chat.cfc` is the **channel component**. Its path makes it
  reachable at `ws://host/ws/chat` (convention discovery; the explicit
  `socket="/chat"` attribute names the wire channel).
- Lifecycle methods are **convention-named and all optional**:
  `onConnect` / `onMessage` / `onDisconnect`.
- `encoding="json"` means `onMessage` receives a **parsed struct**, not raw text.
- `socket.emit(event, data)` sends to one client; `socket.broadcast(...)` to
  everyone else in the channel; `io().to("lobby").emit(...)` to a room from
  anywhere.
- The handler's **return value becomes the client's ack** (`ev:"ack"`).

## Emit from anywhere

Any ordinary `.cfm` page, `cfthread`, or scheduled task can push to connected
clients — no socket handle required:

```cfml
wsPublish( channel="/chat", event="message", data={ from="system", text="Maintenance in 5m" } );
// or the fluent form:
io( "/chat" ).to( "lobby" ).emit( "message", { from="system", text="…" } );
```

See `docs/websocket-design.md` for the full design.
