component {
	// Parent method that probes the component `variables` scope via isDefined.
	// The map it reads is set by the CHILD's pseudo-constructor, so this only
	// works if `isDefined("variables.x")` resolves to the component scope
	// (__variables), not the function-local frame.
	public string function lookup(required string key) {
		if (IsDefined("variables.sqlTypes") && StructKeyExists(variables.sqlTypes, arguments.key)) {
			return variables.sqlTypes[arguments.key];
		}
		return "MISS";
	}
	public string function probe() {
		return IsDefined("variables.sqlTypes") ? "Y" : "N";
	}
}
