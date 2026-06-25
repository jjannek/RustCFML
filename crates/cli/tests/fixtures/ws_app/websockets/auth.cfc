/**
 * Auth test channel (Phase 2). URL `/ws/auth`.
 * onConnect resolves identity from the handshake `role` param onto socket.data
 * (a real app would read it from the session). `secured` handlers are gated on
 * that identity before they run; `canJoin` gates room joins.
 */
component socket="/auth" encoding="json" {

    function onConnect( socket ) {
        var r = socket.param( "role" );
        if ( isNull( r ) ) { r = ""; }
        socket.data.role = r;
        socket.data.authenticated = len( r ) > 0;
        socket.emit( "ready", {} );
    }

    // Requires the "admin" role.
    function adminOnly( socket, data ) on="admin" secured="admin" {
        socket.emit( "adminOk", {} );
        return { ok = true };
    }

    // Requires any authenticated socket (bare `secured`).
    function membersOnly( socket, data ) on="member" secured {
        socket.emit( "memberOk", {} );
        return { ok = true };
    }

    // canJoin gate: only rooms beginning "public-" are allowed.
    function canJoin( socket, room ) {
        return left( room, 7 ) == "public-";
    }

    function doJoin( socket, data ) on="join" {
        socket.join( data.room );          // throws if canJoin rejects
        socket.emit( "joined", { room = data.room } );
        return { ok = true };
    }

    function onError( socket, err ) {
        socket.emit( "denied", { message = err.message } );
    }
}
