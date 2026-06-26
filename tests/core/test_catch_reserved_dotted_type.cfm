<cfscript>
suiteBegin("Core: reserved words in dotted catch type (issue 203)");

// ============================================================
// A dotted exception type in a catch clause may contain CFML reserved words
// as segments — `catch( a.not.b e )`, `catch( a.eq.b e )`. RustCFML used to
// reject these ("Expected identifier (found Dot)") because the catch-type
// loop only accepted plain identifiers between the dots, while
// extract_property_name (used everywhere else) accepts the full reserved-word
// set. Real-world impact: Preside's WebflowInstanceService.cfc has
// `catch( cfflow.workflow.does.not.exist e )`. Runs fine on Lucee/ACF/BoxLang.
// ============================================================

// The exact Preside case: reserved words `does`, `not`, `exist` mid-path.
r1 = "";
try {
	throw(type = "cfflow.workflow.does.not.exist", message = "wf");
} catch( cfflow.workflow.does.not.exist e ) {
	r1 = "caught-" & e.message;
}
assert("dotted type with reserved-word segments parses and matches", r1, "caught-wf");

// `not` as a segment
r2 = "";
try { throw(type = "a.not.b", message = "n"); }
catch( a.not.b e ) { r2 = e.message; }
assert("a.not.b", r2, "n");

// `eq` as a segment
r3 = "";
try { throw(type = "a.eq.b", message = "q"); }
catch( a.eq.b e ) { r3 = e.message; }
assert("a.eq.b", r3, "q");

// Non-reserved dotted type still works (regression guard)
r4 = "";
try { throw(type = "a.b.c", message = "c"); }
catch( a.b.c e ) { r4 = e.message; }
assert("a.b.c still works", r4, "c");

// Bare type and var-only forms unaffected
r5 = "";
try { throw(message = "p"); }
catch( any e ) { r5 = e.message; }
assert("catch(any e) unaffected", r5, "p");

suiteEnd();
</cfscript>
