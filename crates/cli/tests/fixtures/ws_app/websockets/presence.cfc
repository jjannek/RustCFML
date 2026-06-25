/**
 * Presence test channel (Phase 2). URL `/ws/presence`.
 * onConnect tracks the socket with its handshake `user` param; the connecting
 * client gets a `presence_state` snapshot, others a `presence_diff` join, and a
 * disconnect auto-emits a `presence_diff` leave. The `on="roster"` handler
 * returns the live roster (delivered as an ack).
 */
component socket="/presence" encoding="json" {

    function onConnect( socket ) {
        socket.track( { user = socket.param( "user" ) } );
    }

    function roster( socket, data ) on="roster" {
        return io().presence();
    }
}
