<cfscript>
suiteBegin("Lifecycle: Application.cfc load errors");

// Server-feature test: only runs when explicitly opted in via
// `?servertests=1`. It needs the server rooted so `/tests/...` resolves and
// the default `errorStatusCode` (5xx on error). The default runner run skips
// it so it can't conflict with suites that load a different server config.
serverPort = structKeyExists(cgi, "server_port") ? trim(cgi.server_port) : "";
runServerTests = serverPort != "" && serverPort != "0"
    && structKeyExists(url, "servertests") && url.servertests == "1";

if (!runServerTests) {
    assertTrue("Application.cfc load error skipped (server tests not enabled)", true);
} else {
    targetPath = "/tests/lifecycle/application_load_error/index.cfm";
    http url="http://127.0.0.1:#serverPort##targetPath#" method="GET" throwonerror="false" result="loadErrorResult";

    assertTrue("Application.cfc load error returns failure status",
        left(loadErrorResult.statuscode, 1) == "5");
    assertTrue("Application.cfc load error does not execute target page",
        findNoCase("direct-page-executed", loadErrorResult.filecontent) == 0);
}

suiteEnd();
</cfscript>
