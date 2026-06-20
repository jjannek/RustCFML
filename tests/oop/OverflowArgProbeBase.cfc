component {

	// Declared param is `properties` (plural) — a call passing `property`
	// (singular) makes `property` an OVERFLOW named arg.
	function takesNamed(properties = "", minimum = "", when = "") {
		// Mimic Wheels' $args by mutating the by-ref arguments struct.
		if (StructKeyExists(arguments, "property")) {
			arguments.properties = arguments.property;
		}
		return "TN";
	}

	// The in-scope function that the overflow arg name collides with.
	function property(name = "", label = "", sql = "", select = "", dataType = "") {
		return "PROP:" & arguments.name;
	}

	// Mirrors Wheels' $initModelClass calling the developer's config() bare.
	function runConfig() {
		if (StructKeyExists(variables, "config")) {
			return config();
		}
		return "no-config";
	}
}
