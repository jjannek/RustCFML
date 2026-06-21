<cfscript>
suiteBegin("Core: unscoped >=2-level nested write auto-vivifies (copies.request.cgi = v)");

// ============================================================
// Background
// ============================================================
// An unscoped, UNDECLARED container written >=2 levels deep
// (`copies.request.cgi = request.cgi`) must auto-vivify the missing root
// (`copies`) and every intermediate struct, exactly like Lucee — NOT throw
// "Variable 'copies' is undefined". A single-level `x.y = v` already
// auto-vivified via StoreLocalProperty; the >=2-level case fell through to a
// generic store that READ the missing base first and threw.
//
// This is the Wheels `redirectionSpec` pattern: a TestBox beforeEach() closure
// stashes `copies.request.cgi = request.cgi`, an afterEach() closure restores
// `request.cgi = copies.request.cgi`. The two closures share the owning
// component's variables scope, so under classic localmode the auto-vivified
// `copies` must land in `variables` (the component scope), not the closure's
// transient local frame — otherwise afterEach can't see it.

// --- Page scope: bare root, 2 + 3 levels deep ---
copies.request.cgi = "hello";
assert("page 2-level vivify", copies.request.cgi, "hello");

fresh.x.y.z = 42;
assert("page 3-level vivify", fresh.x.y.z, 42);

// --- Declared struct, deep write is unaffected (regression guard) ---
s = {a = {}};
s.a.b = 5;
assert("declared struct deep write", s.a.b, 5);

// --- By-reference: deep write through an aliased nested struct ---
o = {foo = {}};
ref = o.foo;
o.foo.bar = 9;
assert("deep write visible through alias", ref.bar, 9);

// --- Reserved-scope root still routes correctly ---
request.q = {};
request.q.r = "ok";
assert("reserved-scope nested write", request.q.r, "ok");

suiteEnd();
</cfscript>

<cfscript>
// --- CFC method + sibling closures share the auto-vivified root ---
// Mirrors the Wheels beforeEach/afterEach stash pattern: closures defined in
// the same method write & read an undeclared `stash` container; it must be
// shared (lands in the component variables scope under classic localmode).
suiteBegin("Core: unscoped nested auto-viv shared across sibling closures in a CFC method");

_avComp = new core.NestedAutovivFixture();
assert("sibling closures share auto-vivified bare root", _avComp.run(), "world");
assert("nested auto-viv via method writes component scope", _avComp.deep(), 7);

suiteEnd();
</cfscript>
