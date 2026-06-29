<cfscript>
suiteBegin( "arrayFind / arrayFindAll — closure-predicate form" );

structs = [ { name="alpha" }, { name="beta" }, { name="gamma" } ];

// Member form: return 1-based index of first element matching the predicate.
assert( "member .find(closure) returns first match index", structs.find( function( it ){ return it.name == "beta"; } ), 2 );
assert( "member .find(closure) no match returns 0", structs.find( function( it ){ return it.name == "zzz"; } ), 0 );

// Standalone BIF form.
assert( "arrayFind(arr, closure)", arrayFind( structs, function( it ){ return it.name == "gamma"; } ), 3 );

// findAll closure: all matching indices.
nums = [ 5, 12, 3, 20, 8 ];
assert( "member .findAll(closure)", nums.findAll( function( n ){ return n > 7; } ).toList(), "2,4,5" );
assert( "arrayFindAll(arr, closure)", arrayFindAll( nums, function( n ){ return n > 7; } ).toList(), "2,4,5" );

// The value-needle form still works alongside the closure form.
assert( "value-needle find still works", arrayFind( [ 10, 20, 30 ], 20 ), 2 );

suiteEnd();
</cfscript>
