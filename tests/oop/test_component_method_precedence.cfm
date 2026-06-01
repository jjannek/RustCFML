<cfscript>
suiteBegin("OOP: component method precedence");

service = createObject("component", "oop.MemberPrecedenceService");

assert("component delete method wins over struct helper", service.delete(id="abc"), "deleted:abc");
assert("component count method wins over struct helper", service.count(), "component-count");

// Struct helpers must never fire on a component: a helper-named call with no
// matching method must reach onMissingMethod, not structCount/structDelete.
missing = createObject("component", "oop.OnMissingHelperService");
assert("helper-named call routes to onMissingMethod (count)", missing.count(), "missing:count");
assert("helper-named call routes to onMissingMethod (delete)", missing.delete("x"), "missing:delete");
assert("helper-named call routes to onMissingMethod (keyExists)", missing.keyExists("y"), "missing:keyExists");

// With no matching method and no onMissingMethod, an undefined call throws
// (matches Lucee) rather than silently returning null or a struct-helper result.
assertThrows("undefined component method throws", function() {
    service.frobnicate();
});
assertThrows("undefined helper-named component method throws (not structKeyExists)", function() {
    service.keyExists("x");
});

suiteEnd();
</cfscript>
