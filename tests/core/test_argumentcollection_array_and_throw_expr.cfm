<cfscript>
suiteBegin("Core: array argumentCollection spread + throw() in expression position");

// ============================================================
// (1) An ARRAY argumentCollection spreads as POSITIONAL args
// ============================================================
// Lucee/ACF/BoxLang: `fn( argumentCollection=[a,b,c] )` is the same as calling
// `fn(a,b,c)` positionally. RustCFML previously handled only a STRUCT
// argumentCollection and passed an array through as a single argument.
// MockBox's `obj.$("m").$results( argumentCollection=arr )` depends on this to
// register one sequential result per element — the bug surfaced as a Preside
// webflow test (`WebflowService.loadFlowDirectory`) failure.

function collectPositional() {
	var out = [];
	for ( var i = 1; i <= arguments.len(); i++ ) {
		out.append( arguments[ i ] );
	}
	return out.toList( "|" );
}

assert( "array argumentCollection spreads to positional args",
	collectPositional( argumentCollection = [ "a", "b", "c" ] ),
	"a|b|c" );

// A numeric-keyed struct argumentCollection still spreads positionally (control).
assert( "numeric-keyed struct argumentCollection still spreads positionally",
	collectPositional( argumentCollection = { "1" = "x", "2" = "y" } ),
	"x|y" );

// Declared params bind by position from an array argumentCollection.
function twoParams( a, b ) {
	return "a=" & arguments.a & " b=" & arguments.b;
}
assert( "array argumentCollection binds declared params by position",
	twoParams( argumentCollection = [ "first", "second" ] ),
	"a=first b=second" );

// ============================================================
// (2) throw() usable in EXPRESSION position (Lucee BIF form)
// ============================================================
// `x ?: throw("msg","type")` — Preside's WebflowSpecLibrary.getWebflow uses
// exactly this idiom. Previously only the statement form of throw() parsed.

st = {};
caught = "";
try {
	flow = st[ "missing" ] ?: throw( "not found here", "my.custom.type" );
} catch ( any e ) {
	caught = e.type & "|" & e.message;
}
assert( "throw() on RHS of elvis operator throws with correct type/message",
	caught, "my.custom.type|not found here" );

// The elvis short-circuits when the LHS exists — throw() is never evaluated.
st2 = { present = "value" };
noThrow = st2[ "present" ] ?: throw( "should not fire", "should.not.happen" );
assert( "throw() not evaluated when elvis LHS is present", noThrow, "value" );

// Named-arg expression form also works.
caught2 = "";
try {
	x = st[ "missing" ] ?: throw( message = "named msg", type = "named.type" );
} catch ( any e ) {
	caught2 = e.type & "|" & e.message;
}
assert( "throw() named-arg expression form throws correctly",
	caught2, "named.type|named msg" );

suiteEnd();
</cfscript>
