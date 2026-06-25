/**
 * Demo chat channel — SocketBox / Preside-parity sample for the RustCFML
 * realtime engine. Run with:  rustcfml --serve examples/websocket_chat
 * then open http://localhost:8500/ in two tabs.
 *
 * URL `/ws/chat` → this CFC (convention discovery from /websockets/*.cfc).
 * `encoding="json"` → onMessage receives a parsed struct.
 */
component socket="/chat" encoding="json" {

    // Convention lifecycle — all optional, zero ceremony.
    function onConnect( socket ) {
        socket.join( "lobby" );
        socket.emit( "welcome", { id = socket.id(), online = io().in( "lobby" ).count() } );
        socket.broadcast( "userJoined", { id = socket.id() } );
    }

    function onMessage( socket, message ) {
        // Everyone in the lobby, including the sender, sees the message.
        io().to( "lobby" ).emit( "message", { from = socket.id(), text = message.text } );
        return { delivered = true };   // ack back to the sender
    }

    function onDisconnect( socket, reason ) {
        io().to( "lobby" ).emit( "userLeft", { id = socket.id() } );
    }
}
