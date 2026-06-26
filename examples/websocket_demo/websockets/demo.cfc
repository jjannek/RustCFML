/**
 * RustCFML realtime — "kitchen-sink" demo channel.
 *
 * One CFC exercises (nearly) the whole WebSocket surface so the bundled
 * index.html can drive each feature interactively:
 *
 *   • lifecycle        onConnect / onMessage / onError / onDisconnect
 *   • event routing    function … on="chat" / on="join" / on="leave"
 *   • the socket object id / emit / broadcast / send / join / leave / rooms /
 *                      to().except() / track / untrack / data / param /
 *                      sessionId / close
 *   • presence         socket.track() → live roster (presence_state/diff)
 *   • authorization    canJoin gate (the "private" room is refused)
 *   • whisper          clientEvents="typing" → client-typing relayed, no CFML
 *   • resumability     history="50" → lastEventId replay on reconnect
 *   • emit-from-anywhere  see broadcast.cfm (a plain page calling wsPublish)
 *
 * Run:  rustcfml --serve examples/websocket_demo
 *       open http://localhost:8500/ in two or three browser tabs.
 *
 * URL `/ws/demo` → this CFC (convention discovery from /websockets/*.cfc).
 */
component socket="/demo" encoding="json" history="50" clientEvents="typing" {

    // ── lifecycle ─────────────────────────────────────────────────────────

    function onConnect( socket ) {
        // Identity: a ?name= handshake param, else a short id-derived fallback.
        var name = socket.param( "name" );
        if ( !len( name ) ) {
            name = "guest-" & left( replace( socket.id(), ":", "", "all" ), 6 );
        }

        // socket.data is a LIVE per-connection struct — just read/write it.
        socket.data.name     = name;
        socket.data.joinedAt = now();

        // Auto-join the lobby and announce presence (cluster-wide roster).
        socket.join( "lobby" );
        socket.track( { name = name } );

        // Greet just this client with everything it needs to render.
        socket.emit( "welcome", {
            id        = socket.id(),
            name      = name,
            sessionId = socket.sessionId(),
            rooms     = socket.rooms(),
            online    = io().in( "lobby" ).count()
        } );

        // Tell everyone else someone arrived.
        socket.broadcast( "system", { text = name & " joined" } );
    }

    // Un-annotated frames (a raw send with no matching event) land here.
    function onMessage( socket, message ) {
        if ( isStruct( message ) && structKeyExists( message, "boom" ) ) {
            throw( message = "you asked me to boom" );   // → onError
        }
        socket.emit( "echo", message );
        return { ok = true };                            // → ack to the sender
    }

    function onError( socket, err ) {
        socket.emit( "errored", { message = err.message } );
    }

    function onDisconnect( socket, reason ) {
        // socket.data still holds the name captured at connect time.
        socket.broadcast( "system", { text = socket.data.name & " left (" & reason & ")" } );
    }

    // ── event routing (on="…") ──────────────────────────────────────────────

    /** A chat line, routed by annotation rather than going through onMessage. */
    function chat( socket, data ) on="chat" {
        var room = structKeyExists( data, "room" ) ? data.room : "lobby";
        io().to( room ).emit( "chat", {
            from = socket.data.name,
            text = data.text,
            room = room,
            at   = timeFormat( now(), "HH:mm:ss" )
        } );
        // The return value becomes the sender's ack (correlated by `ref`).
        return { delivered = true, room = room };
    }

    /** Join a room. The canJoin gate (below) can refuse it. */
    function joinRoom( socket, data ) on="join" {
        socket.join( data.room );                        // throws if canJoin says no
        socket.emit( "rooms", { rooms = socket.rooms() } );
        // Tell the room's existing members (not me) that I arrived.
        socket.to( data.room ).except( socket.id() )
              .emit( "system", { text = socket.data.name & " entered " & data.room } );
    }

    /** Leave a room. */
    function leaveRoom( socket, data ) on="leave" {
        socket.leave( data.room );
        socket.emit( "rooms", { rooms = socket.rooms() } );
    }

    // ── authorization ───────────────────────────────────────────────────────

    /** Room-join gate: the "private" room is off-limits to demonstrate canJoin. */
    function canJoin( socket, room ) {
        return room != "private";
    }
}
