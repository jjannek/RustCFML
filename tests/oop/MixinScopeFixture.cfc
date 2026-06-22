component {

	function init() {
		return this;
	}

	// Mirror the Wheels plugin-loader injection (Plugins.cfc $initializeMixins):
	// StructAppend a struct of function references into the component's PRIVATE
	// `variables` scope, then — on engines that expose the live `variables.this`
	// alias — into the public scope too. RustCFML has no `variables.this`, so the
	// mixed-in function lands only in `__variables`; member dispatch must still
	// resolve `obj.$mixedIn()`. Lucee/ACF reach the same callable result via the
	// `variables.this` append. Either way the method is invocable externally.
	function injectMixins(required struct mixins) {
		StructAppend(variables, arguments.mixins, true);
		if (StructKeyExists(variables, "this")) {
			StructAppend(variables.this, arguments.mixins, true);
		}
	}

	// Bare in-method call of a mixed-in method (Wheels `$callback` style).
	function callMixedInBare() {
		return $mixedIn();
	}
}
