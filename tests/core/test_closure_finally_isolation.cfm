<cfscript>
// A nested function/closure is its own control-flow boundary. Two regressions
// (surfaced by the WireBox port's Mapping.cfc `produceMetadataUDF`) lived here:
//
//   1. A closure defined inside `lock {}` / `try{}finally{}` must NOT inherit
//      the enclosing function's `finally` block. The compiler's finally-stack
//      leaked across the function boundary, appending the enclosing finally
//      into the closure body — so calling the closure ran the lock's
//      `__cflock_end(...)` (which references the enclosing `arguments` scope,
//      empty inside the closure) and blew up.
//
//   2. `isDefined("arguments.x.y")` wrongly returned false (the arguments scope
//      lives under a reserved key, not the literal "arguments"), so
//      `param arguments.x.y = []` clobbered a populated value.

suiteBegin( "Closure / finally isolation" );

// --- 1. Closure inside a lock returns its own value; the lock's finally
//        (which reads arguments) does not bleed into the closure body. ---
obj1 = new ClosureFinallyFixture();
assert( "closure-in-lock returns its own value", obj1.lockedClosure( "INJ" ), "util-for-INJ" );

// --- bare try/finally variant (no lock) ---
obj2 = new ClosureFinallyFixture();
assert( "closure-in-try-finally returns its own value", obj2.tryFinallyClosure( 7 ), 14 );

// --- the enclosing finally still runs exactly once (side-effect probe) ---
obj3 = new ClosureFinallyFixture();
obj3.runWithFinallyProbe();
assert( "enclosing finally ran exactly once", obj3.getFinallyCount(), 1 );

suiteEnd();

suiteBegin( "isDefined on nested arguments path" );

function checkArgsNested( required metadata ){
    // metadata.properties is populated by the caller — the probe must see it.
    var seenBefore = isDefined( "arguments.metadata.properties" );
    // CFML `param` no-ops when the path is already defined; if isDefined wrongly
    // returns false it would clobber the populated array with [].
    param arguments.metadata.properties = [];
    return { seen : seenBefore, len : arrayLen( arguments.metadata.properties ) };
}

md = { name : "X", properties : [ { name : "a" }, { name : "b" } ] };
r = checkArgsNested( md );
assertTrue( "isDefined sees a nested arguments path", r.seen );
assert( "param does not clobber a populated nested arguments array", r.len, 2 );

// negative: a genuinely-absent nested key is not defined
function checkAbsent( required metadata ){
    return isDefined( "arguments.metadata.missing" );
}
assertFalse( "isDefined is false for an absent nested arguments key", checkAbsent( md ) );

suiteEnd();
</cfscript>
