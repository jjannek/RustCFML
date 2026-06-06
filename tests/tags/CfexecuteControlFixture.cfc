<!---
    Control fixture: <cfexecute> with literal timeout/arguments. RustCFML already
    parses this, so it guards the fixture wiring. The call is behind <cfif false>
    so it is compiled but never executed (no process is spawned); this test is
    about PARSE-time attribute handling, not delivery.
--->
<cfcomponent output="false">
    <cffunction name="run" returntype="string" output="false">
        <cfif false>
            <cfexecute name="/bin/echo" arguments="hello" timeout="5" variable="local.out"></cfexecute>
        </cfif>
        <cfreturn "ok" />
    </cffunction>
</cfcomponent>
