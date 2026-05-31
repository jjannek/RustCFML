<cfif thisTag.executionMode EQ "start">
    <cfset local.payload = { message = "ok" }>
    <cfset caller[attributes.outVar] = local.payload.message>
</cfif>
