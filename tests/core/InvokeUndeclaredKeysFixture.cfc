// Fixture for the invoke()-undeclared-keys test. Each probe reports which
// keys actually reached its arguments scope.
component {

	// Paramless target: every key in the argument struct is "undeclared".
	public string function paramless() {
		return "hasX=" & StructKeyExists(arguments, "x")
			& "|hasLocked=" & StructKeyExists(arguments, "$locked");
	}

	// Declared-param target: x is declared, $locked is not.
	public string function declared(string x = "(default)") {
		return "x=" & arguments.x
			& "|hasLocked=" & StructKeyExists(arguments, "$locked");
	}

	// Mirrors the re-entry guard shape of Wheels' $readFlash()/$simpleLock().
	public string function guarded() {
		if (!StructKeyExists(arguments, "$locked")) {
			return "NOT-LOCKED";
		}
		return "LOCKED-OK";
	}

	// Tag-form probes: <cfinvoke> attribute names cannot contain `$` (Lucee
	// parse-rejects it), so these report a plain undeclared key `extra`.
	public string function declaredPlain(string x = "(default)") {
		return "x=" & arguments.x
			& "|hasExtra=" & StructKeyExists(arguments, "extra");
	}

	public string function paramlessPlain() {
		return "hasX=" & StructKeyExists(arguments, "x")
			& "|hasExtra=" & StructKeyExists(arguments, "extra");
	}

	// In-context dynamic dispatch control: this[name](argumentCollection = st).
	public string function callViaThisBracket(required string m, required struct a) {
		return this[arguments.m](argumentCollection = arguments.a);
	}

}
