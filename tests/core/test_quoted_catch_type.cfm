<cfscript>
suiteBegin("Core: quoted string catch type");

// ============================================================
// Background  (adapted from PR #32 by bpamiri)
// ============================================================
// A try/catch clause may name the exception type either as a bare identifier
// (`catch (any e)`, `catch (Application e)`), a dotted identifier
// (`catch (FW1.AbortControllerException e)`), or a QUOTED STRING literal
// (`catch ("My.Custom.Type" e)`). The quoted form is how CFML catches a dotted,
// namespaced custom exception, and it is accepted on Lucee 5/6/7, Adobe CF
// 2018-2025, and BoxLang. RustCFML used to reject it ("Expected identifier,
// found String(...)"). On the Wheels boot path:
// vendor/wheels/Public.cfc does `catch ("Wheels.Packages.RegistryUnavailable" e)`.
// ============================================================

function loadProbe(required string name) {
	o = createObject("component", arguments.name);
	return isObject(o) ? o.probe() : "NOT-A-COMPONENT";
}

assert("a quoted string catch type parses and catches the thrown type", loadProbe("QuotedCatchFixture"), "caught");

// Direct (non-fixture) form: a quoted catch type matches the thrown type.
result = "";
try {
	throw(type = "My.Custom.Type", message = "x");
} catch ("My.Custom.Type" e) {
	result = "caught-" & e.message;
}
assert("inline quoted catch type binds the exception", result, "caught-x");

suiteEnd();
</cfscript>
