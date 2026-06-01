<cfscript>
suiteBegin("Core: script function post-paren attribute");

// ============================================================
// Background
// ============================================================
// A script function declaration may carry metadata attributes AFTER the
// parameter list, before the body brace:  function f() output=true { ... }.
// RustCFML accepted a quoted value (output="true") but not an unquoted one;
// with output=true the body `{` was then misparsed as a struct literal
// ("Expected RBrace, found ..."). Lucee/Adobe CF/BoxLang accept it. Used in the
// wheelstest BaseSpec. (Modifiers BEFORE the name — remote function f(){} —
// already worked.)
// ============================================================

// page-level function with a single unquoted boolean attribute after ()
function f() output=true {
	return 1;
}
assert("script fn with output=true after () parses and runs", f(), 1);

// multiple attributes, mixed unquoted + quoted
function g() output=false hint="adds" {
	return 2;
}
assert("script fn with multiple post-paren attributes parses", g(), 2);

// the same shape on a CFC method (via a fixture that degrades on parse failure)
o = createObject("component", "ScriptFnAttrFixture");
assert("CFC method with a post-paren attribute parses", isObject(o), true);
assert("the attributed method runs", isObject(o) ? o.calc() : -1, 42);
assert("the rest of the component is intact", isObject(o) ? o.ping() : "NOT-A-COMPONENT", "pong");

suiteEnd();
</cfscript>
