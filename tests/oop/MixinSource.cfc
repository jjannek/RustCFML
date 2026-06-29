component {

	// Mirrors Wheels' $simpleLock/$invoke helpers: a method that self-dispatches
	// a sibling by name. When this method is INTEGRATED onto another component
	// (variables[name] = source[name]) and invoked through that host, `this` /
	// `variables` must resolve to the HOST, not to MixinSource. See issue 220.
	public function caller() {
		var out = [];
		arrayAppend( out, "selfName=" & getMetadata( this ).name );
		try {
			var a = variables[ "target" ];
			arrayAppend( out, "viaVariables=" & a() );
		} catch ( any e ) {
			arrayAppend( out, "viaVariables=ERR:" & e.message );
		}
		try {
			arrayAppend( out, "viaInvoke=" & invoke( this, "target" ) );
		} catch ( any e ) {
			arrayAppend( out, "viaInvoke=ERR:" & e.message );
		}
		return arrayToList( out, " | " );
	}

	// Annotated lifecycle-style method extracted via this[item.name] (TestBox
	// pattern). Reads the host's variables scope.
	public function lifecycle() {
		return "lifecycle:" & variables.marker & ":" & getMetadata( this ).name;
	}

}
