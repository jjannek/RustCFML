<cfscript>
// A var-scoped function EXPRESSION must be able to call ITSELF recursively.
// At the point its closure is created the var isn't bound yet, so the closure
// env is seeded without it; the engine binds a stripped self-reference at the
// store so each recursion level can resolve the name again.
// Repro family: Preside FormsService._loadForms `resolveExtensions` (a recursive
// var-scoped function expression that threw "Variable 'resolveExtensions' is
// undefined"). Lucee-verified.
suiteBegin("Recursive var-scoped function expression");

function factorial() {
	var fact = function( n ){
		if ( n <= 1 ) { return 1; }
		return n * fact( n - 1 );
	};
	return fact( 5 );
}
assert("direct recursion", factorial(), 120);

// Recursion that also mutates an enclosing var-scoped value (closure capture
// must survive frame-to-frame alongside the self-reference).
function accumulate() {
	var total = 0;
	var walk  = function( n ){
		total += n;
		if ( n > 1 ) { walk( n - 1 ); }
		return total;
	};
	return walk( 4 );
}
assert("recursion + enclosing capture", accumulate(), 10);

// Sibling capture (the v0.278 case) must still work: a later var-fn calls an
// earlier one by bare name.
function siblings() {
	var dbl = function( x ){ return x * 2; };
	var run = function( y ){ return dbl( y ) + 1; };
	return run( 10 );
}
assert("sibling var-fn capture", siblings(), 21);

// Recursion inside a CFC method (the store reaches the same locals-insert path).
recursiveCfc = new core.RecursiveClosureCfc();
assert("recursion in CFC method", recursiveCfc.fib( 10 ), 55);

suiteEnd();
</cfscript>
