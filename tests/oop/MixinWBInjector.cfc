/**
 * Stand-in for coldbox.system.ioc.Injector.autowire(), reduced to the exact
 * shape that loses runtime-injected mixins on the WireBox singleton path:
 *   1. alias the target arg into a local           (var targetObject = arguments.target)
 *   2. store a member-CALL result into the arguments scope
 *                                                   (arguments.targetID = arguments.mapping.getName())
 *   3. inject mixins in place via a positional method param
 *                                                   (variables.mixerUtil.start( arguments.target ))
 *   4. use the target through the alias             (targetObject.injectedMixin())
 */
component {
	function init() {
		variables.mixerUtil = new MixinWBHelper();
		return this;
	}

	// targetID omitted -> the member-call self-assign runs (the singleton path)
	function autowire( required target, required mapping, targetID = "" ) {
		var targetObject = arguments.target;
		if ( NOT len( arguments.targetID ) ) {
			arguments.targetID = arguments.mapping.getName();
		}
		variables.mixerUtil.start( arguments.target );
		// downstream wiring uses the alias, exactly like coldbox autowire does
		return structKeyExists( targetObject, "injectedMixin" );
	}

	// end-to-end: the injected mixin must actually be invokable
	function autowireAndCall( required target, required mapping, targetID = "" ) {
		if ( NOT len( arguments.targetID ) ) {
			arguments.targetID = arguments.mapping.getName();
		}
		variables.mixerUtil.start( arguments.target );
		return arguments.target.injectedMixin( "hi" );
	}

	// a brand-new arguments key (not a declared param) triggers the same path
	function autowireNewKey( required target, required mapping ) {
		arguments.somethingNew = arguments.mapping.getName();
		variables.mixerUtil.start( arguments.target );
		return structKeyExists( arguments.target, "injectedMixin" );
	}

	// the helper reference held in `variables` must survive the arguments store
	function mixerSurvives( required target, required mapping ) {
		arguments.targetID = arguments.mapping.getName();
		return isObject( variables.mixerUtil );
	}
}
