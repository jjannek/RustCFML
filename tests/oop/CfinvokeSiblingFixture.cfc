component {

	// A "callback" that mutates the component's public + private scope.
	function setByCallback() {
		this.calledFlag = true;
		variables.privateFlag = "set";
	}

	// Dispatch the sibling via a no-component cfinvoke, exactly how Wheels'
	// Global.cfc $invoke() rides the attributeCollection call form when the
	// method already exists in `variables`. The callback's `this.X`/`variables.X`
	// writes must propagate back to THIS live instance.
	function fireViaAttrCollection() {
		var args = {};
		args.method = "setByCallback";
		args.returnVariable = "local.rv";
		cfinvoke(attributeCollection="#args#");
	}

	// Same, but the method is missing → must route to onMissingMethod against
	// the live instance.
	function fireMissingViaAttrCollection() {
		var args = {};
		args.method = "noSuchMethod";
		args.returnVariable = "local.rv";
		cfinvoke(attributeCollection="#args#");
	}

	function onMissingMethod(missingMethodName, missingMethodArguments) {
		this.ommName = arguments.missingMethodName;
		return true;
	}
}
