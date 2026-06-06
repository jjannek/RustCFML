<!---
    Gap fixture: <cfexecute> whose arguments value contains an embedded double
    quote (single-quoted attribute so the " is literal). The embedded quote was
    escaped with a backslash (arguments: "say \"hi\""), which CFML does not honor
    -- a quote is escaped by doubling it -- so the literal terminated early and
    the component failed to PARSE. Behind <cfif false>: compiled, never executed.
--->
<cfcomponent output="false">
    <cffunction name="run" returntype="string" output="false">
        <cfif false>
            <cfexecute name="/bin/echo" arguments='say "hi"' variable="local.out"></cfexecute>
        </cfif>
        <cfreturn "ok" />
    </cffunction>
</cfcomponent>
