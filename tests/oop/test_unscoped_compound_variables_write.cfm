<cfscript>
suiteBegin("OOP: unscoped compound write resolves to existing variables scope (GitHub 180)");

// An unscoped compound (dotted) LHS instance.newkey = v inside a method,
// where instance already exists in the variables scope (but not in
// local/arguments), must mutate variables.instance, NOT auto-create a
// phantom local.instance whose write is silently lost on return. Verified
// green on Lucee/ACF. Was broken on RustCFML v0.223.0.

ucObj = new UnscopedCompoundFixture();
ucRes = ucObj.writeUnscoped();

assertFalse("unscoped compound write did NOT fork a phantom local.instance",
	ucRes.localHasInstance);
assertTrue("unscoped compound write reached variables.instance (newkey present)",
	ucRes.varsHasNewKey);

// After the call, the shared variables.instance must carry the new key.
assertTrue("variables.instance.newkey persists after the method returns",
	structKeyExists( ucObj.getInstance(), "newkey" ));
assert("variables.instance keys = [existing,newkey]",
	structKeyList( ucObj.getInstance() ), "existing,newkey");

// A function-local of the same name still shadows the variables container.
ucShadow = ucObj.writeShadowedByLocal();
assertTrue("a local var instance shadows the variables-scope container",
	ucShadow.localHasShadowKey);
assertTrue("shadowing local leaves variables.instance untouched",
	ucShadow.varsUntouched);

suiteEnd();
</cfscript>
