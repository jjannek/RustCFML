<cfcomponent>
	<cfscript>
	public any function init() {
		variables.states = {};
		variables.states[ "s1" ] = new PresideFixChainState();
		variables.states[ "s2" ] = new PresideFixChainState();
		return this;
	}
	public boolean function statesIsStruct() {
		return isStruct( variables.states );
	}
	// Mirrors ColdBox InterceptorService.processState:
	//   variables.interceptionStates.find( state ).process( argumentCollection=arguments )
	public any function processState( required any state ) {
		arguments.event = "EVT";
		return variables.states.find( arguments.state ).process( argumentCollection = arguments );
	}
	</cfscript>
</cfcomponent>
