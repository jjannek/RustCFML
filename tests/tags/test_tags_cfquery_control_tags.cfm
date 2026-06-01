<cfscript>
suiteBegin("Tags: cfquery control tags");

tmpfile = getTempDirectory() & "/rustcfml_cfquery_control_" & createUUID() & ".sqlite";
ds = "sqlite://" & tmpfile;
skip = false;

try {
    queryExecute("CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT)", [], { datasource: ds });
    queryExecute("INSERT INTO t (id, name) VALUES (1, 'alpha'), (2, 'beta'), (3, 'gamma')", [], { datasource: ds });
} catch (any e) {
    skip = true;
    assertTrue("cfquery control tags skipped (no sqlite): " & e.message, true);
}
</cfscript>

<cfif NOT skip>

<cfscript>
includeBeta = false;
filteredError = "";
</cfscript>
<cftry>
    <cfquery name="qFiltered" datasource="#ds#">
        SELECT * FROM t
        WHERE 1 = 1
        <cfif NOT includeBeta>
            AND name <> <cfqueryparam value="beta" cfsqltype="cf_sql_varchar">
        </cfif>
        ORDER BY id
    </cfquery>
    <cfcatch type="any">
        <cfset filteredError = cfcatch.message>
    </cfcatch>
</cftry>
<cfscript>
assert("cfquery cfif filtered query error", filteredError, "");
assert("cfquery cfif filters rows", structKeyExists(variables, "qFiltered") ? valueList(qFiltered.name) : "", "alpha,gamma");

includeBeta = true;
allError = "";
</cfscript>
<cftry>
    <cfquery name="qAll" datasource="#ds#">
        SELECT * FROM t
        WHERE 1 = 1
        <cfif NOT includeBeta>
            AND name <> <cfqueryparam value="beta" cfsqltype="cf_sql_varchar">
        </cfif>
        ORDER BY id
    </cfquery>
    <cfcatch type="any">
        <cfset allError = cfcatch.message>
    </cfcatch>
</cftry>
<cfscript>
assert("cfquery cfif unfiltered query error", allError, "");
assert("cfquery cfif omitted when false", structKeyExists(variables, "qAll") ? valueList(qAll.name) : "", "alpha,beta,gamma");

try { fileDelete(tmpfile); } catch (any e) {}
</cfscript>

</cfif>

<cfscript>
suiteEnd();
</cfscript>
