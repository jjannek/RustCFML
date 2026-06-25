<cfscript>
suiteBegin("cfinvoke/invoke component= overlays live receiver scope");

// A method grafted onto another component and invoked via
// `cfinvoke component=obj` must run against obj's LIVE this/variables, so an
// unscoped write inside the method persists into obj and is visible to a later
// direct `obj.method()` dot-call. Before the fix, cfinvoke dispatched against a
// detached clone, so the write was lost and the dot-call read threw
// "Variable 'hasObjectChanged' is undefined". Regression for Wheels
// callbacksSpec afterSaveProperties ($callback -> $invoke -> cfinvoke).
spec = new oop.CfinvokeOverlaySpec();
assert("cfinvoke component= write persists to receiver (read via dot-call)", spec.runWithComponent(), "yes");
assert("invoke() BIF write persists to receiver (read via dot-call)", spec.runWithInvokeBif(), "yes");

suiteEnd();
</cfscript>
