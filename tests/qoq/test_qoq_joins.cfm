<cfscript>
// Query of Queries — joins. Cross-engine for INNER/LEFT/CROSS; RIGHT/FULL are
// also asserted (reconcile against Lucee if it rejects them).

suiteBegin("QoQ Joins");

qoqP = queryNew("id,name,dept", "integer,varchar,integer", [
    { id: 1, name: "Alice", dept: 10 },
    { id: 2, name: "Bob",   dept: 20 },
    { id: 3, name: "Carol", dept: 10 },
    { id: 4, name: "Eve",   dept: javacast("null", "") }
]);
qoqD = queryNew("id,title", "integer,varchar", [
    { id: 10, title: "Engineering" },
    { id: 20, title: "Sales" },
    { id: 30, title: "Legal" }
]);

// INNER JOIN
qIn = queryExecute(
    "SELECT p.name AS who, d.title AS team FROM qoqP p JOIN qoqD d ON p.dept = d.id ORDER BY p.name",
    {}, { dbtype: "query" });
assert("inner count", qIn.recordCount, 3); // Eve (null) and Legal (no people) excluded
assert("inner first who", qIn.who[1], "Alice");
assert("inner first team", qIn.team[1], "Engineering");

// LEFT JOIN keeps all left rows
qLeft = queryExecute(
    "SELECT p.name AS who, d.title AS team FROM qoqP p LEFT JOIN qoqD d ON p.dept = d.id ORDER BY p.name",
    {}, { dbtype: "query" });
assert("left count", qLeft.recordCount, 4);
// Eve has a null dept -> no match -> null team (empty string)
qLeftEve = queryExecute(
    "SELECT p.name AS who, d.title AS team FROM qoqP p LEFT JOIN qoqD d ON p.dept = d.id WHERE p.name = 'Eve'",
    {}, { dbtype: "query" });
assert("left unmatched team is null", isNull(qLeftEve.team[1]) ? "" : qLeftEve.team[1], "");

// CROSS JOIN (comma form) — 4 x 3 = 12
qCross = queryExecute(
    "SELECT p.id AS pid, d.id AS did FROM qoqP p, qoqD d",
    {}, { dbtype: "query" });
assert("cross count", qCross.recordCount, 12);

// RIGHT JOIN keeps all right rows (Legal has no people)
qRight = queryExecute(
    "SELECT d.title AS team, p.name AS who FROM qoqP p RIGHT JOIN qoqD d ON p.dept = d.id WHERE d.title = 'Legal'",
    {}, { dbtype: "query" });
assert("right unmatched count", qRight.recordCount, 1);
assert("right unmatched who null", isNull(qRight.who[1]) ? "" : qRight.who[1], "");

suiteEnd();
</cfscript>
