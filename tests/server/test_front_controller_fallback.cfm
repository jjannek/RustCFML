<cfscript>
suiteBegin("Server: front controller fallback");

// Server-feature test: only runs when explicitly opted in via
// `?servertests=1`, because it needs the server started with a specific
// doc root + `server.fallback` config (see this dir's fixture). The default
// CLI/HTTP runner run skips it so it can't conflict with other suites.
serverPort = structKeyExists(cgi, "server_port") ? trim(cgi.server_port) : "";
runServerTests = serverPort != "" && serverPort != "0"
    && structKeyExists(url, "servertests") && url.servertests == "1";

if (!runServerTests) {
    assertTrue("front controller fallback skipped (server tests not enabled)", true);
} else {
    http
        url="http://127.0.0.1:#serverPort#/missing/path?probe=abc"
        method="GET"
        throwonerror="false"
        result="fallbackResult";

    assert("unresolved route is served by fallback template", trim(fallbackResult.fileContent), "fallback|route=/missing/path|probe=abc");
    assert("front controller fallback returns success status", fallbackResult.statusCode, "200 OK");
}

suiteEnd();
</cfscript>
