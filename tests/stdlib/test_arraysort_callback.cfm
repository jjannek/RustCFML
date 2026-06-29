<cfscript>
suiteBegin( "arraySort — callback-comparator (BIF) form" );

// The standalone `arraySort( arr, comparatorFn )` must run the CFML closure per
// comparison (like the `arr.sort(fn)` member form). Previously the BIF
// stringified the closure and did a text sort, leaving structs effectively
// unsorted — Preside RulesEngineExpressionService.listExpressions relies on it.
list = [ { label="banana" }, { label="apple" }, { label="cherry" } ];
arraySort( list, function( a, b ){ return a.label > b.label ? 1 : -1; } );
assert( "callback sorts structs ascending", list[1].label & "," & list[2].label & "," & list[3].label, "apple,banana,cherry" );

// Descending comparator.
nums = [ 3, 1, 2 ];
arraySort( nums, function( a, b ){ return a < b ? 1 : -1; } );
assert( "callback sorts descending", nums.toList(), "3,2,1" );

// Stable: equal keys keep relative order.
pairs = [ { k=1, tag="a" }, { k=1, tag="b" }, { k=0, tag="c" } ];
arraySort( pairs, function( a, b ){ return a.k > b.k ? 1 : ( a.k < b.k ? -1 : 0 ); } );
assert( "stable sort keeps order of equal keys", pairs[1].tag & pairs[2].tag & pairs[3].tag, "cab" );

// The string sort-type form still works (falls through to the builtin).
words = [ "Banana", "apple", "Cherry" ];
arraySort( words, "textnocase" );
assert( "string sort-type still works", words.toList(), "apple,Banana,Cherry" );

suiteEnd();
</cfscript>
