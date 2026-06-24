<cfscript>
suiteBegin("Core: live variables.this alias (Lucee/ACF parity)");

// ============================================================
// Background
// ============================================================
// In Lucee/ACF a component's private `variables` scope carries a LIVE alias to
// its public `this` scope. Mutating `variables.this` (e.g.
// `StructAppend(variables.this, fns)`) mutates the live object, and the new
// keys are immediately public/callable. RustCFML's `variables.this` used to be
// a detached snapshot, so appends vanished — breaking Wheels' plugin-mixin
// injection (Plugins.cfc does `StructAppend(variablesScope.this, mixins)`).
//
// Engine fix: a WEAK back-edge from `__variables` to the live instance,
// resolved on read. Weak => no Arc cycle => no per-request leak (v0.185.0).
//
// The critical case is the CROSS-OBJECT hand-off: a component passes its own
// `variables` to a FOREIGN object which appends to `variables.this`. All
// assertions below pass on Lucee 7.

t = new core.VarThisTarget();

// Private-scope append propagates (the pre-existing v0.272.0 behavior).
assert("private mixin (variables append) is callable", t.mixedViaVars(), "VIA_VARS");

// Public append via the live `variables.this` alias, done by a foreign object,
// is now visible as a public member.
assert("public mixin (variables.this append) callable externally", t.mixedViaThis(), "VIA_THIS");

// A plain property write through `variables.this` reaches the public scope.
assertTrue("variables.this.x = v reaches public scope", StructKeyExists(t, "injectedProp") && t.injectedProp == "PROP");

// StructKeyExists(variables, "this") is true (the gate Wheels Plugins.cfc uses
// before the public append).
assertTrue("StructKeyExists(variables, 'this') is true in a CFC method", t.probeKeyExists());

suiteEnd();
</cfscript>
