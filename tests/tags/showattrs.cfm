<cfif thisTag.executionMode EQ "start">
    <cfif structKeyExists(attributes, "placeholder")>
        <cfset labelText = attributes.placeholder>
    <cfelse>
        <cfset labelText = attributes.label>
    </cfif>
    <cfoutput>#attributes.model#|#labelText#|#attributes.class#</cfoutput>
</cfif>
