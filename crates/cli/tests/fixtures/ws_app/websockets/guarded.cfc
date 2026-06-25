/**
 * Reject-gate test channel: onConnect returns false, so the handshake is
 * rejected and the socket closed.
 */
component socket="/guarded" {
    function onConnect( socket ) {
        return false;
    }
}
