<cfscript>
// String concat in a loop — exercises Concat / Add string-fallthrough and the
// String value representation. Target of T1.1 (String(Arc<str>)) and T4.1
// (in-place string writer). Uses an array-join-free accumulation pattern so it
// stresses concat itself, not a builtin.
function run( n ) {
    var s = "";
    var i = 0;
    for ( i = 1; i <= arguments.n; i++ ) {
        s = s & "x" & i;
        // bound the working-set length so this measures concat throughput,
        // not unbounded O(n^2) growth
        if ( len( s ) > 4096 ) {
            s = right( s, 64 );
        }
    }
    return len( s );
}

iterations = 2000000;
run( 1000 );
start = getTickCount();
result = run( iterations );
elapsed = getTickCount() - start;

writeOutput( "RESULT " & elapsed & chr(10) );
writeOutput( "CHECK " & result & chr(10) );
</cfscript>
