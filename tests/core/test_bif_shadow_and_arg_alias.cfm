<cfscript>
suiteBegin("Builtin shadowing by CFC method + arguments<->param aliasing");

// --- BIF shadowing: a method named like a builtin must NOT win a bare call ---
fix = createObject("component", "BifShadowFixture");
assert("bare BIF call inside same-named method hits the BIF (valid json)",
    fix.isJSON('{"a":1}'), "valid");
assert("bare BIF call inside same-named method hits the BIF (invalid json)",
    fix.isJSON('not json'), "invalid");
assert("this.method() still reaches the component method",
    fix.callViaThis(), "valid");

// --- arguments.X writes alias the bare param X (CFML scope semantics) ---
function argScalarAssign( idx = 1 ) {
    arguments.idx = 99;
    return arguments.idx & "|" & idx;   // both must read 99
}
assert("arguments.X = v updates bare X", argScalarAssign(), "99|99");

function argInc( idx = 1 ) {
    var picked = ++arguments.idx;
    return arguments.idx & "|" & idx & "|" & picked;
}
assert("++arguments.X updates bare X", argInc(), "2|2|2");

// --- but an EXPLICIT local declaration keeps the two views separate ---
function localShadows( params = "" ) {
    local.params = { controller = "x" };
    arguments.params = "ARGWRITE";
    return isStruct(local.params) & "|" & arguments.params;
}
assert("explicit local.X is NOT clobbered by an arguments.X write",
    localShadows(), "true|ARGWRITE");

suiteEnd();
</cfscript>
