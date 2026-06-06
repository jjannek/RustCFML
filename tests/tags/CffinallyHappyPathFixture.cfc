<!---
    Gap fixture: a tag-form <cftry>/<cfcatch>/<cffinally> on the no-exception
    path, written with normal multi-line formatting. The <cffinally> body must
    parse AND run (Lucee/ACF parity): run() returns "try,finally".

    On RustCFML the whitespace between </cfcatch> and <cffinally> was emitted as
    __writeText(...) in the structural gap of the generated try statement
    ("} __writeText(); finally {"), so any normally-formatted try/catch/finally
    failed to PARSE. The parse failure surfaces at createObject() time as
    "Could not find the component", which is why this lives in a fixture (an
    inline parse error escapes try/catch and would abort the runner).
--->
<cfcomponent output="false">
    <cffunction name="run" returntype="string" output="false">
        <cfset var steps = [] />
        <cftry>
            <cfset arrayAppend(steps, "try") />
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
