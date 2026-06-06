<cfscript>
// Query of Queries — RustCFML/BoxLang SUPERSET features that Lucee QoQ rejects:
// LIMIT/OFFSET, scalar subqueries in the SELECT list, and derived FROM tables.
// Probe support first (via LIMIT) and skip on engines without it, so the
// cross-engine run stays green.

suiteBegin("QoQ RustCFML Extensions");

qoqExt = queryNew("id,name,dept", "integer,varchar,integer", [
    { id: 1, name: "Alice", dept: 10 },
    { id: 2, name: "Bob",   dept: 20 },
    { id: 3, name: "Carol", dept: 10 },
    { id: 4, name: "Dave",  dept: 20 }
]);
qoqExtD = queryNew("id,title", "integer,varchar", [
    { id: 10, title: "Engineering" }, { id: 20, title: "Sales" }
]);

qoqExtSupported = true;
try {
    queryExecute("SELECT id FROM qoqExt ORDER BY id LIMIT 1", {}, { dbtype: "query" });
} catch (any e) {
    qoqExtSupported = false;
}

if (qoqExtSupported) {
    // LIMIT / OFFSET
    qLim = queryExecute(
        "SELECT name FROM qoqExt ORDER BY name LIMIT 2 OFFSET 1",
        {}, { dbtype: "query" });
    assert("limit/offset count", qLim.recordCount, 2);
    assert("limit/offset first", qLim.name[1], "Bob");
    assert("limit/offset second", qLim.name[2], "Carol");

    // Scalar subquery in the SELECT list
    qScalar = queryExecute(
        "SELECT name, (SELECT COUNT(*) FROM qoqExtD) AS dcount FROM qoqExt WHERE id = 1",
        {}, { dbtype: "query" });
    assert("scalar subquery", qScalar.dcount[1], 2);

    // Derived table in FROM
    qDerived = queryExecute(
        "SELECT t.name AS n FROM (SELECT name, dept FROM qoqExt WHERE dept = 10) AS t ORDER BY t.name",
        {}, { dbtype: "query" });
    assert("derived table count", qDerived.recordCount, 2);
    assert("derived table first", qDerived.n[1], "Alice");

    // CASE expression (searched + simple) — Lucee QoQ has no CASE
    qCase = queryExecute(
        "SELECT name, CASE WHEN dept = 10 THEN 'eng' ELSE 'other' END AS band FROM qoqExt ORDER BY name",
        {}, { dbtype: "query" });
    assert("searched case eng", qCase.band[1], "eng");   // Alice dept 10
    assert("searched case other", qCase.band[2], "other"); // Bob dept 20
    qSimpleCase = queryExecute(
        "SELECT CASE dept WHEN 10 THEN 'E' WHEN 20 THEN 'S' END AS code FROM qoqExt WHERE id = 2",
        {}, { dbtype: "query" });
    assert("simple case", qSimpleCase.code[1], "S");
} else {
    writeOutput("       (skipped: LIMIT/scalar-subquery/derived-table not supported on this engine)" & chr(10));
}

suiteEnd();
</cfscript>
