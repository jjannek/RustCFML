<cfscript>
suiteBegin("mixin into private variables scope is dispatchable as a method");

// Wheels' plugin loader injects plugin methods by StructAppend-ing function
// references into a component's `variables` scope (and `variables.this` on
// engines that expose it). The injected method must be callable as a member
// (controller.$helper01()) and via a bare in-method call. RustCFML has no live
// `variables.this`, so the function lands only in the private __variables
// scope; member dispatch falls back to it. Lucee reaches the same callable
// result through the `variables.this` append. Cross-engine safe: assert only
// callability (key VISIBILITY on the public object differs by engine and is
// intentionally not asserted).

obj = new oop.MixinScopeFixture();
mixins = {};
mixins["$mixedIn"] = function() { return "mixed-in-ok"; };
obj.injectMixins(mixins);

// External member dispatch of the injected method.
assert("injected method callable as a member", obj.$mixedIn(), "mixed-in-ok");

// Bare in-method call of the injected method.
assert("injected method callable bare from another method", obj.callMixedInBare(), "mixed-in-ok");

suiteEnd();
</cfscript>
