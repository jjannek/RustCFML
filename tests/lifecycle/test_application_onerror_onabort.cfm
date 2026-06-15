<cfscript>
suiteBegin("Lifecycle: Application.cfc onError / onAbort");

// Server-feature test: onError/onAbort only fire under the full request
// lifecycle (serve mode with a live application). Gated on `?servertests=1`
// just like the Application.cfc load-error suite; the default CLI run skips it.
serverPort = structKeyExists(cgi, "server_port") ? trim(cgi.server_port) : "";
runServerTests = serverPort != "" && serverPort != "0"
    && structKeyExists(url, "servertests") && url.servertests == "1";

if (!runServerTests) {
    assertTrue("onError/onAbort skipped (server tests not enabled)", true);
} else {
    // --- onError: an uncaught exception is handed to Application.cfc::onError,
    //     which renders the response and suppresses the default error page.
    onErrorPath = "/tests/lifecycle/onerror/thrower.cfm";
    http url="http://127.0.0.1:#serverPort##onErrorPath#" method="GET" throwonerror="false" result="onErrorResult";

    assertTrue("onError fires for uncaught exception and writes its output",
        findNoCase("ONERROR_FIRED:boom-uncaught", onErrorResult.filecontent) > 0);
    assertTrue("onError receives empty eventName for a target-page exception",
        findNoCase("EVENT[]", onErrorResult.filecontent) > 0);

    // --- onAbort: a cfabort unwind fires onAbort in place of onRequestEnd.
    onAbortPath = "/tests/lifecycle/onabort/aborter.cfm";
    http url="http://127.0.0.1:#serverPort##onAbortPath#" method="GET" throwonerror="false" result="onAbortResult";

    assertTrue("onAbort fires on cfabort",
        findNoCase("ONABORT_FIRED", onAbortResult.filecontent) > 0);
    assertTrue("output before abort is preserved",
        findNoCase("BEFORE_ABORT", onAbortResult.filecontent) > 0);
    assertTrue("code after abort does not run",
        findNoCase("AFTER_ABORT", onAbortResult.filecontent) == 0);
    assertTrue("onRequestEnd does not fire when the request aborts",
        findNoCase("ONREQUESTEND_FIRED", onAbortResult.filecontent) == 0);

    // --- cfabort showError="..." is a CATCHABLE error: it fires onError (with
    //     the showError message), NOT onAbort (Adobe/Lucee parity).
    showErrorPath = "/tests/lifecycle/onabort/showerror.cfm";
    http url="http://127.0.0.1:#serverPort##showErrorPath#" method="GET" throwonerror="false" result="showErrorResult";

    assertTrue("cfabort showError fires onError",
        findNoCase("ONERROR_FIRED:aborted-with-error", showErrorResult.filecontent) > 0);
    assertTrue("cfabort showError does NOT fire onAbort",
        findNoCase("ONABORT_FIRED", showErrorResult.filecontent) == 0);
    assertTrue("code after cfabort showError does not run",
        findNoCase("AFTER_SHOWERROR", showErrorResult.filecontent) == 0);
    assertTrue("onRequestEnd does not fire when onError handles the error",
        findNoCase("ONREQUESTEND_FIRED", showErrorResult.filecontent) == 0);
}

suiteEnd();
</cfscript>
