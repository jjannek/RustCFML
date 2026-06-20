component {
	// A bound method passed as a value and invoked via a non-component receiver
	// (`arguments.fn(x)`) must run with THIS component's variables — the ColdBox
	// Binder.mapDirectory(filter=this._filterServices) shape.
	public any function run() {
		variables.svc = "bound-svc";
		return _invoker( this._filter );
	}
	private any function _invoker( required any fn ) {
		return arguments.fn( "x" );
	}
	private any function _filter( objectPath ) {
		return "svc=" & svc & " arg=" & arguments.objectPath;
	}
}
