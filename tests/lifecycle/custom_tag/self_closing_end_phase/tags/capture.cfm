<cfif thisTag.executionMode EQ "start">
    <cfset savedValue = attributes.value />
</cfif>

<cfif thisTag.executionMode EQ "end">
    <cfset caller[attributes.returnContentVariable] = savedValue />
</cfif>
