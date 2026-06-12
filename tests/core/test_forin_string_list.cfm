<cfscript>
suiteBegin("Core: for-in over a string iterates comma-list ITEMS, not characters");

// ============================================================
// Background
// ============================================================
// In CFML, a for-in loop over a STRING treats the string as a
// comma-delimited LIST and iterates its items:
//
//     for (col in "id,title,body")
//     // Lucee:    "id", "title", "body"            (3 iterations)
//     // RustCFML: "i","d",",","t","i","t","l","e",
//     //           ",","b","o","d","y"              (13 iterations,
//     //                                             commas included)
//
// The exact list contract, pinned empirically on Lucee 5.4.8.2:
//
//   - the delimiter is the comma and ONLY the comma -- "a;b;c" is a
//     single item, the whole string;
//   - items are NOT trimmed -- "a, b , c" yields "a", " b ", " c";
//   - empty items are KEPT -- "a,,b" is 3 items ("a", "", "b") and
//     "a,b," is 3 items ("a", "b", ""); note this DIFFERS from
//     ListToArray's default, which skips empty items;
//   - a string with no commas is ONE iteration of the whole string;
//   - an empty string never enters the loop (zero iterations).
//
// RustCFML 0.108.0 instead iterates the string CHARACTER BY CHARACTER
// (including the comma characters themselves). Controls that pass on
// both engines: for-in over ListToArray() of the same string, and
// for-in over the empty string.
//
// Why it matters for Wheels: $queryRowToStruct() in
// vendor/wheels/model/serialize.cfc hydrates EVERY finder result row
// into a model object via
//
//     for (local.column in arguments.properties.columnList) { ... }
//
// where columnList is the query's comma-delimited column-name string.
// On RustCFML each model object built from a finder grew junk
// single-character property keys (I, D, ",", T, L, E, ...) instead of
// its real column properties -- findByKey()/findAll() returned garbage
// objects for every model. All assertions below PASS on Lucee.
// ============================================================

// Helpers (fsl-prefixed: the runner shares one template scope).
function fslJoin(required subject) {
	var seen = [];
	for (var fslItem in arguments.subject) {
		ArrayAppend(seen, fslItem);
	}
	return ArrayToList(seen, "|");
}
function fslCount(required subject) {
	var seen = [];
	for (var fslItem in arguments.subject) {
		ArrayAppend(seen, fslItem);
	}
	return ArrayLen(seen);
}

// ------------------------------------------------------------
// (1) The Wheels shape: for-in over a comma-delimited column-list
//     string iterates the LIST ITEMS.
// ------------------------------------------------------------
assert("for (col in 'id,title,body'): 3 list items, not 13 characters",
	fslCount("id,title,body"), 3);
assert("for (col in 'id,title,body'): the items are the fields",
	fslJoin("id,title,body"), "id|title|body");

// ------------------------------------------------------------
// (2) Comma is the ONLY delimiter -- a semicolon string is one item.
// ------------------------------------------------------------
assert("for-in over 'a;b;c' is a single item (comma is the only delimiter)",
	fslJoin("a;b;c"), "a;b;c");

// ------------------------------------------------------------
// (3) Items are not trimmed.
// ------------------------------------------------------------
assert("items keep their surrounding spaces",
	fslJoin("a, b , c"), "a| b | c");

// ------------------------------------------------------------
// (4) A string with no commas is ONE iteration of the whole string.
// ------------------------------------------------------------
assert("for-in over 'title' is one item, not five characters",
	fslJoin("title"), "title");
assert("for-in over 'title' iterates exactly once",
	fslCount("title"), 1);

// ------------------------------------------------------------
// (5) Empty items are kept (Lucee contract -- differs from
//     ListToArray's default empty-skipping).
// ------------------------------------------------------------
assert("'a,,b' is 3 items -- the empty middle item is kept",
	fslCount("a,,b"), 3);
assert("'a,b,' is 3 items -- the empty trailing item is kept",
	fslCount("a,b,"), 3);

// ------------------------------------------------------------
// (6) CONTROL: an empty string never enters the loop (passes on
//     both engines).
// ------------------------------------------------------------
assert("for-in over '' is zero iterations",
	fslCount(""), 0);

// ------------------------------------------------------------
// (7) CONTROL: for-in over ListToArray() of the same string (passes
//     on both engines) -- the workaround Wheels code would not need
//     on a compliant engine.
// ------------------------------------------------------------
assert("control: for-in over ListToArray('id,title,body')",
	fslJoin(ListToArray("id,title,body")), "id|title|body");

// ------------------------------------------------------------
// (8) End-to-end Wheels shape: a real query's columnList string.
//     (Count, not join -- engines differ on columnList letter case,
//     which is not the contract under test.)
// ------------------------------------------------------------
fslQ = QueryNew("id,title,body", "integer,varchar,varchar", [[1, "Hello", "World"]]);
assert("for (col in query.columnList) yields one item per column",
	fslCount(fslQ.columnList), 3);

suiteEnd();
</cfscript>
