<cfscript>suiteBegin("Tags: cfqueryparam attributeCollection");

// Backed by SQLite — built into RustCFML's default features. Lucee can run
// this too provided a "sqlitedb" datasource is configured; if not, the suite
// short-circuits with a single skip-pass.
tmpfile = getTempDirectory() & "/rustcfml_qparam_ac_" & createUUID() & ".sqlite";
ds = "sqlite://" & tmpfile;
skip = false;

try {
    queryExecute("CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT, n INTEGER)", [], { datasource: ds });
    queryExecute("INSERT INTO t (id, name, n) VALUES (1, 'alpha', 10), (2, 'beta', 20), (3, 'gamma', 30)", [], { datasource: ds });
} catch (any e) {
    skip = true;
    assertTrue("cfqueryparam attributeCollection skipped (no sqlite): " & e.message, true);
}
</cfscript>

<cfif NOT skip>

<cfscript>
// Case 1: attributeCollection alone supplies value + cfsqltype
params1 = { value: "beta", cfsqltype: "cf_sql_varchar" };
</cfscript>
<cfquery name="q1" datasource="#ds#">
    SELECT * FROM t WHERE name = <cfqueryparam attributeCollection="#params1#">
</cfquery>
<cfscript>
assert("AC alone: recordCount", q1.recordCount, 1);
assert("AC alone: id matches", q1.id[1], 2);

// Case 2: explicit value= overrides AC's value (explicit attrs win)
params2 = { value: "beta", cfsqltype: "cf_sql_varchar" };
</cfscript>
<cfquery name="q2" datasource="#ds#">
    SELECT * FROM t WHERE name = <cfqueryparam attributeCollection="#params2#" value="gamma">
</cfquery>
<cfscript>
assert("explicit value overrides AC", q2.id[1], 3);

// Case 3: AC supplies value, explicit cfsqltype mixes in
params3 = { value: 20 };
</cfscript>
<cfquery name="q3" datasource="#ds#">
    SELECT * FROM t WHERE n = <cfqueryparam attributeCollection="#params3#" cfsqltype="cf_sql_integer">
</cfquery>
<cfscript>
assert("AC value + explicit cfsqltype", q3.id[1], 2);

// Case 4: AC must not be mutated when explicit overrides applied
params4 = { value: "alpha", cfsqltype: "cf_sql_varchar" };
</cfscript>
<cfquery name="q4" datasource="#ds#">
    SELECT * FROM t WHERE name = <cfqueryparam attributeCollection="#params4#" value="gamma">
</cfquery>
<cfscript>
assert("AC source struct unmutated (value)", params4.value, "alpha");
assert("AC source struct unmutated (cfsqltype)", params4.cfsqltype, "cf_sql_varchar");

try { fileDelete(tmpfile); } catch (any e) {}
</cfscript>

</cfif>

<cfscript>suiteEnd();</cfscript>
