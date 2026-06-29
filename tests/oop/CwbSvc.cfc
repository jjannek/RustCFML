component {
	// Invokes a closure passed in as an UNBOUND argument (a non-CFC receiver),
	// behind this CFC-method frame. Used to exercise closure unscoped-write
	// writeback across a CFC-method boundary.
	public any function relay( required any processor ) {
		return arguments.processor( "CAPTURED" );
	}
}
