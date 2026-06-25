/**
 * Raw (non-JSON) channel: echoes whatever it receives straight back. Used to
 * verify binary frames round-trip as binary (no encoding="json").
 */
component socket="/raw" {
    function onMessage( socket, message ) {
        socket.send( message );
    }
}
