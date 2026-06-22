<cfscript>
suiteBegin("Core: continue inside switch continues the enclosing loop");

// ============================================================
// Background (GH #195, surfaced porting Preside DataExportService)
// ============================================================
// In CFML a `switch` has no loop semantics of its own, so a `continue` inside
// a switch continues the *enclosing loop*. RustCFML previously captured the
// continue in the switch's own break-frame and discarded it, leaving an
// unpatched Jump(0) that re-ran the function -> 100% CPU infinite loop.
// `break` inside a switch still exits only the switch (C-style). Expectations
// verified against Lucee 7.
// ============================================================

// for-in: p=1 continues, p=2,3 reach n++
function forInCase() {
	var n = 0;
	for (p in [1, 2, 3]) {
		switch (p) { case 1: continue; break; }
		n++;
	}
	return n;
}
assert("continue in switch (for-in) continues the loop", forInCase(), 2);

// C-style for: i=1 continues, i=2,3 reach n++; the stride must still run
function cStyleFor() {
	var n = 0;
	for (i = 1; i <= 3; i++) {
		switch (i) { case 1: continue; break; }
		n++;
	}
	return n;
}
assert("continue in switch (C-style for) continues the loop", cStyleFor(), 2);

// while: i=1 continues, i=2,3 reach n++ (i incremented before the switch)
function whileLoop() {
	var n = 0;
	var i = 0;
	while (i < 3) {
		i++;
		switch (i) { case 1: continue; break; }
		n++;
	}
	return n;
}
assert("continue in switch (while) continues the loop", whileLoop(), 2);

// Multiple stacked labels all continue (the Preside shape).
function stackedContinue() {
	var kept = [];
	for (v in ["one-to-many", "keep", "many-to-many", "select-data-view", "alsoKeep"]) {
		switch (v) {
			case "one-to-many":
			case "many-to-many":
			case "select-data-view":
				continue;
			break;
			default:
				arrayAppend(kept, v);
		}
	}
	return kept.toList();
}
assert("stacked-label continue skips, others kept", stackedContinue(), "keep,alsoKeep");

// `break` inside switch exits only the switch, NOT the loop.
function breakStaysInSwitch() {
	var n = 0;
	for (p in [1, 2, 3]) {
		switch (p) { case 2: break; default: n++; }
	}
	return n;
}
assert("break in switch exits switch only, loop continues", breakStaysInSwitch(), 2);

// Nested loops: continue inside switch targets the INNER loop only.
function nestedLoops() {
	var n = 0;
	for (a in [1, 2]) {
		for (b in [1, 2, 3]) {
			switch (b) { case 1: continue; break; }
			n++;
		}
	}
	return n;
}
assert("continue in switch targets innermost loop", nestedLoops(), 4);

suiteEnd();
</cfscript>
