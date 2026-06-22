component extends="IsDefinedVarsParent" {
	// Pseudo-constructor sets the map in the shared component `variables` scope.
	variables.sqlTypes = {};
	variables.sqlTypes['date'] = "TEXT";
	variables.sqlTypes['integer'] = "INTEGER";

	// A child-defined method must also see it via isDefined.
	public string function childProbe() {
		return IsDefined("variables.sqlTypes") ? "Y" : "N";
	}
}
