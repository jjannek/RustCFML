<cfscript>
suiteBegin("logical AND / OR short-circuit");

// Regression: AND/OR (and their script aliases &&/||) used to evaluate BOTH
// operands. That diverged from Lucee/ACF and broke real-world code that
// relies on a guarded right-hand side, e.g. WireBox's
//   if ( hasAnnotationValue( md, "cache" ) AND isBoolean( getAnnotationValue( md, "cache", "" ) ) )
// where the right-hand call is unsafe unless the left already validated the key.

// Sentinel that lets us prove whether the RHS evaluated. Use an array
// (reference type) so the running counter is visible to both the page scope
// AND the called function — a plain scalar in `variables` is not refreshed
// inside an already-defined function (separate engine quirk, out of scope here).
variables.log = [];
function trueSE()  { arrayAppend( variables.log, "T" ); return true; }
function falseSE() { arrayAppend( variables.log, "F" ); return false; }
function boom()    { arrayAppend( variables.log, "B" ); throw( type = "BoomShort", message = "rhs evaluated" ); }
function resetLog() { arrayClear( variables.log ); }

// --- AND short-circuit: false AND <anything> must NOT evaluate RHS ---
resetLog();
r1 = ( false AND trueSE() );
assert("AND: false AND x => false", r1, false);
assert("AND: RHS skipped when LHS false", arrayLen(variables.log), 0);

resetLog();
r2 = ( false && trueSE() );
assert("&&: false && x => false", r2, false);
assert("&&: RHS skipped when LHS false", arrayLen(variables.log), 0);

// --- AND: true LHS must evaluate RHS, and result is RHS-as-bool ---
resetLog();
r3 = ( true AND trueSE() );
assert("AND: true AND true => true", r3, true);
assert("AND: RHS evaluated when LHS true", arrayLen(variables.log), 1);

resetLog();
r4 = ( true AND falseSE() );
assert("AND: true AND false => false", r4, false);
assert("AND: RHS evaluated when LHS true (falsey)", arrayLen(variables.log), 1);

// --- OR short-circuit: true OR <anything> must NOT evaluate RHS ---
resetLog();
r5 = ( true OR falseSE() );
assert("OR: true OR x => true", r5, true);
assert("OR: RHS skipped when LHS true", arrayLen(variables.log), 0);

resetLog();
r6 = ( true || falseSE() );
assert("||: true || x => true", r6, true);
assert("||: RHS skipped when LHS true", arrayLen(variables.log), 0);

// --- OR: false LHS must evaluate RHS ---
resetLog();
r7 = ( false OR trueSE() );
assert("OR: false OR true => true", r7, true);
assert("OR: RHS evaluated when LHS false", arrayLen(variables.log), 1);

resetLog();
r8 = ( false OR falseSE() );
assert("OR: false OR false => false", r8, false);
assert("OR: RHS evaluated when LHS false (falsey)", arrayLen(variables.log), 1);

// --- Side-effect that throws is suppressed by the short circuit ---
caught = "";
try {
    r9 = ( false AND boom() );
} catch ( any e ) {
    caught = e.type;
}
assert("AND: throwing RHS suppressed by short-circuit", caught, "");

caught = "";
try {
    r10 = ( true OR boom() );
} catch ( any e ) {
    caught = e.type;
}
assert("OR: throwing RHS suppressed by short-circuit", caught, "");

// --- And the throw still fires when the short circuit doesn't kick in ---
caught = "";
try {
    r11 = ( true AND boom() );
} catch ( any e ) {
    caught = e.type;
}
assert("AND: throwing RHS fires when not short-circuited", caught, "BoomShort");

// --- if-context (separate code path from value-context expressions) ---
resetLog();
if ( false AND trueSE() ) { /* not reached */ }
assert("AND in if(): RHS skipped when LHS false", arrayLen(variables.log), 0);

resetLog();
if ( true OR trueSE() ) { /* taken */ }
assert("OR in if(): RHS skipped when LHS true", arrayLen(variables.log), 0);

// --- Guard pattern (the WireBox case): falsy LHS must protect an unsafe RHS ---
// `structKeyExists` guards a key lookup that would otherwise be undefined.
sample = { present = "yes" };
ok = ( structKeyExists( sample, "missing" ) AND sample.missing eq "anything" );
assert("guard pattern with AND: missing key is safe", ok, false);

// --- Chained AND: middle false must skip the tail ---
resetLog();
r12 = ( true AND falseSE() AND trueSE() );
assert("AND chain: result", r12, false);
assert("AND chain: tail skipped after middle false", arrayLen(variables.log), 1);

// --- Chained OR: middle true must skip the tail ---
resetLog();
r13 = ( false OR trueSE() OR falseSE() );
assert("OR chain: result", r13, true);
assert("OR chain: tail skipped after middle true", arrayLen(variables.log), 1);

suiteEnd();
</cfscript>
