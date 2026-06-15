<cfscript>
    writeOutput("BEFORE_SHOWERROR ");
</cfscript>
<cfabort showError="aborted-with-error">
<cfscript>
    writeOutput("AFTER_SHOWERROR");
</cfscript>
