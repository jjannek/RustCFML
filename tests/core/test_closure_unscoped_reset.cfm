<!---
  Regression: an UNSCOPED variable REASSIGNED inside a stored/dispatched closure
  (the TestBox `beforeEach = function(){ auth = new X(); }` shape) must be visible
  to sibling spec closures — each spec sees the freshly-reset instance, not a
  stale one shared across iterations.

  Root cause (fixed): in a classic-localmode CFC method an unscoped write lands in
  the component (__variables) scope, but the method-receiver write-back after a
  method call on the var (e.g. `obj.add()`), and the closure parent write-back,
  routed through scope_aware_store — which forked a TOP-LEVEL `locals` shadow
  instead of updating __variables. A bare read checks top-level locals before
  __variables, so that shadow — pinned on the first dispatch and persisted in the
  looping frame's locals — masked every later reset the `before` closure wrote to
  __variables, and the object accumulated across iterations (e.g. "1,2,3,4"
  instead of "1,1,1,1").

  Fix: scope_aware_store now routes an unscoped bare name into __variables in a
  classic-localmode CFC frame, mirroring StoreLocal / store_runtime_path, so the
  write side agrees with where a bare read resolves. Cleared all 6 Wheels
  AuthenticatorSpec failures; verified identical on Lucee 7.
--->
<cfscript>
suiteBegin("Closure unscoped reassignment visible to sibling closures");

suite = new ClosureResetSuite();

// THE bug: each iteration the `before` closure resets the unscoped object to a
// fresh instance; the spec mutates+reads it. Each fresh object holds exactly one
// item, so the count is 1 every time. The bug accumulated → "1,2,3,4".
assert("unscoped reset visible each iteration (mutate)", suite.mutateEachIteration(), "1,1,1,1");

// Read-only spec on a freshly-reset object sees an empty instance every time.
assert("unscoped reset visible each iteration (read-only)", suite.readOnlyEachIteration(), "0,0,0,0");

// Distinct specs run against a per-iteration reset object (AuthenticatorSpec
// register/replace/remove shape): each spec's own fresh object.
assert("distinct specs against per-iteration reset", suite.distinctSpecsSequence(), "0,1,2,1");

// Control: explicit variables-scoped reset/read always worked; must still pass.
assert("explicit variables-scoped reset control", suite.explicitScopeControl(), "1,1,1,1");

// Sibling closures defined together share the captured unscoped var.
assert("sibling closures share captured unscoped var", suite.siblingShare(), "SET");

suiteEnd();
</cfscript>
