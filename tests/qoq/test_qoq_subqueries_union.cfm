<cfscript>
// Query of Queries — subqueries (scalar / IN / derived) + UNION + params. Cross-engine.

suiteBegin("QoQ Subqueries & Union");

qoqP = queryNew("id,name,dept", "integer,varchar,integer", [
    { id: 1, name: "Alice", dept: 10 },
    { id: 2, name: "Bob",   dept: 20 },
    { id: 3, name: "Carol", dept: 10 }
]);
qoqD = queryNew("id,title", "integer,varchar", [
    { id: 10, title: "Engineering" },
    { id: 20, title: "Sales" }
]);

// IN subquery
qInSub = queryExecute(
    "SELECT name FROM qoqP WHERE dept IN (SELECT id FROM qoqD WHERE title = 'Engineering') ORDER BY name",
    {}, { dbtype: "query" });
assert("in-subquery count", qInSub.recordCount, 2);
assert("in-subquery first", qInSub.name[1], "Alice");

// (Scalar subqueries in SELECT and derived FROM tables are a RustCFML/BoxLang
// superset — Lucee QoQ rejects them — so they live in test_qoq_rustcfml_ext.)

// UNION (distinct) — overlapping ids collapse
qUnion = queryExecute(
    "SELECT id FROM qoqP WHERE id <= 2 UNION SELECT id FROM qoqP WHERE id >= 2 ORDER BY id",
    {}, { dbtype: "query" });
assert("union distinct count", qUnion.recordCount, 3); // {1,2} U {2,3} = {1,2,3}

// UNION ALL keeps duplicates
qUnionAll = queryExecute(
    "SELECT id FROM qoqP WHERE id <= 2 UNION ALL SELECT id FROM qoqP WHERE id >= 2",
    {}, { dbtype: "query" });
assert("union all count", qUnionAll.recordCount, 4); // 2 + 2

// Positional parameter
qPos = queryExecute(
    "SELECT name FROM qoqP WHERE dept = ? ORDER BY name",
    [ 10 ], { dbtype: "query" });
assert("positional param count", qPos.recordCount, 2);
assert("positional param first", qPos.name[1], "Alice");

// Named parameter
qNamed = queryExecute(
    "SELECT name FROM qoqP WHERE dept = :d ORDER BY name",
    { d: 20 }, { dbtype: "query" });
assert("named param count", qNamed.recordCount, 1);
assert("named param match", qNamed.name[1], "Bob");

// returntype = array
qArr = queryExecute(
    "SELECT name FROM qoqP ORDER BY name",
    {}, { dbtype: "query", returntype: "array" });
assert("returntype array len", arrayLen(qArr), 3);
assert("returntype array first", qArr[1].name, "Alice");

suiteEnd();
</cfscript>
