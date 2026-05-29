<cfif thisTag.executionMode EQ "start">
    <cfset prefix = "start:" />
</cfif>

<cfif thisTag.executionMode EQ "end">
    <cfset caller[attributes.returnContentVariable] = prefix & thisTag.generatedContent />
    <cfset thisTag.generatedContent = "" />
</cfif>
