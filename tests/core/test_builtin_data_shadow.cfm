<cfscript>
suiteBegin("Core: data variable named like a builtin must not shadow it in call position");

// Background
// ----------
// A plain DATA variable whose name matches a built-in function must never make
// that builtin uncallable. On RustCFML 0.108.0, once a variable named `val`
// exists in the template scope, in a function's local scope, or in any LIVE
// ANCESTOR frame on the call stack, every call-position use of `Val(...)`
// throws:
//
//     Variable is not a function or function '<unknown>' is not defined
//
//   val = "29"; Val(val)
//     RustCFML 0.108.0 -> throws (template scope, function-local, bare-read
//                          argument, and cross-stack: a CALLER's `local.val`
//                          poisons the CALLEE's `Val("123")`)
//     Lucee 5.4.8.x    -> 29 everywhere; data variables never shadow builtins
//                          in call position
//
// Value-position reads of the variable keep working on both engines — only
// call-position resolution breaks, and only for the same-named builtin (other
// builtins stay callable). The poison follows frame lifetime: a function-local
// shadow clears when its frame dies, but a template-scope variable poisons the
// builtin for the rest of the request — and structDelete()ing the variable
// does NOT restore it.
//
// Sibling gap, kept disjoint: bare-name USER-function resolution through
// caller frames (a caller's data variable hiding a user-defined function from
// the callee). This suite is exclusively about BUILTINs vs data variables —
// template scope included, which the user-function gap does not reach.
//
// ORDERING NOTE: the template-scope shape runs LAST because its poison is
// request-sticky on the broken engine, and `Val` is deliberately a builtin no
// other suite in tests/runner.cfm calls — a red run cannot cascade into
// unrelated suites. The function-local shapes are self-contained (frame death
// clears them).

// --- CONTROL (green on both engines): no same-named variable exists yet, the
//     builtin is an ordinary builtin. Guards the wiring. ---
bdsCtl = {ok = false, got = ""};
try { bdsCtl.got = Val("7"); bdsCtl.ok = true; } catch (any e) { bdsCtl.err = e.message; }
assertTrue("CONTROL: Val('7') callable before any shadowing variable exists", bdsCtl.ok);
assert("CONTROL: Val('7') returns 7", bdsCtl.got, 7);

// --- 1. function-LOCAL, scoped argument: the exact Wheels $convertToString
//        shape (local.val = ...; Val(local.val)) ---
function bdsLocalShape() {
	var s = {ok = false, got = ""};
	local.val = "42";
	try { s.got = Val(local.val); s.ok = true; } catch (any e) { s.err = e.message; }
	return s;
}
bdsLocalRes = bdsLocalShape();
assertTrue("function-local: Val(local.val) callable with local.val in the frame", bdsLocalRes.ok);
assert("function-local: Val(local.val) returns 42", bdsLocalRes.got, 42);

// --- 2. function-LOCAL, bare-read argument: in Val(val) the bare `val`
//        resolves to the local DATA variable while the callee `Val` stays the
//        builtin ---
function bdsBareShape() {
	var s = {ok = false, got = ""};
	local.val = "42";
	try { s.got = Val(val); s.ok = true; } catch (any e) { s.err = e.message; }
	return s;
}
bdsBareRes = bdsBareShape();
assertTrue("function-local bare read: Val(val) callable with local.val in the frame", bdsBareRes.ok);
assert("function-local bare read: Val(val) returns 42", bdsBareRes.got, 42);

// --- 3. generality: a second builtin (`Len`) behaves the same ---
function bdsLenShape() {
	var s = {ok = false, got = ""};
	local.len = "abcde";
	try { s.got = Len(local.len); s.ok = true; } catch (any e) { s.err = e.message; }
	return s;
}
bdsLenRes = bdsLenShape();
assertTrue("generality: Len(local.len) callable with local.len in the frame", bdsLenRes.ok);
assert("generality: Len(local.len) returns 5", bdsLenRes.got, 5);

// --- 4. cross-STACK: a CALLER's local data variable must not poison the
//        CALLEE's builtin call ---
function bdsAncestor() {
	local.val = "77";
	return bdsDescendant();
}
function bdsDescendant() {
	var s = {ok = false, got = ""};
	try { s.got = Val("123"); s.ok = true; } catch (any e) { s.err = e.message; }
	return s;
}
bdsStackRes = bdsAncestor();
assertTrue("cross-stack: Val('123') callable while a caller frame holds local.val", bdsStackRes.ok);
assert("cross-stack: Val('123') returns 123", bdsStackRes.got, 123);

// --- 5. component-method shape: mirrors Wheels' Global.cfc $convertToString
//        (local.val = arguments.value; ... return Val(val);) ---
bdsObj = createObject("component", "BuiltinDataShadowFixture");
bdsCfcRes = bdsObj.convertProbe("29abc");
assertTrue("component method: Val(local.val) callable with local.val in the frame", bdsCfcRes.ok);
assert("component method: Val('29abc') returns 29", bdsCfcRes.got, 29);

// --- 6. TEMPLATE scope (LAST — see ordering note above) ---
bdsTpl = {ok = false, got = ""};
val = "29";
assert("template: value-position read of 'val' still returns the data", val, "29");
try { bdsTpl.got = Val(val); bdsTpl.ok = true; } catch (any e) { bdsTpl.err = e.message; }
assertTrue("template: Val(val) callable with template variable 'val' set", bdsTpl.ok);
assert("template: Val(val) returns 29", bdsTpl.got, 29);

// --- 7. deleting the variable must restore the builtin ---
structDelete(variables, "val");
bdsPost = {ok = false, got = ""};
try { bdsPost.got = Val("7"); bdsPost.ok = true; } catch (any e) { bdsPost.err = e.message; }
assertTrue("post-delete: Val('7') callable again after structDelete(variables, 'val')", bdsPost.ok);
assert("post-delete: Val('7') returns 7", bdsPost.got, 7);

suiteEnd();
</cfscript>
