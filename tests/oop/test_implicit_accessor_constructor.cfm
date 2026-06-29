<cfscript>
suiteBegin( "Implicit accessor constructor (accessors=true, no init)" );

// Named args populate declared properties via the generated getter backing.
named = new oop.AccessorDto( api="/v1", uri="/some/test/", verb="OPTIONS" );
assert( "named arg -> getApi", named.getApi(), "/v1" );
assert( "named arg -> getUri", named.getUri(), "/some/test/" );
assert( "named arg -> getVerb", named.getVerb(), "OPTIONS" );

// Unprovided properties keep their declared defaults.
partial = new oop.AccessorDto( api="/only-api" );
assert( "provided prop set", partial.getApi(), "/only-api" );
assert( "unprovided prop keeps default", partial.getUri(), "/" );

// argumentCollection spread populates the same way.
ac = new oop.AccessorDto( argumentCollection={ api="/v2", uri="/another/" } );
assert( "argumentCollection -> getApi", ac.getApi(), "/v2" );
assert( "argumentCollection -> getUri", ac.getUri(), "/another/" );

// Positional args are NOT mapped to properties (Lucee 7 verified).
positional = new oop.AccessorDto( "/posApi", "/posUri", "POSV" );
assert( "positional does not populate", positional.getApi(), "/" );

// A property whose name collides with a method: the property value is set
// (getter returns it) but the method stays callable.
collide = new oop.AccessorDto( flag=true );
assert( "colliding property getter", collide.getFlag(), true );
assert( "colliding method still callable", collide.flag(), "FLAG-METHOD" );

// An explicit init() takes over entirely — no implicit population.
withInit = new oop.AccessorInit( api="/should-be-ignored" );
assert( "explicit init wins over implicit population", withInit.getApi(), "from-init" );

// No accessors -> no implicit population; the default stands.
plain = new oop.PlainDto( api="/v1" );
assert( "non-accessor component is not populated", plain.readApi(), "/" );

suiteEnd();
</cfscript>
