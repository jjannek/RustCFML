<cfscript>
// arrayMap over a 10k array with a non-trivial capturing callback, repeated
// PASSES times — exercises closure capture + per-call scope merge (T2.4) and
// call dispatch. Array construction is done ONCE outside the timed region so
// this measures closure/map throughput, not array building.
function buildData( size ) {
    var d = [];
    var i = 0;
    for ( i = 1; i <= arguments.size; i++ ) {
        arrayAppend( d, i );
    }
    return d;
}

function mapPass( data ) {
    // factor/offset are captured by the closure on every element (whole-scope
    // CoW today — the T2.4 target).
    var factor = 3;
    var offset = 7;
    var mapped = arrayMap( arguments.data, function( v ) {
        return ( v * factor ) + offset;
    } );
    var total = 0;
    var i = 0;
    for ( i = 1; i <= arrayLen( mapped ); i++ ) {
        total += mapped[ i ];
    }
    return total;
}

size = 10000;
passes = 1;
data = buildData( size );

mapPass( data );  // warm-up
start = getTickCount();
result = 0;
for ( p = 1; p <= passes; p++ ) {
    result = mapPass( data );
}
elapsed = getTickCount() - start;

writeOutput( "RESULT " & elapsed & chr(10) );
writeOutput( "CHECK " & result & chr(10) );
</cfscript>
