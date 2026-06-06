<cfscript>
// Query of Queries — aggregates, GROUP BY, HAVING. Cross-engine.

suiteBegin("QoQ Aggregates");

qoqEmp = queryNew("name,dept,salary", "varchar,varchar,integer", [
    { name: "Alice", dept: "eng",   salary: 100 },
    { name: "Bob",   dept: "sales", salary: 80  },
    { name: "Carol", dept: "eng",   salary: 120 },
    { name: "Dave",  dept: "sales", salary: 90  },
    { name: "Eve",   dept: "eng",   salary: 110 }
]);

// Aggregates with no GROUP BY (single row over all)
qAgg = queryExecute(
    "SELECT COUNT(*) AS c, SUM(salary) AS total, MIN(salary) AS lo, MAX(salary) AS hi, AVG(salary) AS mean FROM qoqEmp",
    {}, { dbtype: "query" });
assert("count(*)", qAgg.c[1], 5);
assert("sum", qAgg.total[1], 500);
assert("min", qAgg.lo[1], 80);
assert("max", qAgg.hi[1], 120);
assert("avg", qAgg.mean[1], 100);

// GROUP BY
qGrp = queryExecute(
    "SELECT dept, COUNT(*) AS n, SUM(salary) AS total FROM qoqEmp GROUP BY dept ORDER BY dept",
    {}, { dbtype: "query" });
assert("group count rows", qGrp.recordCount, 2);
assert("group eng dept", qGrp.dept[1], "eng");
assert("group eng n", qGrp.n[1], 3);
assert("group eng total", qGrp.total[1], 330);
assert("group sales total", qGrp.total[2], 170);

// GROUP BY + HAVING
qHav = queryExecute(
    "SELECT dept, COUNT(*) AS n FROM qoqEmp GROUP BY dept HAVING COUNT(*) > 2 ORDER BY dept",
    {}, { dbtype: "query" });
assert("having rows", qHav.recordCount, 1);
assert("having dept", qHav.dept[1], "eng");

// COUNT(column) ignores NULLs
qCntSrc = queryNew("id,opt", "integer,varchar", [
    { id: 1, opt: "a" }, { id: 2, opt: javacast("null", "") }, { id: 3, opt: "c" }
]);
qCnt = queryExecute(
    "SELECT COUNT(opt) AS nonnull, COUNT(*) AS allrows FROM qCntSrc",
    {}, { dbtype: "query" });
assert("count(col) skips null", qCnt.nonnull[1], 2);
assert("count(*) includes null rows", qCnt.allrows[1], 3);

// COUNT(DISTINCT ...)
qCd = queryExecute(
    "SELECT COUNT(DISTINCT dept) AS d FROM qoqEmp",
    {}, { dbtype: "query" });
assert("count distinct", qCd.d[1], 2);

suiteEnd();
</cfscript>
