// Fixture: a function whose parameters are named with the soft keywords
// `extends` and `implements`. On Lucee/Adobe CF/BoxLang these are legal
// parameter names (and reachable via the arguments scope and as bare names).
// Mirrors vendor/wheels/wheelstest/system/mockutils/MockGenerator.cfc:
//   function generateClass( string extends="", string implements="" ) { ... }
// (Originally from PR #32 by bpamiri.)
component {
	public string function gen(string extends = "", string implements = "") {
		return arguments.extends & "/" & arguments.implements;
	}
	public string function probe() {
		return gen("a", "b");
	}
}
