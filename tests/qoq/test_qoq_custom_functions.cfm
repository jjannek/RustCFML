<cfscript>
// Query of Queries — custom CFML functions inside SQL via queryRegisterFunction.
// RustCFML-specific: probe support first and skip the suite where unavailable
// (e.g. Lucee), so the cross-engine run stays green.

suiteBegin("QoQ Custom Functions");

qoqCustomFnSupported = true;
try {
    queryRegisterFunction("__qoqProbe", function(x) { return x; });
} catch (any e) {
    qoqCustomFnSupported = false;
}

if (qoqCustomFnSupported) {
    qoqNums = queryNew("n,grp", "integer,varchar", [
        { n: 1, grp: "a" }, { n: 2, grp: "a" }, { n: 3, grp: "b" }, { n: 4, grp: "b" }
    ]);

    // Scalar UDF in SQL
    queryRegisterFunction("doubleIt", function(x) { return x * 2; });
    qScalar = queryExecute(
        "SELECT n, doubleIt(n) AS dbl FROM qoqNums ORDER BY n",
        {}, { dbtype: "query" });
    assert("custom scalar first", qScalar.dbl[1], 2);
    assert("custom scalar last", qScalar.dbl[4], 8);

    // Aggregate UDF in SQL (receives the column's values for the group as an array)
    queryRegisterFunction("product", function(vals) {
        var p = 1;
        for (var v in vals) { p = p * v; }
        return p;
    }, "aggregate");
    qAgg = queryExecute(
        "SELECT grp, product(n) AS prod FROM qoqNums GROUP BY grp ORDER BY grp",
        {}, { dbtype: "query" });
    assert("custom aggregate a", qAgg.prod[1], 2);  // 1 * 2
    assert("custom aggregate b", qAgg.prod[2], 12); // 3 * 4

    // Scalar UDF combined with a built-in in the WHERE clause
    qWhere = queryExecute(
        "SELECT n FROM qoqNums WHERE doubleIt(n) > 4 ORDER BY n",
        {}, { dbtype: "query" });
    assert("custom in where count", qWhere.recordCount, 2); // n=3,4 -> 6,8 > 4
    assert("custom in where first", qWhere.n[1], 3);
} else {
    writeOutput("       (skipped: queryRegisterFunction not supported on this engine)" & chr(10));
}

suiteEnd();
</cfscript>
