<!---
  Regression tests for issue #188 and the argumentCollection-precedence bug it
  uncovered. Both reproduced as a 97% CPU hang in the Wheels test suite:
    - cfloop `index="local.i"` (and cfscript `for (var local.i = …)`) dropped the
      loop counter's initializer, lagging and over-running; nested in Wheels'
      populate it corrupted the `local` scope and spun forever.
    - explicit named args lost to `argumentCollection` keys when they appeared
      BEFORE it at the call site — so Wheels' recursive
      `resource(name=part, argumentCollection=arguments)` never shed its
      comma-list `name` and recursed without bound.
  All assertions below pass identically on Lucee 7.
--->
<cffunction name="tagLoopTest" returntype="string">
	<cfset var c = 0><cfset var o = "">
	<cfloop from="1" to="5" index="local.i">
		<cfset c = c + 1><cfset o = o & local.i>
	</cfloop>
	<cfreturn o & "/" & c>
</cffunction>

<cfscript>
suiteBegin("Scoped loop index + argumentCollection precedence issue 188");

// 1. Tag-form cfloop from/to with a scope-qualified index.
assert("cfloop from/to index=local.i runs 1..5", tagLoopTest(), "12345/5");

// 2. cfscript C-style for with `var local.i` initializer.
function scriptVarLocal() {
	var o = "";
	var c = 0;
	for (var local.i = 1; local.i <= 5; local.i = local.i + 1) {
		c = c + 1;
		o &= local.i;
	}
	return o & "/" & c;
}
assert("for (var local.i = 1; ...) runs 1..5", scriptVarLocal(), "12345/5");

// 3. Nested scoped counters stay independent.
function nested() {
	var total = 0;
	for (var local.i = 1; local.i <= 3; local.i = local.i + 1) {
		for (var local.j = 1; local.j <= 3; local.j = local.j + 1) {
			total = total + 1;
		}
	}
	return total;
}
assert("nested for (var local.i)/(var local.j)", nested(), 9);

// 4. Explicit named args win over argumentCollection keys, regardless of order.
function pick(name, other) {
	return arguments.name;
}
ac = {name: "FROM_COLL", other: "x"};
assert("explicit name BEFORE argumentCollection wins", pick(name = "EXPLICIT", argumentCollection = ac), "EXPLICIT");
assert("explicit name AFTER argumentCollection wins", pick(argumentCollection = ac, name = "EXPLICIT2"), "EXPLICIT2");

// 5. The exact Wheels resource() recursion shape: an explicit `name` must
//    override the comma-list `name` carried in argumentCollection, so the
//    recursion terminates (1 list call + 3 leaf calls = 4).
request._expandCalls = 0;
function expand(name) {
	request._expandCalls = request._expandCalls + 1;
	if (request._expandCalls > 100) {
		return "RUNAWAY";
	}
	if (Find(",", arguments.name)) {
		var parts = ListToArray(arguments.name);
		for (var i = 1; i <= ArrayLen(parts); i = i + 1) {
			expand(name = parts[i], argumentCollection = arguments);
		}
		return "expanded";
	}
	return "leaf:" & arguments.name;
}
expand("a,b,c");
assert("recursive expand terminates (explicit name overrides argColl list)", request._expandCalls, 4);

suiteEnd();
</cfscript>
