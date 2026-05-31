<cfif thisTag.executionMode EQ "start">
    <cfset local.payload = {message = "ok"} />
    <cfset caller[attributes.returnContentVariable] = local.payload.message />
</cfif>
