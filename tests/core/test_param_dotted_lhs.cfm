<cfscript>
suiteBegin("param shorthand with dotted / scoped LHS");

// Regression: the cfscript `param` shorthand only accepted a single bare
// identifier as the variable name. A dotted lvalue (`param arguments.obj.key =
// default`, `param s.k = v`) was misparsed — the first segment was consumed as
// the declared *type* and the parser then choked on the `.`. Because a parse
// failure inside a CFC body makes createObject() return null silently, this
// surfaced as WireBox's Injector silently failing to construct (Injector.cfc
// uses `param arguments.target.$wbDelegateMap = {}`). Lucee accepts the dotted
// shorthand, so this is a cross-engine compatibility fix.

// --- 2-level dotted: creates when undefined ---
cfgA = {};
param cfgA.timeout = 30;
assert("param cfgA.timeout creates", cfgA.timeout, 30);

// --- already defined: param leaves the existing value ---
cfgB = { timeout = 5 };
param cfgB.timeout = 30;
assert("param keeps existing value", cfgB.timeout, 5);

// --- 3-level dotted path ---
cfgD = { conn = {} };
param cfgD.conn.poolSize = 8;
assert("param 3-level dotted creates", cfgD.conn.poolSize, 8);

// --- scoped LHS (request scope) ---
param request.paramDottedProbe = "ok";
assert("param request.x scoped", request.paramDottedProbe, "ok");

// --- the WireBox shape: param on a dotted arguments path inside a function ---
function seedDelegateMap( target ) {
	param arguments.target.delegateMap = {};
	arguments.target.delegateMap[ "hit" ] = true;
	return arguments.target;
}
seeded = seedDelegateMap( {} );
assertTrue("param arguments.target.key created struct", structKeyExists(seeded, "delegateMap"));
assertTrue("…and is writable afterwards", seeded.delegateMap.hit);

// --- simple (non-dotted) shorthand still works ---
param plainParam = 7;
assert("plain param still works", plainParam, 7);

suiteEnd();
</cfscript>
