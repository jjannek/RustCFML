<cfscript>
suiteBegin("Core: component method table is visible during construction (ordering)");

// ============================================================
// Background
// ============================================================
// CFML (Lucee/ACF/BoxLang) assembles a component's `variables` scope — all
// own + inherited methods — BEFORE running the pseudo-constructor. So a
// method invoked DURING construction sees a fully-populated `variables`.
// RustCFML previously ran the body against a scope that lacked the method
// table, so a mid-construction `StructKeyExists(variables, "someMethod")`
// was false. That broke real frameworks: Wheels `model("user")` called in a
// controller's pseudo-constructor dispatched through inherited helpers whose
// `variables` was empty, threw, the body error was swallowed, and the
// instance came back half-built ("X is not an object").
//
// Three call shapes are exercised, all evaluated during the body:
//   * bare sibling call             -> variables sees sibling
//   * this.method() dispatch        -> in-construction `this` (no own
//                                      __variables yet) falls back to the
//                                      caller's hoisted method table
//   * cfinvoke method= (NO component) -> in-scope invoke, not "Component ''
//                                      not found" (Wheels' $invoke shape)

_co = new core.ConstructOrderFixture();
assert("bare sibling call sees method table during ctor", _co.bareR(), "sees");
assert("this.method() dispatch works during ctor", _co.thisR(), "sib");
assert("cfinvoke with no component invokes in-scope during ctor", _co.invokeR(), "sib");

suiteEnd();
</cfscript>

<cfscript>
suiteBegin("Core: inherited method table is visible during construction");

// An inherited method called during the CHILD's pseudo-constructor must see
// inherited siblings in variables — both as a bare call and via this.method().
_coc = new core.ConstructOrderChild();
assert("inherited bare call sees inherited sibling during ctor", _coc.inhBareR(), "sees");
assert("inherited this.method() dispatch during ctor", _coc.inhThisR(), "sees");

suiteEnd();
</cfscript>
