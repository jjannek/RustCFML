<cfif not thisTag.HasEndTag>
    <cfabort showerror="Must have an end tag..." />
</cfif>

<cfif thisTag.executionMode EQ "end">
    <cfset caller[attributes.returnContentVariable] = thisTag.HasEndTag />
</cfif>
