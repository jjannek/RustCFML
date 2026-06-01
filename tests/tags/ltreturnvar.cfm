<cfif thisTag.executionMode NEQ "end">
    <cfset variables.startToken = "tok-" & attributes.seed>
    <cfreturn>
</cfif>

<cfset thisTag.generatedContent = "">
<cfoutput>[#variables.startToken#]</cfoutput>
