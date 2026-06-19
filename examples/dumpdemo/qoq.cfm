<!doctype html>
<html><head><meta charset="utf-8"><title>RustCFML QoQ writeDump</title>
<style>body{font-family:system-ui,sans-serif;margin:24px;max-width:1200px} h2{margin-top:32px;border-bottom:2px solid #4e6e81;padding-bottom:4px;color:#3a5563} p{color:#555} code{background:#f3eee9;padding:1px 4px;border-radius:3px} pre{background:#f3eee9;padding:8px 12px;border-radius:4px;overflow:auto}</style>
</head><body>
<p><a href="index.cfm" style="color:#b7410e">&larr; back to examples</a></p>
<h1>RustCFML — Query-of-Queries <code>writeDump</code></h1>
<p>Each query box footer shows <b>Records • Execution time (ms) • SQL</b>. All run through the pure-Rust QoQ engine.</p>

<cfscript>
// ---- Source data: 100 employees across 8 departments ----
depts = ["Engineering","Sales","Marketing","Support","Finance","HR","Legal","Ops"];
emp = queryNew("id,name,deptId,salary,active,hireYear", "integer,varchar,integer,decimal,bit,integer");
for (i = 1; i <= 4000; i++) {
    queryAddRow(emp);
    querySetCell(emp, "id", i);
    querySetCell(emp, "name", "Employee_" & i);
    querySetCell(emp, "deptId", (i mod 8) + 1);
    querySetCell(emp, "salary", 40000 + ((i * 137) mod 60000));
    querySetCell(emp, "active", (i mod 7 neq 0));
    querySetCell(emp, "hireYear", 2010 + (i mod 15));
}
dept = queryNew("deptId,deptName", "integer,varchar");
for (d = 1; d <= 8; d++) {
    queryAddRow(dept);
    querySetCell(dept, "deptId", d);
    querySetCell(dept, "deptName", depts[d]);
}

// ---- Inline aggregate UDF: group_concat(col) -> "a, b, c" ----
queryRegisterFunction("group_concat", function(vals) {
    var out = "";
    for (var v in vals) { out = len(out) ? out & ", " & v : v; }
    return out;
}, "aggregate");
</cfscript>

<h2>1. Raw 4000-row employee query (collapsed — click to expand)</h2>
<cfscript>
all = queryExecute("SELECT id, name, deptId, salary, active, hireYear FROM emp ORDER BY id", [], {dbtype:"query"});
writeDump(var=all, label="4000 employees", expand=false);
</cfscript>

<h2>2. JOIN + GROUP BY + aggregates + HAVING, ordered by avg salary</h2>
<cfscript>
summary = queryExecute("
    SELECT
        d.deptName,
        COUNT(e.id)   AS headcount,
        AVG(e.salary) AS avgSalary,
        MAX(e.salary) AS topSalary,
        MIN(e.salary) AS lowSalary,
        SUM(e.salary) AS payroll
    FROM emp e
    INNER JOIN dept d ON e.deptId = d.deptId
    WHERE e.active = 1
    GROUP BY d.deptName
    HAVING COUNT(e.id) > 5
    ORDER BY AVG(e.salary) DESC
", [], {dbtype:"query"});
writeDump(summary);
</cfscript>

<h2>3. Inline UDF <code>group_concat</code> — member names per department</h2>
<cfscript>
rosters = queryExecute("
    SELECT d.deptName,
           COUNT(e.id)          AS headcount,
           group_concat(e.name) AS members
    FROM emp e
    INNER JOIN dept d ON e.deptId = d.deptId
    WHERE e.active = 1
    GROUP BY d.deptName
    ORDER BY COUNT(e.id) DESC
", [], {dbtype:"query"});
writeDump(rosters);
</cfscript>

<h2>4. Filtered: high earners hired 2018+</h2>
<cfscript>
earners = queryExecute("
    SELECT e.name, d.deptName, e.salary, e.hireYear
    FROM emp e
    INNER JOIN dept d ON e.deptId = d.deptId
    WHERE e.salary > 70000 AND e.hireYear >= 2018 AND e.active = 1
    ORDER BY e.salary DESC
", [], {dbtype:"query"});
writeDump(var=earners, label="High earners (2018+)");
</cfscript>

<h2>5. Heavy: self-join (~2M intermediate rows) — watch the execution time</h2>
<p>Joins <code>emp</code> to itself within each department, then aggregates. This is where the QoQ engine actually breaks a sweat.</p>
<cfscript>
pairs = queryExecute("
    SELECT e1.deptId,
           COUNT(e2.id)   AS pairCount,
           AVG(e2.salary) AS avgPeerSalary
    FROM emp e1
    INNER JOIN emp e2 ON e1.deptId = e2.deptId
    WHERE e1.active = 1
    GROUP BY e1.deptId
    ORDER BY e1.deptId
", [], {dbtype:"query"});
writeDump(pairs);
</cfscript>

</body></html>
