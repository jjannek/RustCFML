<cfscript>
suiteBegin("new X() binds named constructor args by name, not position");

// Regression (WireBox port, Phase-5 providers blocker). `new X(...)` compiled to
// a positional NewObject op that discarded the call-site argument names, so init
// received args bound by CALL ORDER. When the call order differed from the
// declared parameter order (as in `new Provider( scopeRegistration:..,
// targetObject:.., name:.., injectorName:.. )` where `name` is declared 2nd),
// `arguments.name` got the wrong slot — Provider.getName() came back empty and
// every provider lookup failed with "Instance not found: ''". createObject().init()
// already bound by name; only the `new` path was broken. Fixed by emitting a
// NewObjectNamed op that reorders args against init()'s declared params.

// out-of-order named args (declared: meta, name, targetObject, tag)
p = new NamedArgCtor( meta : { x : 1 }, targetObject : "T", name : "N", tag : "G" );
assert(     "out-of-order: name bound by name",         p.getName(), "N" );
assert(     "out-of-order: tag bound by name",          p.getTag(), "G" );
assert(     "out-of-order: targetObject bound by name", p.getTargetObject(), "T" );
assert(     "out-of-order: meta bound by name",         p.getMeta().x, 1 );

// in-order named (control)
q = new NamedArgCtor( meta : { x : 2 }, name : "N2", targetObject : "T2", tag : "G2" );
assert( "in-order: name",  q.getName(), "N2" );
assert( "in-order: tag",   q.getTag(), "G2" );

// omit the optional middle param while the rest are out of order
r = new NamedArgCtor( meta : { x : 3 }, tag : "G3", targetObject : "T3" );
assertFalse( "omitted optional name is unset", r.hasName() );
assert(      "omit-optional: tag still correct",          r.getTag(), "G3" );
assert(      "omit-optional: targetObject still correct", r.getTargetObject(), "T3" );

suiteEnd();

// ---------------------------------------------------------------------------
suiteBegin("createObject().init() named args still bind by name (control)");

c = createObject( "component", "oop.NamedArgCtor" ).init(
	meta         : { x : 4 },
	targetObject : "T4",
	name         : "N4",
	tag          : "G4"
);
assert( "createObject init: name", c.getName(), "N4" );
assert( "createObject init: tag",  c.getTag(), "G4" );

suiteEnd();
</cfscript>
