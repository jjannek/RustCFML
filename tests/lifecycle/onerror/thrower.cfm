<cfscript>
    writeOutput("BEFORE_THROW ");
    throw(type="Test.Uncaught", message="boom-uncaught");
</cfscript>
