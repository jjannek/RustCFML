<!---
    Gap fixture: a <cfquery> whose body contains double-quoted SQL identifiers
    (AS "where", AS "myCol"). The SQL body is emitted as a double-quoted CFML
    string with embedded " escaped as \" (backslash); CFML does not treat \" as
    an escaped quote (a quote is escaped by doubling it, ""), so the embedded
    quote terminated the string early and the component failed to PARSE. The
    failure surfaces at createObject() time as "Could not find the component",
    hence the runtime-instantiated fixture. The query is guarded by <cfif false>
    so it is compiled but never executed.
--->
<cfcomponent output="false">
    <cffunction name="run" returntype="string" output="false">
        <cfif false>
            <cfquery name="local.q" datasource="probe">
                SELECT a AS "where", b AS "myCol" FROM t WHERE status = 'open'
            </cfquery>
        </cfif>
        <cfreturn "ok" />
    </cffunction>
</cfcomponent>
