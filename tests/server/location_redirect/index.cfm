<cfscript>
    // Exercises the script BIF `location()` (alias for <cflocation>). The form
    // is selected via url.form so one fixture covers named, positional, and
    // statusCode call shapes.
    form = structKeyExists(url, "form") ? url.form : "named";
    writeOutput("BEFORE_REDIRECT ");
    switch (form) {
        case "positional":
            location("/tests/server/location_redirect/landed.cfm", false);
            break;
        case "status":
            location(url="/tests/server/location_redirect/landed.cfm", statusCode=301, addToken=false);
            break;
        default:
            location(url="/tests/server/location_redirect/landed.cfm", addToken=false);
    }
    writeOutput("AFTER_REDIRECT");
</cfscript>
