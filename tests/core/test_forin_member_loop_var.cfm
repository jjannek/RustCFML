<cfscript>
suiteBegin("Core: for-in with a member-path loop variable");

// ============================================================
// Background
// ============================================================
// CFScript's for-in loop lets the loop variable be ANY assignable
// expression (lvalue), not just a bare name. Lucee 5/6/7, Adobe
// ColdFusion 2018-2025, and BoxLang all accept a struct member path as
// the loop variable:
//
//     for (ctx.item in myArray)          // array element -> ctx.item
//     for (ctx.key  in myStruct)         // struct key   -> ctx.key
//     for (this.wheels.folder in arr)    // `this`-scoped member path
//
// CFWheels/Wheels relies on the `this`-scoped form in
// public/Application.cfc, whose pseudo-constructor scans the plugin
// directory with:
//
//     for (this.wheels.folder in this.wheels.pluginFolders) { ... }
//
// RustCFML v0.20.2 only supports a BARE-NAME loop variable. A member-path
// loop variable fails in two distinct ways:
//
//   (1) A plain member path (ctx.item, a.b.c) PARSES, but the loop never
//       binds the variable and its body never runs -- the iteration is
//       silently skipped (the sum stays 0, the join stays ""). This is
//       the run-time gap the first assertions below pin down.
//
//   (2) A `this`-headed path fails to PARSE outright
//       (`Parse error: Expected Semicolon, found In`). Because Wheels'
//       Application.cfc uses exactly this form, the component degrades to
//       an empty object at instantiation and its pseudo-constructor never
//       completes. The fixture assertions below pin this down via
//       ForInThisLoopFixture (the parse failure is contained to that
//       component -- it does NOT abort this run).
//
// A bare-name loop variable -- for (item in myArray) -- works on RustCFML
// and is already covered by
// tests/compat_engine/test_language_controlflow.cfm. All assertions in
// this file PASS on Lucee/ACF/BoxLang.
// ============================================================

// ------------------------------------------------------------
// (1) Plain struct-member path, iterating an array: parses on RustCFML
//     but does not iterate.
// ------------------------------------------------------------
ctx = {arr: [10, 20, 30], sum: 0};
for (ctx.item in ctx.arr) {
    ctx.sum = ctx.sum + ctx.item;
}
assert("for (struct.member in array): sums every element",
    ctx.sum, 60);

// ------------------------------------------------------------
// (1) Plain struct-member path, iterating a struct (key iteration).
// ------------------------------------------------------------
ctx2 = {src: {a: 1, b: 2, c: 3}, keys: ""};
for (ctx2.k in ctx2.src) {
    ctx2.keys = listAppend(ctx2.keys, ctx2.k);
}
assert("for (struct.member in struct): visits every key",
    listLen(ctx2.keys), 3);

// ------------------------------------------------------------
// (2) `this`-headed path -- the exact shape Wheels' Application.cfc uses,
//     exercised through a fixture component. PARSE error on RustCFML
//     0.20.2; degrades the component so the methods return "".
// ------------------------------------------------------------
probe = createObject("component", "ForInThisLoopFixture");

assert("for (this.wheels.folder in array): the Wheels Application.cfc shape",
    probe.scanArray(), "xyz");

assert("`this`-scoped loop var holds the final element after the loop",
    probe.lastFolder(), "z");

// ------------------------------------------------------------
// (3) Member-path loop variable whose ROOT does not yet exist:
//     `for (loc.route in coll)` where `loc` is undeclared. Lucee/ACF/
//     BoxLang auto-vivify `loc` as a struct on the first iteration; the
//     manual write-back chain used to load the root first and threw
//     "Variable 'loc' is undefined". (Wheels mapperSpec wildcard tests
//     iterate `for (loc.route in application.wheels.routes)`.)
//
//     Wrapped in a function so `loc` is a fresh undeclared local.
function scanUndeclaredRoot() {
    var total = 0;
    for (loc.n in [1, 2, 3, 4]) {
        total += loc.n;
    }
    return total;
}
assert("for (undeclaredRoot.member in array): auto-vivifies the root",
    scanUndeclaredRoot(), 10);

function lastUndeclaredRoot() {
    for (loc.item in ["a", "b", "c"]) {
        // loop body intentionally empty
    }
    return loc.item;
}
assert("undeclared-root loop var holds the final element after the loop",
    lastUndeclaredRoot(), "c");

suiteEnd();
</cfscript>
