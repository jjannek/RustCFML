component extends="OverflowArgProbeBase" {

	function config() {
		// `property` is an overflow named arg here (param is `properties`).
		takesNamed(property = "password", minimum = "4", when = "onUpdate");
		// Bare call must still resolve to the inherited property() function,
		// not the leaked string "password".
		return property(name = "salesTotal", sql = "x", select = false, dataType = "int");
	}
}
