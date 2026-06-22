component accessors="true" {

	property name="title" type="string";

	// Declared return types must surface in getMetadata() on the function ref.
	public string function doString(required string exception, string eventName = "x") {
		return "ok";
	}

	void function doVoid() {}

	// No declared return type — getMetadata should omit returnType
	// (so `meta.returnType ?: "any"` resolves to "any", matching Lucee/ACF).
	function doNone() {
		return 1;
	}

}
