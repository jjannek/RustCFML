component {

	this.name = "sio_compat_app";

	/**
	 * Bootstrap the imperative socket.io-lucee surface once, on first request.
	 * Stored in application scope so the captured handler closures (and the
	 * whole SocketIoServer object graph) ride across the per-message dispatches.
	 */
	public boolean function onApplicationStart() {
		application.io = new SocketIoServer();

		var io = application.io;
		io.of( "/im" ).on( "connect", function( socket ){
			// Greet the client with its own id (proves the connect listener ran
			// against a live socket facade).
			socket.emit( "welcome", { id = socket.getId() } );

			// Per-socket inbound listener with a native ack (the return value).
			socket.on( "say", function( msg ){
				socket.emit( "sayEcho", { routed = "say", text = msg.text } );
				socket.broadcast( "said", msg );        // to everyone else
				return { routed = "say", text = msg.text };
			} );

			// Rooms: join, then namespace-broadcast only to that room.
			socket.on( "joinRoom", function( data ){
				socket.joinRoom( data.room );
				io.of( "/im" ).emit( "roomNews", { room = data.room }, [ data.room ] );
			} );
		} );

		return true;
	}
}
