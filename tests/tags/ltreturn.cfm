<cfif thisTag.executionMode NEQ "end">
    <cfset attributes.content_class = "px-5 py-5">
    <cfreturn>
</cfif>

<cfif len(trim(thisTag.generatedContent))>
    <cfset attributes.body = thisTag.generatedContent>
    <cfset thisTag.generatedContent = "">
</cfif>

<cfoutput><main class="#attributes.content_class#">#attributes.body#</main></cfoutput>
