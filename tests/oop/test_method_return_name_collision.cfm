<cfscript>
// Regression (TestBox blocker #2): a component pseudo-constructor (`__cfc_body__`)
// run with an injected parent scope used to leak ALL its method declarations out
// as a stale closure_parent_writeback. When a method returned a freshly-built
// component whose own method name collided with the caller's method name, the
// leaked methods poisoned the caller frame's locals — so a SECOND bare call to
// the caller's method resolved to the RETURNED object's method instead.
//   var x = getThing();   // OK — Host.getThing
//   var y = getThing();   // used to resolve to Inner.getThing (required key) -> error
// This is what stopped TestBox's TestBox.cfc activateModule() from running.
suiteBegin("Method return / name collision");

host = new CollisionHost();

r = host.twoBareCalls();
assert("two bare calls don't error + both return inner", listLen(r, "-") == 2 && listGetAt(r, 1, "-") == listGetAt(r, 2, "-"), true);
assertTrue("extract-then-bare-call doesn't error", host.extractThenCall() != "");
assertTrue("new-each-call doesn't error", host.newEachCall() != "");

suiteEnd();
</cfscript>
