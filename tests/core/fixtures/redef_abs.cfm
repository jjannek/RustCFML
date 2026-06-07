<cfscript>
// Fixture for test_builtin_shadowing.cfm — must NOT load standalone.
// Lucee 7 refuses to even compile this ("The name [abs] is already used
// by a built in Function"); RustCFML matches by throwing the same error
// the first time the DefineFunction op executes.
function abs(x) { return 999; }
</cfscript>
