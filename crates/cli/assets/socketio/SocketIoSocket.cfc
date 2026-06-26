/**
 * A single socket.io connection (socket.io-lucee compat). Register inbound
 * event listeners with `on`, send direct messages with `emit`/`send`, broadcast
 * to others, and join/leave rooms. Per-socket data lives in `socketData`
 * (engine-side, so it survives across the per-message dispatches).
 */
component {

	public any function init( required string id, required string namespace ) {
		variables.id        = arguments.id;
		variables.namespace = arguments.namespace;
		return this;
	}

	public string function getId() {
		return variables.id;
	}

	public string function getNamespace() {
		return variables.namespace;
	}

	/**
	 * Register an inbound event listener. The callback receives the argument(s)
	 * the client sent.
	 */
	public void function on( required string event, required any callback ) {
		$sioRegisterSocketHandler( variables.id, arguments.event, arguments.callback );
	}

	/**
	 * Send a direct event to this connected client.
	 *
	 * @event       The event name.
	 * @args        Payload delivered to the client listener.
	 * @ackCallback Accepted for API compatibility; client→server acks for
	 *              server-initiated emits are not delivered back (documented).
	 */
	public void function emit(
		  required string event
		,          any    args = []
		,          any    ackCallback
	) {
		$sioSend( variables.id, arguments.event, arguments.args );
	}

	/**
	 * Send a direct `message` event to this connected client.
	 */
	public void function send( required string message ) {
		$sioSend( variables.id, "message", arguments.message );
	}

	/**
	 * Broadcast an event to all other sockets in the namespace (excluding this
	 * one), optionally narrowed to room(s).
	 */
	public void function broadcast(
		  required string event
		,          any    args  = []
		,          any    rooms = []
	) {
		$sioBroadcast( arguments.event, arguments.args, arguments.rooms, "", variables.id );
	}

	public void function joinRoom( required string roomName ) {
		$sioJoinRoom( variables.id, arguments.roomName );
	}

	public void function leaveRoom( required string roomName ) {
		$sioLeaveRoom( variables.id, arguments.roomName );
	}

	public void function leaveAllRooms() {
		$sioLeaveAllRooms( variables.id );
	}

	public void function disconnect( boolean close = true ) {
		$sioDisconnect( variables.id, arguments.close );
	}

	public struct function getSocketData() {
		return $sioGetData( variables.id );
	}

	public void function setSocketData( required struct data ) {
		$sioSetData( variables.id, arguments.data );
	}
}
