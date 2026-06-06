<!---
    Gap fixture: <cfexecute> with an interpolated timeout (timeout="#...#") and an
    interpolated arguments value. Both were emitted verbatim, leaving literal
    "#...#" in the generated __cfexecute({...}) call so the component failed to
    PARSE (surfacing at createObject() time as "Could not find the component" --
    hence the runtime-instantiated fixture). The call is behind <cfif false> so it
    is compiled but never executed (no process is spawned).
--->
<cfcomponent output="false">
    <cffunction name="run" returntype="string" output="false">
        <cfargument name="timeout" default="5" />
        <cfargument name="args" default="hello" />
        <cfif false>
            <cfexecute name="/bin/echo" arguments="#arguments.args#" timeout="#arguments.timeout#" variable="local.out"></cfexecute>
        </cfif>
        <cfreturn "ok" />
    </cffunction>
</cfcomponent>
