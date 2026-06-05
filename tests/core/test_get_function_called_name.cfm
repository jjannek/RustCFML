<cfscript>
suiteBegin("getFunctionCalledName");

// getFunctionCalledName() returns the name the currently-executing UDF was
// invoked under. For a plain named call that's the declared name; for a UDF
// injected under several aliases (as WireBox does for delegated methods) it's
// the alias used at the call site. This is the primitive WireBox delegation
// dispatch relies on: one getByDelegate() UDF, injected under many method
// names, routed by getFunctionCalledName().

// --- plain named function: returns its own declared name ---
function whoAmI(){
	return getFunctionCalledName();
}
assert("plain named call returns declared name", whoAmI(), "whoAmI");

// --- aliased dispatch: same UDF reached via different method names ---
probe = new CalledNameProbe();
assert("alias alpha dispatches as 'alpha'", probe.alpha(), "alpha");
assert("alias beta dispatches as 'beta'",   probe.beta(),  "beta");
assert("alias gamma dispatches as 'gamma'", probe.gamma(), "gamma");
// the component's own, normally-declared method reports its real name
assert("declared method reports its name",   probe.named(), "named");

suiteEnd();
</cfscript>
