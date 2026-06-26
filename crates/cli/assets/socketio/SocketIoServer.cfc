/**
 * socket.io-lucee compatibility entrypoint, ported to RustCFML.
 *
 * Faithful to pixl8/socket.io-lucee's `SocketIoServer`, but the embedded Java
 * server is replaced by RustCFML's built-in `/socket.io/` transport: there is
 * nothing to start/stop, so the lifecycle methods are no-ops and the server is
 * always "RUNNING". Namespace + handler registration goes to the engine via the
 * flat `$sio*` BIFs; fan-out rides the same WebSocketRegistry as the fluent API.
 *
 * Store the instance somewhere long-lived (typically application scope) and
 * register your namespaces/handlers once, e.g. in onApplicationStart:
 *
 *   application.io = new SocketIoServer();
 *   application.io.of( "/chat" ).on( "connect", function( socket ){
 *       socket.on( "say", function( msg ){ socket.broadcast( "said", msg ); } );
 *   } );
 */
component {

	variables._namespaces = {};
	this.sockets = ""; // alias for the root namespace, set in init()

	/**
	 * The host/port/cors/ping arguments are accepted for socket.io-lucee API
	 * compatibility but ignored — the transport is RustCFML's own server.
	 */
	public any function init(
		  string  host                     = ""
		, any     port                     = 3000
		, boolean enableCorsHandling       = false
		, numeric pingInterval             = 5000
		, numeric pingTimeout              = 25000
		, numeric maxTimeoutThreadPoolSize = 20
		, array   allowedCorsOrigins       = [ "*" ]
		, boolean start                    = true
	) {
		this.sockets = this.of( "/" );
		return this;
	}

	/**
	 * Get (registering on first use) a namespace object.
	 */
	public any function of( required string namespace ) {
		return this.namespace( arguments.namespace );
	}

	/**
	 * Slightly less weirdly named alias of `of`.
	 */
	public any function namespace( required string namespace ) {
		$sioRegisterNamespace( arguments.namespace );
		if ( !structKeyExists( variables._namespaces, arguments.namespace ) ) {
			variables._namespaces[ arguments.namespace ] = new SocketIoNamespace( arguments.namespace );
		}
		return variables._namespaces[ arguments.namespace ];
	}

	/**
	 * Register an event listener on the default ("/") namespace.
	 */
	public void function on( required string event, required any callback ) {
		this.of( "/" ).on( argumentCollection=arguments );
	}

	/**
	 * Names of every registered namespace.
	 */
	public array function getRegisteredNamespaces() {
		return $sioRegisteredNamespaces();
	}

	// Lifecycle — no-ops: the transport is the engine's own server.
	public void function start() {}
	public void function stop() {}
	public void function close() { this.stop(); }
	public void function shutdown() { this.stop(); }
	public boolean function isRunning() { return true; }
	public string function getState() { return "RUNNING"; }
}
