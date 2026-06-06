<!---
    Control fixture: a tag-form <cftry>/<cfcatch> with NO <cffinally>, written
    with normal multi-line formatting (whitespace/newlines between the tags).
    RustCFML already parses this, so it is the regression guard proving the
    fixture wiring (createObject + run) and the try/catch transpilation are
    sound. A failure on the sibling cffinally fixtures therefore isolates to the
    <cffinally> block specifically, not to <cftry> handling in general.
--->
<cfcomponent output="false">
    <cffunction name="run" returntype="string" output="false">
        <cfset var steps = [] />
        <cftry>
            <cfset arrayAppend(steps, "try") />
            <cfcatch type="any">
                <cfset arrayAppend(steps, "catch") />
            </cfcatch>
        </cftry>
        <cfreturn arrayToList(steps) />
    </cffunction>
</cfcomponent>
