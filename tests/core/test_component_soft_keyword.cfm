<cfscript>
suiteBegin("Core: `component` is a soft keyword");

// ============================================================
// Background
// ============================================================
// `component` is a SOFT keyword on Lucee 5/6/7, Adobe ColdFusion 2018-2025, and
// BoxLang: it introduces a CFC ONLY when it begins a declaration
// (`component { ... }`, `component Name ...`, `component output="false" ...`).
// Used anywhere else it is an ordinary identifier — legal as a variable name,
// a struct key, a member name, and as the `component` attribute of cfinvoke.
//
// RustCFML used to treat `component` as a HARD reserved keyword: the parser
// committed to a `component { ... }` declaration the moment it saw the token, so
// any `component` followed by `=` (a bare assignment, or an attribute) failed to
// PARSE with "Expected LBrace, found Equal". These tests pin the soft-keyword
// behavior and guard that genuine declarations still parse.
// ============================================================

// --- `component` as an ordinary variable ---------------------------------

component = "widget";
assert("bare `component = x` assignment then read", component, "widget");

component = component & "-2";
assert("`component` usable on both sides of an assignment", component, "widget-2");

// --- `component` as a struct key -----------------------------------------

s = { component = "from-struct-literal" };
assert("`component` as a struct-literal key", s.component, "from-struct-literal");

s.component = "via-member-set";
assert("`component` as a struct member assignment target", s.component, "via-member-set");

s["component"] = "via-bracket-set";
assert("`component` reachable via bracket access", s.component, "via-bracket-set");

// --- `component` in expressions ------------------------------------------

component = 3;
assert("`component` participates in arithmetic", component * 2, 6);

// --- genuine declarations still parse (regression guard) -----------------

obj = createObject("component", "ComponentKeywordFixture");
assert("a real component declaration with a metadata-attr header still parses & runs",
	obj.ping(), "pong");

newObj = new ComponentKeywordFixture();
assert("new on a real component declaration still works", newObj.ping(), "pong");

suiteEnd();
</cfscript>
