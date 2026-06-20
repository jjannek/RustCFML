component accessors="true" {
	property name="appRootPath";

	// Bare `appRootPath` (line below) must resolve to the param even when this
	// init is reached via a paramless child calling
	// `super.init(argumentCollection=arguments)` with numeric (positional) keys.
	function init( required appRootPath, appKey = "cbController" ) {
		if ( NOT reFind( "(/|\\)$", arguments.appRootPath ) ) {
			arguments.appRootPath = appRootPath & "/";
		}
		variables.appRootPath = arguments.appRootPath;
		return this;
	}
}
