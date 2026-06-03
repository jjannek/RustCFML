<cfscript>
suiteBegin("HOF member dispatch — no this-writeback leak");

// Regression: a higher-order struct member function (some/every/map/filter/...)
// invoked INSIDE a CFC method, over a struct whose closure captured the method's
// `this`, leaked that `this` via the method-writeback channel — overwriting the
// receiver variable with the enclosing component and returning wrong results.
// This is the bug that made WireBox's `binder.hasAspects()`
// (`mappings.some( (k,m) => m.isAspect() )`) spuriously true, which then
// auto-registered AOP and corrupted the binder. The fix gates the method
// `this`/variables write-back on receivers that actually have those semantics
// (CFCs / Java shims), discarding the leaked write-back for plain structs.

probe = new HofWritebackProbe();

// `.some()` over an instance-var struct of components must return false.
assertFalse("anyFlagged (instance var) returns false", probe.anyFlagged());
// Calling again must be consistent (the first call must not corrupt the struct).
assertFalse("anyFlagged is stable across calls", probe.anyFlagged());
// Local-copy form must also work (used to throw "has no function [some]").
assertFalse("anyFlaggedViaLocal returns false", probe.anyFlaggedViaLocal());
// The instance-var struct itself must be intact (not replaced by `this`).
assert("instance struct still has 3 items after HOF calls", probe.itemCount(), 3);
assertFalse("instance struct not corrupted into the component", probe.looksLikeComponent());

suiteEnd();
</cfscript>
