<cfscript>
suiteBegin("Closure env: sibling aliasing (leak-fix guard)");

// These assertions lock in the closure semantics that must survive the fix for
// the closure_env Arc-cycle memory leak (closures are no longer stored inside
// the shared captured env). All the preserved behaviours rely on sharing/seeing
// NON-function captured variables, which still flow through the env.

// (1) Two sibling closures share a mutated captured (non-function) variable.
function makeCounterPair() {
    var n = 0;
    var inc = function() { n++; };
    var get = function() { return n; };
    return { inc: inc, get: get };
}
pair = makeCounterPair();
pair.inc();
pair.inc();
pair.inc();
assert("sibling closures share mutated captured var", pair.get(), 3);

// (2) A later closure sees a variable declared between the two definitions.
function makeLate() {
    var first = function() { return 1; };
    var mid = 99;
    var second = function() { return mid; };
    return second;
}
assert("later closure sees intervening declaration", makeLate()(), 99);

// (3) Factory closures remain independent (each call's capture is distinct).
function makeAdder( a ) {
    return function( b ) { return a + b; };
}
assert("factory closures are independent", makeAdder( 5 )( 3 ) + makeAdder( 10 )( 3 ), 21);

// (4) A closure still captures and reads an outer variable correctly after the
//     defining frame has returned (escape + capture of non-function value).
function makeGreeter( name ) {
    return function() { return "hi " & name; };
}
g = makeGreeter( "ada" );
assert("escaped closure reads captured value", g(), "hi ada");

suiteEnd();
</cfscript>
