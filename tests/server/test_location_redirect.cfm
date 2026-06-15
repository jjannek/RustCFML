<cfscript>
suiteBegin("Server: location() / cflocation redirect");

// Server-feature test (gated on `?servertests=1`): a redirect only manifests as
// an HTTP 3xx + Location header in serve mode. Regression for the
// `location()` script BIF reaching the stub ("__cflocation requires VM
// intercept", reported with Wheels `?reload=true`).
serverPort = structKeyExists(cgi, "server_port") ? trim(cgi.server_port) : "";
runServerTests = serverPort != "" && serverPort != "0"
    && structKeyExists(url, "servertests") && url.servertests == "1";

if (!runServerTests) {
    assertTrue("location() redirect skipped (server tests not enabled)", true);
} else {
    base = "http://127.0.0.1:#serverPort#/tests/server/location_redirect/index.cfm";

    // Named-arg form: location(url=..., addToken=false) → 302 + Location header,
    // and the page does NOT continue past the redirect.
    http url="#base#?form=named" method="GET" redirect="false" throwonerror="false" result="named";
    assert("named location() returns 302", left(named.statusCode, 3), "302");
    assertTrue("named location() sets Location header",
        findNoCase("/tests/server/location_redirect/landed.cfm", named.responseHeader["Location"]) > 0);
    assertTrue("code after named location() does not run",
        findNoCase("AFTER_REDIRECT", named.fileContent) == 0);

    // Positional form: location(url, addToken).
    http url="#base#?form=positional" method="GET" redirect="false" throwonerror="false" result="positional";
    assert("positional location() returns 302", left(positional.statusCode, 3), "302");

    // statusCode override.
    http url="#base#?form=status" method="GET" redirect="false" throwonerror="false" result="statusForm";
    assert("location() honours statusCode=301", left(statusForm.statusCode, 3), "301");
}

suiteEnd();
</cfscript>
