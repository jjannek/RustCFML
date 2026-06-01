<cfscript>
suiteBegin("Core: dotted FQN parameter type");

// ============================================================
// Background
// ============================================================
// A function/method parameter may carry a dotted fully-qualified type name
// (`function f( required wheels.system.TestResult x )`). A single-token type
// (`string`, `Foo`) always parsed; the dotted form broke at the first `.`
// ("Expected RParen, found Dot"). The wheelstest (TestBox-derived) runners use
// it throughout: `required wheels.wheelstest.system.TestResult results`.
//
// The type is only an ANNOTATION. Lucee enforces it at CALL time (passing a
// mismatched value throws), so to stay cross-engine these tests assert that the
// declaration PARSES — never that a mismatched call succeeds.
//
// Gap-D shape lives in a fixture (a parse error would escape try/catch); via
// createObject an unparseable fixture degrades to a non-object.
// ============================================================

// A page-level function with a dotted FQN param type. Merely reaching the
// assertion below proves the declaration parsed (a parse error would abort the
// whole file before printSummary).
function inlineDotted( required a.b.C value ) {
	return "declared";
}
assert("a function with a dotted FQN param type parses inline", "reached", "reached");

// The same shape inside a CFC: if the dotted param type failed to parse, the
// component degrades to a non-object.
o = createObject("component", "DottedParamTypeFixture");
assert("a CFC method with a dotted FQN param type parses", isObject(o), true);
assert("the rest of the component is intact", isObject(o) ? o.ping() : "NOT-A-COMPONENT", "pong");

suiteEnd();
</cfscript>
