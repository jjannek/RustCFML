<cfcomponent>
	<cfscript>
	// A looked-up element returned by `states.find( key )`. Its method is
	// non-mutating and returns a value (not `this`).
	public string function process( required any event ) {
		return "processed:" & arguments.event;
	}
	</cfscript>
</cfcomponent>
