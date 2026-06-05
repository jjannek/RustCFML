<cfscript>
suiteBegin("Chained method call does not clobber base variable");

// Regression: `outer.getDep().mutate()` — a chained call whose OUTER mutating
// method runs on a different CFC than the base variable `outer` — must not
// write that inner CFC back onto `outer`. Codegen propagates the writeback
// path (["outer"]) to both calls in the chain; a single-segment path bypassed
// the chained-CFC identity guard, so `outer` was clobbered and became the
// inner CFC. (Sibling of the deep-path bug #15.)

o = new ChainClobberOuter();
assert("base is Outer before chain", o.whoAmI(), "Outer");

o.getDep().setMark( "X" );

// base variable keeps its identity
assert("base still Outer after chained mutate", o.whoAmI(), "Outer");
// the mutation landed on the shared inner instance (reference semantics)
assert("chained mutation persists on shared dep", o.getDep().getMark(), "X");

// Java-shim chaining must STILL propagate (the guard only gates CFC<->CFC):
sb = createObject( "java", "java.lang.StringBuilder" ).init( "" );
sb.append( "a" ).append( "b" ).append( "c" );
assert("java shim chained mutation still works", sb.toString(), "abc");

suiteEnd();
</cfscript>
