<cfscript>
// Query of Queries — SELECT / WHERE / ORDER BY / DISTINCT / LIMIT / scalar fns.
// Cross-engine (RustCFML + Lucee). No `var` at page scope; alias computed columns.

suiteBegin("QoQ Select");

qoqPeople = queryNew("id,name,age,dept", "integer,varchar,integer,integer", [
    { id: 1, name: "Alice", age: 30, dept: 10 },
    { id: 2, name: "Bob",   age: 25, dept: 20 },
    { id: 3, name: "Carol", age: 40, dept: 10 },
    { id: 4, name: "Dave",  age: 35, dept: 20 },
    { id: 5, name: "Eve",   age: 28, dept: 10 }
]);

// Basic projection + WHERE + ORDER BY
qSel = queryExecute(
    "SELECT name, age FROM qoqPeople WHERE age >= 30 ORDER BY age DESC",
    {}, { dbtype: "query" });
assert("where/order recordCount", qSel.recordCount, 3);
assert("order desc first", qSel.name[1], "Carol");
assert("order desc last",  qSel.name[3], "Alice");

// ORDER BY ascending, multiple keys
qOrd = queryExecute(
    "SELECT name FROM qoqPeople ORDER BY dept ASC, age ASC",
    {}, { dbtype: "query" });
assert("multi-key order first", qOrd.name[1], "Eve");   // dept 10, age 28
assert("multi-key order second", qOrd.name[2], "Alice"); // dept 10, age 30

// DISTINCT
qDist = queryExecute(
    "SELECT DISTINCT dept FROM qoqPeople ORDER BY dept",
    {}, { dbtype: "query" });
assert("distinct count", qDist.recordCount, 2);
assert("distinct first", qDist.dept[1], 10);

// TOP n (cross-engine; Lucee QoQ uses TOP, not LIMIT — see test_qoq_rustcfml_ext
// for LIMIT/OFFSET, a RustCFML/BoxLang superset feature)
qTop = queryExecute(
    "SELECT TOP 2 name FROM qoqPeople ORDER BY name",
    {}, { dbtype: "query" });
assert("top count", qTop.recordCount, 2);
assert("top first", qTop.name[1], "Alice");
assert("top second", qTop.name[2], "Bob");

// Scalar functions + alias (LENGTH, not LEN — HSQLDB/Lucee name)
qFn = queryExecute(
    "SELECT UPPER(name) AS u, LENGTH(name) AS nlen FROM qoqPeople WHERE id = 1",
    {}, { dbtype: "query" });
assert("upper()", qFn.u[1], "ALICE");
assert("length()", qFn.nlen[1], 5);

// BETWEEN + IN list
qRange = queryExecute(
    "SELECT name FROM qoqPeople WHERE age BETWEEN 28 AND 35 ORDER BY age",
    {}, { dbtype: "query" });
assert("between count", qRange.recordCount, 3); // 28,30,35
assert("between first", qRange.name[1], "Eve");

qIn = queryExecute(
    "SELECT name FROM qoqPeople WHERE id IN (1, 3, 5) ORDER BY name",
    {}, { dbtype: "query" });
assert("in-list count", qIn.recordCount, 3);
assert("in-list first", qIn.name[1], "Alice");

// LIKE (data is mixed-case; anchor on the literal upper-case initial so the
// assertion holds regardless of engine LIKE case-sensitivity)
qLike = queryExecute(
    "SELECT name FROM qoqPeople WHERE name LIKE 'A%' ORDER BY name",
    {}, { dbtype: "query" });
assert("like A% count", qLike.recordCount, 1);
assert("like A% match", qLike.name[1], "Alice");

// IS NULL / IS NOT NULL
qNullSrc = queryNew("id,note", "integer,varchar", [
    { id: 1, note: "x" }, { id: 2, note: javacast("null", "") }, { id: 3, note: "y" }
]);
qNotNull = queryExecute(
    "SELECT id FROM qNullSrc WHERE note IS NOT NULL ORDER BY id",
    {}, { dbtype: "query" });
assert("is not null count", qNotNull.recordCount, 2);
qIsNull = queryExecute(
    "SELECT id FROM qNullSrc WHERE note IS NULL",
    {}, { dbtype: "query" });
assert("is null count", qIsNull.recordCount, 1);
assert("is null id", qIsNull.id[1], 2);

// Integer-preserving arithmetic
qMath = queryExecute(
    "SELECT id + 1 AS plus, id * id AS sq FROM qoqPeople WHERE id = 3",
    {}, { dbtype: "query" });
assert("int add", qMath.plus[1], 4);
assert("int mul", qMath.sq[1], 9);

suiteEnd();
</cfscript>
