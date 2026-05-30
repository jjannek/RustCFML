<cfscript>
// Tight integer loop — exercises ForLoopStep, integer add, local load/store.
// Primary target of T3.1 (slot-based locals) and T1.2 (drop to_lowercase on
// every local access). Bodies live in a function so `var` is legal on Lucee.
function run( n ) {
    var sum = 0;
    var i = 0;
    for ( i = 1; i <= arguments.n; i++ ) {
        sum += i;
    }
    return sum;
}

iterations = 10000000;
// warm-up (let any lazy init settle) then timed run
run( 100000 );
start = getTickCount();
result = run( iterations );
elapsed = getTickCount() - start;

writeOutput( "RESULT " & elapsed & chr(10) );
writeOutput( "CHECK " & result & chr(10) );
</cfscript>
