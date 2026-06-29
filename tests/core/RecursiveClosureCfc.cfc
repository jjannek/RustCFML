component {
	// A var-scoped recursive function expression inside a CFC method.
	public numeric function fib( required numeric n ) {
		var f = function( x ){
			return x < 2 ? x : f( x - 1 ) + f( x - 2 );
		};
		return f( arguments.n );
	}
}
