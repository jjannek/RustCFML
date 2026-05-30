<cfscript>
// Fixture for test_repeated_instantiation: a function-defining file that gets
// re-included in a loop. The merge-dedup fix must keep the function callable
// every time without growing the program function table per include.
function repeatedHelperFn(required numeric n) {
    return arguments.n * 2;
}
</cfscript>
