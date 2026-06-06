<!---
    Gap fixture: a tag-form <cftry>/<cfcatch>/<cffinally> on the exception path.
    The try body throws, the catch handles it, and the finally must still run
    (Lucee/ACF parity): run() returns "try,catch,finally". Same parse-time root
    cause and fixture rationale as CffinallyHappyPathFixture; this case
    additionally pins that <cffinally> runs after a caught exception.
--->
<cfcomponent output="false">
    <cffunction name="run" returntype="string" output="false">
        <cfset var steps = [] />
        <cftry>
            <cfset arrayAppend(steps, "try") />
            <cfthrow message="boom" />
            <cfcatch type="any">
                <cfset arrayAppend(steps, "catch") />
            </cfcatch>
            <cffinally>
                <cfset arrayAppend(steps, "finally") />
            </cffinally>
        </cftry>
        <cfreturn arrayToList(steps) />
    </cffunction>
</cfcomponent>
