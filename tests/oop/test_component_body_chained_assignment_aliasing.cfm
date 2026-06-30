<cfscript>
suiteBegin( "Component-body assignment aliasing: this.x and variables.x share one ref (##221)" );

demo = createObject( "component", "oop.ChainAliasDemo" );

// Chained `variables.obj = this.obj = new CFC()`: mutating through one name
// is visible through the other (single shared reference, not a copy).
assert( "chained assignment shares the instance", demo.mutateChained(), "true/true" );

// The dynamically-added function is reachable through the leftmost name too.
assert( "added fn callable via variables.obj", demo.callViaVariables(), "hi" );

// Non-chained body-level aliasing (this.alias = new; variables.alias = this.alias)
// is the same root cause and must also share.
assert( "non-chained body alias shares the instance", demo.mutateAlias(), "true/true" );

// Two SEPARATE `new CFC()` objects in this/variables must NOT alias.
assert( "distinct objects stay distinct", demo.mutateDistinct(), "true/false" );

// Instance isolation: a second instance must not see the first's mutation.
demo2 = createObject( "component", "oop.ChainAliasDemo" );
demo.mutateChained();
assert( "second instance is isolated", structKeyExists( demo2.obj, "added" ), false );

suiteEnd();
</cfscript>
