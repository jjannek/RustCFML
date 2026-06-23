<cfscript>
suiteBegin("local.<scopeName> as a plain local");

// `local.variables = <value>` (and local.session/request/application) must store
// an ordinary function-local literally named after the scope — NOT be swallowed
// by the scope write-back path. RustCFML's StoreLocal scope-name branches matched
// the name and then dropped the value when it wasn't a Struct, so
// `local.variables = ListToArray(...)` silently vanished — breaking Wheels'
// mapper constraint helpers (`local.variables = ListToArray(arguments.variableName)`).

function usesLocalVariables() {
	local.variables = ListToArray("a,b,c");
	local.session = ListToArray("x,y");
	local.iterations = 0;
	for (local.v in local.variables) {
		local.iterations++;
	}
	return {
		varsLen: ArrayLen(local.variables),
		sessLen: ArrayLen(local.session),
		hasVars: StructKeyExists(local, "variables"),
		iters: local.iterations
	};
}

r = usesLocalVariables();
assert("local.variables length", r.varsLen, 3);
assert("local.session length", r.sessLen, 2);
assertTrue("structKeyExists(local,'variables')", r.hasVars);
assert("for-in over local.variables iterates", r.iters, 3);

suiteEnd();
</cfscript>
