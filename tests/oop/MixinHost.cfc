component {

	variables.marker = "HOST-VARS";

	private function target() {
		return "HOST-TARGET-OK";
	}

	// $integrateFunctions pattern: copy a method off a source component onto this
	// host, then invoke it through the host. The integrated method must run
	// against THIS host's scope.
	public function runIntegrated() {
		var src = createObject( "component", "oop.MixinSource" );
		var ref = src[ "caller" ];
		variables[ "caller" ] = ref;
		this[ "caller" ]      = ref;
		var f = variables[ "caller" ];
		return f();
	}

	// TestBox lifecycle pattern: extract a method via this[name] into a PLAIN
	// struct, then invoke it as a struct member from within this component. It
	// must bind to this host (the caller's component context).
	public function runStructDispatch() {
		var localSrc = createObject( "component", "oop.MixinSource" );
		variables[ "lifecycle" ] = localSrc[ "lifecycle" ];
		this[ "lifecycle" ]      = localSrc[ "lifecycle" ];
		var item = { name = "lifecycle" };
		var bag  = { fn = this[ item.name ] };
		return bag.fn();
	}

	// Extract one of THIS host's own methods via this[name] into a plain struct
	// and dispatch it — must still see this host's scope.
	public function runOwnStructDispatch() {
		var item = { name = "lifecycleOwn" };
		var bag  = { fn = this[ item.name ] };
		return bag.fn();
	}

	public function lifecycleOwn() {
		return "own:" & variables.marker;
	}

}
