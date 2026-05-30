<cfscript>
// CFC instantiation + property reads — exercises Component construction,
// GetProperty, and method dispatch. Primary target of T1.3 (CI-hash property
// lookup) and T3.2 (inline caches for this.X).
function run( n ) {
    var total = 0;
    var i = 0;
    var p = "";
    for ( i = 1; i <= arguments.n; i++ ) {
        p = new Point( i, i + 1 );
        // read properties + call a method that reads more properties
        total += p.x + p.y + p.distSq();
    }
    return total;
}

iterations = 100000;
run( 1000 );
start = getTickCount();
result = run( iterations );
elapsed = getTickCount() - start;

writeOutput( "RESULT " & elapsed & chr(10) );
writeOutput( "CHECK " & result & chr(10) );
</cfscript>
