/**
 * Stand-in for coldbox.system.ioc.config.Mapping — its getName() is the
 * member call whose result the injector stores into the arguments scope.
 */
component {
	function init( required nm ) {
		variables.nm = arguments.nm;
		return this;
	}
	function getName() {
		return variables.nm;
	}
}
