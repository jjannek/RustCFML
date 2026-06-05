<cfscript>
suiteBegin("Core: switch/case C-style fall-through");

// ============================================================
// Background  (surfaced porting WireBox onto RustCFML)
// ============================================================
// CFML `switch` is C-style: matching a case transfers control to its body and
// execution FALLS THROUGH into subsequent case bodies until an explicit
// `break`. RustCFML previously emitted an implicit jump-to-end after EVERY
// case body (auto-break), so:
//   - stacked empty labels (`case "a": case "b": { ... }`) did NOT share the
//     following body — `case "a"` matched, ran nothing, and exited; and
//   - a non-empty case without `break` did NOT continue into the next case.
// This broke ColdBox/WireBox's `Builder.cfc`, where the DSL dispatch relies on
// `case "model": case "id": { ... }` stacked labels.
// Verified against Lucee 7 (the reference engine) — all expectations below
// match Lucee's output.
// ============================================================

// Stacked empty labels share the next non-empty body.
function stacked(required string v) {
	var r = "";
	switch (arguments.v) {
		case "a":
		case "b": { r = "ab"; break; }
		default: { r = "d"; }
	}
	return r;
}
assert("stacked empty label 'a' falls through to shared body", stacked("a"), "ab");
assert("stacked label 'b' runs the shared body", stacked("b"), "ab");
assert("no match runs default", stacked("z"), "d");

// A non-empty case WITHOUT break falls through into the next case body.
function fallThrough(required string v) {
	var r = "";
	switch (arguments.v) {
		case "a": { r &= "A"; }
		case "b": { r &= "B"; break; }
		default: { r &= "D"; }
	}
	return r;
}
assert("non-empty case 'a' without break falls into 'b'", fallThrough("a"), "AB");
assert("case 'b' with break stops", fallThrough("b"), "B");
assert("no match runs default only", fallThrough("z"), "D");

// `break` exits the switch (no fall-through past it).
function withBreaks(required string v) {
	var r = "";
	switch (arguments.v) {
		case "a": { r = "A"; break; }
		case "b": { r = "B"; break; }
	}
	return r;
}
assert("break exits after case 'a'", withBreaks("a"), "A");
assert("break exits after case 'b'", withBreaks("b"), "B");

// Numeric switch fall-through.
function numeric(required numeric n) {
	var r = "";
	switch (arguments.n) {
		case 1:
		case 2: { r = "low"; break; }
		case 3: { r = "three"; }
		case 4: { r &= "four"; break; }
		default: { r = "other"; }
	}
	return r;
}
assert("numeric stacked label 1", numeric(1), "low");
assert("numeric stacked label 2", numeric(2), "low");
assert("numeric case 3 falls into 4", numeric(3), "threefour");
assert("numeric case 4 stops", numeric(4), "four");
assert("numeric default", numeric(9), "other");

suiteEnd();
</cfscript>
