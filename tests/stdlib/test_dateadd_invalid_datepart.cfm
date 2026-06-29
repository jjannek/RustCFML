<cfscript>
suiteBegin( "dateAdd / dateDiff — invalid datepart throws" );

// Lucee/ACF throw an `expression` error for an unknown datepart rather than
// silently no-op'ing. Frameworks rely on the throw — Preside's
// RulesEngineTimePeriodService wraps dateAdd in try/catch and returns {} when
// the user-supplied unit is invalid.
assertThrows( "dateAdd invalid datepart throws", function(){
	dateAdd( "sadfjhasd", -3, now() );
} );
assertThrows( "dateDiff invalid datepart throws", function(){
	dateDiff( "zzz", now(), now() );
} );

// Valid dateparts still work. Measure the result with dateDiff so the
// assertions don't depend on per-engine date stringification.
base = "2026-01-01 00:00:00";
assert( "dateAdd d still works", dateDiff( "d", base, dateAdd( "d", 5, base ) ), 5 );
assert( "dateAdd yyyy still works", dateDiff( "yyyy", base, dateAdd( "yyyy", 1, base ) ), 1 );
assert( "dateDiff d still works", dateDiff( "d", "2026-01-01", "2026-01-06" ), 5 );

suiteEnd();
</cfscript>
