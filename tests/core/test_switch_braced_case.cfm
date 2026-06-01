<cfscript>
suiteBegin("Core: compound assignment inside a braced switch/case");

// ============================================================
// Background  (parse gap surfaced in PR #32 by bpamiri)
// ============================================================
// A `case` / `default` body may be wrapped in its own braces:
//   case "x": { total += 1; break; }
// RustCFML used to misread the leading brace as a struct literal, so a
// compound-assignment statement inside it failed ("Expected RBrace, found
// PlusEqual"). The un-braced form (`case "x": total += 1;`) always worked.
// Lucee/Adobe CF/BoxLang accept both. CFML blocks introduce no scope of their
// own, so the braces are pure grouping.
// ============================================================

function classify(required string x) {
	var total = 0;
	switch (arguments.x) {
		case "a": { total += 1; break; }
		case "b": { total += 10; total += 5; break; }
		default: { total += 100; }
	}
	return total;
}

assert("braced case with a compound assignment", classify("a"), 1);
assert("braced case with multiple statements", classify("b"), 15);
assert("braced default with a compound assignment", classify("z"), 100);

// un-braced case still works (regression guard)
function classifyBare(required string x) {
	var total = 0;
	switch (arguments.x) {
		case "a": total += 7; break;
		default: total += 1;
	}
	return total;
}
assert("un-braced case with a compound assignment", classifyBare("a"), 7);

suiteEnd();
</cfscript>
