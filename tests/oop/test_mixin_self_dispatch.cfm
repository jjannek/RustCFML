<cfscript>
suiteBegin( "Mixin self-dispatch: extracted method binds to invocant (##220)" );

host = createObject( "component", "oop.MixinHost" );

// Wheels $integrateFunctions: a method copied from a source component onto a
// host and invoked through the host must self-dispatch on the HOST. Pre-##220 an
// eager GetIndex binding froze the source scope, so this returned
// selfName=MixinSource and the sibling lookups failed.
integrated = host.runIntegrated();
assert( "integrated method sees HOST as this", integrated, "selfName=oop.MixinHost | viaVariables=HOST-TARGET-OK | viaInvoke=HOST-TARGET-OK" );

// TestBox lifecycle dispatch: a method extracted via this[name] into a plain
// struct and invoked as `bag.fn()` from inside the host must bind to the
// caller's component context (the host).
structDispatch = host.runStructDispatch();
assert( "struct-dispatched mixin sees HOST scope", structDispatch, "lifecycle:HOST-VARS:oop.MixinHost" );

// The host's OWN method, extracted via this[name] into a plain struct and
// dispatched, must also see the host scope (the original TestBox fix #5 case).
ownDispatch = host.runOwnStructDispatch();
assert( "own struct-dispatched method sees HOST scope", ownDispatch, "own:HOST-VARS" );

suiteEnd();
</cfscript>
