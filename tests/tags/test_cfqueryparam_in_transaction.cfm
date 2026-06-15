<cfscript>
suiteBegin("cfqueryparam: binds inside cftransaction issue 147");

tmpfile = getTempDirectory() & "/rustcfml_qparam_txn_" & createUUID() & ".sqlite";
ds = "sqlite://" & tmpfile;
skip = false;

try {
    queryExecute("CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT, qty INTEGER)", [], { datasource: ds });
} catch (any e) {
    skip = true;
    assertTrue("cfqueryparam-in-transaction skipped (no sqlite): " & e.message, true);
}
</cfscript>

<cfif NOT skip>

<!--- Baseline: cfqueryparam OUTSIDE a transaction binds correctly --->
<cfquery datasource="#ds#">
    INSERT INTO t (name, qty) VALUES (
        <cfqueryparam value="OUT" cfsqltype="cf_sql_varchar">,
        <cfqueryparam value="10" cfsqltype="cf_sql_integer">
    )
</cfquery>

<!--- The bug: cfqueryparam INSIDE a transaction was stringified into the SQL
      as a struct representation instead of being bound. --->
<cftransaction>
    <cfquery datasource="#ds#">
        INSERT INTO t (name, qty) VALUES (
            <cfqueryparam value="IN" cfsqltype="cf_sql_varchar">,
            <cfqueryparam value="20" cfsqltype="cf_sql_integer">
        )
    </cfquery>
</cftransaction>

<cfquery name="q" datasource="#ds#">
    SELECT name, qty FROM t ORDER BY id
</cfquery>

<cfscript>
assert("param bound outside transaction (baseline)", q.name[1], "OUT");
assert("integer param bound outside transaction", q.qty[1], 10);
assert("cfqueryparam bound (not stringified) inside transaction", q.name[2], "IN");
assert("integer cfqueryparam bound inside transaction", q.qty[2], 20);

try { fileDelete(tmpfile); } catch (any e) {}
</cfscript>

</cfif>

<cfscript>suiteEnd();</cfscript>
