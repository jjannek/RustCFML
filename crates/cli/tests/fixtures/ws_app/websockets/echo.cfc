/**
 * Test channel for the WebSocket integration suite.
 * URL `/ws/echo` → this CFC. `encoding="json"` means onMessage receives a
 * parsed struct.
 */
component socket="/echo" encoding="json" {

    function onConnect( socket ) {
        socket.join( "lobby" );
        // socket.param() exposes handshake query params (?user=alice).
        socket.emit( "welcome", { id = socket.id(), user = socket.param( "user" ) } );
    }

    function onMessage( socket, message ) {
        // A message asking us to blow up exercises the onError lifecycle.
        if ( isStruct( message ) && structKeyExists( message, "boom" ) ) {
            throw( message = "boom requested" );
        }
        // Echo straight back to the sender...
        socket.emit( "echo", message );
        // ...and tell everyone else in the channel.
        socket.broadcast( "said", message );
        // Non-null return → delivered to the sender as an ack.
        return { ok = true };
    }

    // Event routing: an inbound `{"ev":"say","d":{…},"id":"…"}` frame is
    // dispatched here (by the `on="say"` annotation) rather than to onMessage.
    // The non-null return rides back as an ack whose `ref` echoes the inbound id.
    function handleSay( socket, data ) on="say" {
        socket.emit( "sayEcho", { text = data.text, routed = "say" } );
        return { routed = "say" };
    }

    function onError( socket, err ) {
        socket.emit( "errored", { message = err.message } );
    }

    function onDisconnect( socket, reason ) {
        io().to( "lobby" ).emit( "left", socket.id() );
    }
}
