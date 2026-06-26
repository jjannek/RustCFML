/**
 * A socket.io namespace (socket.io-lucee compat). Register `connect` /
 * `disconnect` / `disconnecting` listeners with `on`, and broadcast to all
 * connected sockets (optionally narrowed to room(s)).
 */
component {

	public any function init( required string name ) {
		variables.name = arguments.name;
		return this;
	}

	public string function getName() {
		return variables.name;
	}

	/**
	 * Register a namespace listener. Supported events: connect, disconnect,
	 * disconnecting. The callback receives a socket object as its sole argument.
	 */
	public void function on( required string event, required any callback ) {
		$sioRegisterNsHandler( variables.name, arguments.event, arguments.callback );
	}

	/**
	 * Broadcast an event to all sockets in the namespace, or — when `rooms` is
	 * given — to the occupants of those room(s).
	 *
	 * @event The event name.
	 * @args  Payload delivered to the client listener.
	 * @rooms A single room name or array of room names (empty = whole namespace).
	 */
	public void function broadcast(
		  required string event
		,          any    args  = []
		,          any    rooms = []
	) {
		$sioBroadcast( arguments.event, arguments.args, arguments.rooms, variables.name, "" );
	}

	/**
	 * Alias of `broadcast`.
	 */
	public void function emit() {
		broadcast( argumentCollection=arguments );
	}

	/**
	 * Number of connected sockets in this namespace.
	 */
	public numeric function getSocketCount() {
		return $sioSocketCount( variables.name );
	}
}
