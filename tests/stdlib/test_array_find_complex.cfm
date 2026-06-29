<cfscript>
suiteBegin( "arrayFind / arrayContains — deep complex-value matching" );

// Lucee/ACF match COMPLEX needles (structs/arrays) by deep equality, with
// case-insensitive + order-insensitive struct keys. Previously RustCFML only
// did scalar string matching, so a struct needle always returned 0/false
// (root cause of Preside RulesEngineAutoPresideObjectExpressionGenerator's
// `expressions.find( expectedExpr ) > 0` specs).
arr = [ { a=1, b="x" }, { a=2, b="y", c=[ 1, 2 ] } ];

// Struct needle with different key order + a nested array — still matches.
assert( "arrayFind struct needle (order-insensitive)", arrayFind( arr, { b="y", a=2, c=[ 1, 2 ] } ), 2 );
assert( "member .find struct needle", arr.find( { b="y", a=2, c=[ 1, 2 ] } ), 2 );

// No match when a value differs.
assert( "arrayFind struct no-match", arrayFind( arr, { a=2, b="z" } ), 0 );

// No match when a key count differs (extra/missing key).
assert( "arrayFind struct extra-key no-match", arrayFind( arr, { a=1, b="x", d=9 } ), 0 );

// Nested array element mismatch.
assert( "arrayFind nested-array mismatch", arrayFind( arr, { a=2, b="y", c=[ 1, 3 ] } ), 0 );

// arrayContains with a struct needle (deep).
assert( "arrayContains struct needle", arrayContains( arr, { a=1, b="x" } ), true );
assert( "arrayContains struct no-match", arrayContains( arr, { a=9, b="q" } ), false );

// Case-insensitive scalar still works; numeric/string coercion preserved.
assert( "arrayFind scalar", arrayFind( [ 10, 20, 30 ], 20 ), 2 );
assert( "arrayFindNoCase scalar", arrayFindNoCase( [ "Foo", "Bar" ], "bar" ), 2 );

// Array-of-arrays needle.
nested = [ [ 1, 2 ], [ 3, 4 ] ];
assert( "arrayFind array needle", arrayFind( nested, [ 3, 4 ] ), 2 );

suiteEnd();
</cfscript>
