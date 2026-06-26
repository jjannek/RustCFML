<cfscript>
suiteBegin("Core: MockBox back-ref survives scope-prefixed writeback (issue 204)");

// Regression for issue #204 (v0.281.0). The v0.281 change made a CFC method's
// `variables`-writeback target the full receiver path. For a method reached
// through a SCOPE PREFIX — `arguments.targetObject.$include(...)` inside
// MockBox's MockGenerator — that navigated into the arguments/this/variables
// scope and re-stored a reconstructed copy, detaching the shared component Arc
// and nulling the target's back-reference (`this.mockBox`). MockBox's generated
// methods then failed with "cannot call method [normalizeArguments] on a null
// value". This reproduces that decoration chain in miniature. Verified against
// the real TestBox MockBox.

factory = new oop.Mock204Factory();
mock    = factory.createEmptyMock( "oop.Plain204" );

assertTrue( "back-ref set after createEmptyMock", isObject( mock.backref ) );

// Drives generation via this.backref.getGen() then arguments.targetObject.doInc()
mock.doMock( "m" );

assertTrue(
    "back-ref survives the scope-prefixed injection writeback",
    structKeyExists( mock, "backref" ) && isObject( mock.backref )
);

// The injected method reads this.backref at call time — the symptom of #204.
assert( "injected method resolves the back-ref", mock.m(), "norm:ok" );

suiteEnd();
</cfscript>
