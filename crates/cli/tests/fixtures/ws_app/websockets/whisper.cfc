/**
 * Whisper / client-event channel (Phase 4). A `client-*` event is relayed to
 * peers by the hub with NO CFML running — none of the handlers below fire for
 * a whisper. `clientEvents` declares the relay events for the socket.io
 * transport (the raw-WS transport relays any inbound `client-*` dynamically).
 */
component socket="/whisper" encoding="json" clientEvents="typing,cursor" {

    function onConnect( socket ) {
        socket.join( "lobby" );
        socket.emit( "ready", { id = socket.id() } );
    }

    // A normal (non-whisper) message DOES run CFML — used to prove the
    // connection is intact and that whispers bypassed this handler.
    function onMessage( socket, message ) {
        socket.broadcast( "said", message );
    }
}
