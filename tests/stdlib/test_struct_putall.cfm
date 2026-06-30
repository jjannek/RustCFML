<cfscript>
suiteBegin( "struct.putAll() — java.util.Map.putAll member (cross-engine)" );

// Lucee exposes java.util.Map.putAll on structs; it merges the argument struct
// into the receiver in place (overwriting). Preside's cfflow YamlParser.toCF()
// relies on `cfObj.putAll( map )`. Runs on RustCFML and Lucee.
dst = { existing = 1 };
src = { a = 1, b = "x", c = [ 1, 2 ] };
dst.putAll( src );

assert( "merged key count", structCount( dst ), 4 );
assert( "kept existing key", dst.existing, 1 );
assert( "copied scalar", dst.b, "x" );
assert( "copied nested array element", dst.c[ 2 ], 2 );

// putAll overwrites existing keys.
dst.putAll( { existing = 99 } );
assert( "putAll overwrites", dst.existing, 99 );

suiteEnd();
</cfscript>
