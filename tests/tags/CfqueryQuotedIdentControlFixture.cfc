<!---
    Control fixture: a <cfquery> whose body uses only single-quoted SQL string
    literals (no double-quoted identifiers). RustCFML already parses this, so it
    guards the cfquery-in-fixture wiring. The query is guarded by <cfif false>
    so it is compiled but never executed (no datasource needed); this test is
    about PARSE-time handling of the SQL body.
--->
<cfcomponent output="false">
    <cffunction name="run" returntype="string" output="false">
        <cfif false>
            <cfquery name="local.q" datasource="probe">
                SELECT a FROM t WHERE status = 'open'
            </cfquery>
        </cfif>
        <cfreturn "ok" />
    </cffunction>
</cfcomponent>
