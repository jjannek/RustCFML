<cfscript>
// A `local`-scoped variable named `arguments` is an ordinary local struct
// (Lucee parity). Credit: Blute (PR #116).
//
// `local.arguments` is a variable in the `local` scope whose name happens to
// be "arguments" — it is NOT the `arguments` scope. Assigning and reading it
// must behave like any other local struct: the keys written to it are the
// keys it has.
//
// Real-world hit: a Moopa route dispatcher (a custom tag, no declared
// arguments) builds a working struct `local.arguments = {}`, fills it with
// the matched route params, and passes it to the endpoint. When the engine
// treats `local.arguments` as the `arguments` scope, the struct reads back
// EMPTY, the route params are lost, and every parameterised route 500s with
// "No ID provided".
//
// NOTE: assertions are key-ORDER-insensitive — Lucee structs are unordered, so
// only key membership and values are portable across engines.

suiteBegin("Core: local-scoped 'arguments' variable behaves as an ordinary struct");

o = createObject("component", "LocalArgumentsVarFixture");
r = o.build();

assertTrue("local.arguments keeps key 'route' (got: [" & r & "])",
	find("route", r) GT 0);
assertTrue("local.arguments keeps key 'track_id' (got: [" & r & "])",
	find("track_id", r) GT 0);
assertTrue("local.arguments keeps key 'extra' (got: [" & r & "])",
	find("extra", r) GT 0);

assertTrue("local.arguments values are readable (got: [" & r & "])",
	find("track_id=[THE-ID]", r) GT 0);

assertTrue("structAppend into local.arguments works (got: [" & r & "])",
	find("extra=[Z]", r) GT 0);

// Shadowing local.arguments must NOT poison the function's declared params:
// the arguments scope and the local var are independent (Lucee + BoxLang).
r2 = o.buildWithArgs("THE-ID");

assertTrue("declared param survives local.arguments shadow (got: [" & r2 & "])",
	find("argId=[THE-ID]", r2) GT 0);
assertTrue("arguments scope keeps only the real param (got: [" & r2 & "])",
	find("argCount=[1]", r2) GT 0);
assertTrue("local.arguments stays independent of arguments scope (got: [" & r2 & "])",
	find("localKeys=[route]", r2) GT 0);

suiteEnd();
</cfscript>
