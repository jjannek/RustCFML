<cfif thisTag.executionMode EQ "start">
    <cfset savedValue = attributes.value>
</cfif>

<cfif thisTag.executionMode EQ "end">
    <cfset caller[attributes.outVar] = savedValue>
    <cfset caller[attributes.hetVar] = thisTag.hasEndTag>
</cfif>
