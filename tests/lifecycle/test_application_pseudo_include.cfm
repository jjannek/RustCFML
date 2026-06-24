<cfscript>
suiteBegin("Lifecycle: Application.cfc pseudo-constructor relative include");

// Regression: a relative `include` in the Application.cfc pseudo-constructor
// must resolve against the Application.cfc's own directory, not the requested
// (possibly deep) target page. A deep page (`.../sub/page.cfm`) triggers the
// Application.cfc at `application_pseudo_include/`, whose body does
// `include "shared_config.cfm"`. Before the fix the include resolved relative
// to the target page's dir (`.../sub/`), missing the file (Wheels: a
// `<webroot>/public/Application.cfc` doing `include "../config/app.cfm"`
// triggered from `/tests/runner.cfm` escaped to the wrong directory).

serverPort = structKeyExists(cgi, "server_port") ? trim(cgi.server_port) : "";

if (serverPort == "" || serverPort == "0") {
    assertTrue("pseudo-constructor include lifecycle skipped (no cgi.server_port)", true);
} else {
    targetPath = "/tests/lifecycle/application_pseudo_include/sub/page.cfm";
    http url="http://127.0.0.1:#serverPort##targetPath#" method="GET" result="incResult";
    assert("pseudo-include request status", incResult.statuscode, "200 OK");
    assert("Application.cfc relative include resolved against its own dir",
        trim(incResult.filecontent), "included=ok");
}

suiteEnd();
</cfscript>
