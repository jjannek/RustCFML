<cfscript>
// Regression: Lucee/ACF/BoxLang treat a newline between function parameters as
// a soft separator and tolerate a missing comma. RustCFML's parser used to
// require the comma, which broke loading TestBox's BaseSpec.cfc (createMock).
suiteBegin("Comma-less function params");

// Missing comma between two typed params with defaults.
function f(
    boolean a = false
    boolean b = true
){ return a & "-" & b; }
assert("comma-less typed defaults", f(), "false-true");

// required keyword + mixed types, no commas.
function g(
    required string name
    numeric count = 5
){ return name & count; }
assert("comma-less with required", g("x"), "x5");

// Commas still work (no regression).
function h(string a, string b = "z"){ return a & b; }
assert("comma still works", h("p"), "pz");

suiteEnd();
</cfscript>
