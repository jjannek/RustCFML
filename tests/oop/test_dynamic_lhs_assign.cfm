<cfscript>
suiteBegin("Dynamic / quoted-string LHS assignment");

// Regression: CFML (Lucee/ACF) treats a string-valued lvalue as a runtime
// scope path. `"variables.x" = v` and the interpolated `"#scope#.#prop#" = v`
// must assign scope-aware into the CURRENT frame — landing in a CFC's private
// (variables) scope, not the page scope. Surfaced booting WireBox: its
// MixerUtil.injectPropertyMixin does `"#scope#.#prop#" = value` to apply
// property/DSL injection; without this, every `inject="model:X"` silently
// no-ops. Previously the `=` after a string literal wasn't parsed as an
// assignment at all (the string and value became separate statements).

// --- literal string LHS at page scope ---
"variables.litVar" = "L1";
assert("literal string LHS sets page variable", variables.litVar, "L1");

// --- interpolated string LHS at page scope ---
scope = "variables";
prop  = "interpVar";
"#scope#.#prop#" = "I1";
assert("interpolated string LHS sets page variable", variables.interpVar, "I1");

// --- assignment returns the assigned value ---
ret = ( "variables.retVar" = "R1" );
assert("dynamic assignment yields the value", ret, "R1");

// --- inside a CFC method: must hit the component's private (variables) scope ---
o = new DynamicLhsBag();
o.put( "alpha", "A" );
o.put( "beta",  "B" );
assert("CFC dynamic LHS writes private scope (alpha)", o.read("alpha"), "A");
assert("CFC dynamic LHS writes private scope (beta)",  o.read("beta"),  "B");
// not leaked to page scope
assertFalse("CFC private write does not leak to page scope", isDefined("variables.alpha"));

// --- request scope prefix routes correctly ---
"request.dynReq" = "RQ";
assert("dynamic LHS honours request scope", request.dynReq, "RQ");

suiteEnd();
</cfscript>
