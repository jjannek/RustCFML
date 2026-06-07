<!---
    Gap fixture: a sweep of tag forms whose attribute value contains an embedded
    double quote (single-quoted attribute so the " is literal). These arms
    escaped the embedded quote with a backslash (value: "a\"b"), which the CFML
    lexer does not honor — a quote ends the string, so a quote is escaped by
    DOUBLING it ("") — leaving the value malformed and the component unable to
    PARSE. Behind <cfif false>: compiled, never executed.
--->
<cfcomponent output="false">
    <cffunction name="run" returntype="string" output="false">
        <cfif false>
            <cfheader name="X-Note" value='say "hi"' />
            <cfcookie name="c" value='a"b' />
            <cfdirectory action="list" directory='/tmp/"odd"' name="local.qd" />
            <cflock name='lock "x"' timeout="10" type="exclusive">
                <cfset local.noop = 1 />
            </cflock>
        </cfif>
        <cfreturn "ok" />
    </cffunction>
</cfcomponent>
