<cfscript>
suiteBegin("cfqueryparam: interpolated value attribute");

tmpfile = getTempDirectory() & "/rustcfml_qparam_interp_" & createUUID() & ".sqlite";
ds = "sqlite://" & tmpfile;
skip = false;

try {
    queryExecute("CREATE TABLE t (id INTEGER PRIMARY KEY, search_value TEXT)", [], { datasource: ds });
    queryExecute("INSERT INTO t (id, search_value) VALUES (1, '%mat%'), (2, '%other%')", [], { datasource: ds });
} catch (any e) {
    skip = true;
    assertTrue("cfqueryparam interpolated value skipped (no sqlite): " & e.message, true);
}
</cfscript>

<cfif NOT skip>

<cfscript>
url = { q: "mat" };
</cfscript>

<cfquery name="q" datasource="#ds#">
    SELECT search_value
    FROM t
    WHERE search_value = <cfqueryparam value="%#url.q#%" cfsqltype="cf_sql_varchar">
</cfquery>

<cfscript>
assert("cfqueryparam preserves literal text around interpolated value", q.search_value[1], "%mat%");
assert("cfqueryparam interpolated value finds one row", q.recordCount, 1);

try { fileDelete(tmpfile); } catch (any e) {}
</cfscript>

</cfif>

<cfscript>suiteEnd();</cfscript>
