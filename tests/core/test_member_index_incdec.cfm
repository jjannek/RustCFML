<cfscript>
suiteBegin("Member / index increment & decrement (rvalue + statement)");

// ============================================================
// Postfix/prefix ++/-- on a MEMBER or INDEX target (obj.m, obj[k]) used as an
// rvalue previously emitted NO bytecode at all — the value never reached the
// stack, silently shifting any surrounding struct literal / arg list by one
// slot, and statement-form `x[k]++` was a no-op. Index targets (ArrayAccess)
// were unhandled entirely. These bit TestBox hard:
//   - `{ "order": this.$specOrderIndex++ }` built a key-shifted spec struct
//   - `variables.lookup[id]["total#type#"]++` never incremented (stats stuck 0)
// ============================================================

// --- statement form, struct member (dot) ---
s = { n = 5 };
s.n++;
assert("s.n++ statement", s.n, 6);
s.n--;
assert("s.n-- statement", s.n, 5);

// --- statement form, struct member (index, literal key) ---
s["n"]++;
assert("s['n']++ statement", s.n, 6);

// --- statement form, index with a VARIABLE key ---
k = "n";
s[k]++;
assert("s[k]++ statement (var key)", s.n, 7);

// --- statement form, index with an INTERPOLATED key (compile_expression_static
//     used to fall back to Null here -> wrote an empty "" key) ---
suffix = "n";
s2 = { totalN = 0 };
s2[ "total#suffix#" ]++;
assert("s2['total#suffix#']++ hits the real (case-insensitive) key", s2.totalN, 1);
assert("interpolated-key inc does not leak an empty key", structKeyExists(s2, ""), false);

// --- array index ---
arr = [10, 20];
arr[1]++;
arr[2]--;
assert("arr[1]++", arr[1], 11);
assert("arr[2]--", arr[2], 19);

// --- rvalue: postfix yields OLD, prefix yields NEW; both persist ---
s3 = { v = 5 };
old = s3.v++;
assert("postfix member rvalue returns old", old, 5);
assert("postfix member persisted", s3.v, 6);
newv = ++s3.v;
assert("prefix member rvalue returns new", newv, 7);
assert("prefix member persisted", s3.v, 7);

s4 = { v = 5 };
oldI = s4["v"]++;
assert("postfix index rvalue returns old", oldI, 5);
assert("postfix index persisted", s4.v, 6);

// --- the struct-literal shift regression: an inc/dec as a literal VALUE must
//     not corrupt the surrounding keys ---
idx = { c = 0 };
lit = {
    "a" : "AA",
    "b" : "BB",
    "order" : idx.c++,
    "d" : "DD"
};
// RustCFML preserves struct-literal key order (IndexMap); Lucee 7.0.4 reorders
// the keys here (it does not guarantee literal order with a member-inc value),
// so the key-order assert is RustCFML-only. The value/side-effect asserts below
// hold on both engines.
if (isRustCFML()) assert("struct literal keys intact with member-inc value", structKeyList(lit), "a,b,order,d");
assert("struct literal inc value is the old value", lit.order, 0);
assert("struct literal member-inc persisted on source", idx.c, 1);

// --- nested deep target (member -> index -> interpolated key), as TestBox's
//     incrementSpecStat does ---
root = { lookup = { "s1" = { totalPass = 0 } }, totalPass = 0 };
type = "pass";
root.lookup[ "s1" ][ "total#type#" ]++;
root[ "total#type#" ]++;
assert("deep nested interpolated-key inc (suite)", root.lookup.s1.totalPass, 1);
assert("deep nested interpolated-key inc (global)", root.totalPass, 1);

suiteEnd();
</cfscript>
