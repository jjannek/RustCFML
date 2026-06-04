<cfscript>
suiteBegin("CFC method reference binding through higher-order callbacks");

// Regression: a CFC method referenced by bare name (`.each( record )`)
// used to lose its receiver — when the higher-order BIF invoked the
// callback, `this`/`variables` were undefined. Lucee/ACF bind the
// receiver at the load site so the callback still sees the component
// instance. Surfaced booting WireBox: `processDIMetadata` does
// `arguments.metadata.properties.each( processPropertyMetadata )` and
// the callee's `addDIProperty(...)` body ended with `return this;`
// raising "Variable 'this' is undefined".

bag = new MethodRefBag();
bag.setPrefix( "px-" );
bag.setAllow( "keep1,keep2" );

// --- .each() preserves `this`/`variables` ---
out = bag.runEach( [ "a", "b", "c" ] );
assert("each callback writes via variables", arrayToList( out ), "a,b,c");

// --- .map() preserves `variables` (callee reads its own state) ---
mapped = bag.runMap( [ "x", "y" ] );
assert("map callback reads variables", arrayToList( mapped ), "PX-X,PX-Y");

// --- .filter() preserves `variables` (callee reads its own allow-list) ---
kept = bag.runFilter( [ "keep1", "drop", "keep2", "junk" ] );
assert("filter callback reads variables", arrayToList( kept ), "keep1,keep2");

// --- Nested bare-name dispatch: callback calls another bare-name method ---
bag2 = new MethodRefBag();
nested = bag2.nestedCall( [ "u", "v" ] );
assert("nested bare-name method call preserves binding",
    arrayToList( nested ), "stamped:u,stamped:v");

// --- An inline closure used at the same call site still works (regression
//     guard: the load-site rewrap must not damage closures that already
//     carry their own captured_scope). ---
inlineHits = [];
[ 1, 2, 3 ].each( function( n ){ arrayAppend( inlineHits, n * 10 ); } );
assert("inline closure callback still works", arrayToList( inlineHits ), "10,20,30");

suiteEnd();
</cfscript>
