/**
 * Stand-in for coldbox.system.core.dynamic.MixerUtil — start() runtime-injects
 * a struct of function refs onto the target via structAppend(target,...,true),
 * called positionally as `variables.mixerUtil.start( arguments.target )`.
 */
component {
	function init() {
		variables.mixins                  = {};
		variables.mixins[ "$wbMixer" ]    = true;
		variables.mixins.injectedMixin    = function( required value ) {
			return "INJECTED:" & arguments.value;
		};
		return this;
	}

	function start( required target ) {
		if ( !structKeyExists( arguments.target, "$wbMixer" ) ) {
			structAppend( arguments.target, variables.mixins, true );
		}
		return arguments.target;
	}
}
