<cfscript>
suiteBegin("Lifecycle: application-scope object method in custom tag");

// A CFC instance stored in the application scope (set in onApplicationStart)
// must remain callable from anywhere — including inside a custom tag. This is
// how a typical app exposes services and calls them from view/control tags
// (e.g. <cf_route> calling application.lib.db.getService("...")).
//
// This is a serve-mode lifecycle behavior, so it runs only when the suite is
// driven over HTTP (cgi.server_port present) and is skipped under the CLI
// runner. The fixture's index.cfm prints:
//   page=<result>;tag=<result>;
// where `page` calls the application-scoped object directly (control) and
// `tag` makes the same call from inside a cfmodule custom tag.

serverPort = structKeyExists(cgi, "server_port") ? trim(cgi.server_port) : "";

if (serverPort == "" || serverPort == "0") {
    assertTrue("application-scope custom-tag lifecycle skipped (no cgi.server_port)", true);
} else {
    targetPath = "/tests/lifecycle/application_scope_custom_tag/index.cfm";
    http url="http://127.0.0.1:#serverPort##targetPath#" method="GET" result="probeResult";
    body = trim(probeResult.filecontent);

    assert("custom-tag fixture status", probeResult.statuscode, "200 OK");
    // Control: a regular page can call the application-scoped object's method.
    assertTrue("regular page calls application-scope object method (control)",
        findNoCase("page=pong;", body) GT 0);
    // The behavior under test: the same call from inside a custom tag.
    assertTrue("custom tag calls application-scope object method",
        findNoCase("tag=pong;", body) GT 0);
}

suiteEnd();
</cfscript>
