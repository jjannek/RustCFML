<cfscript>
suiteBegin("CFScript Tag Statements");

// ============================================================
// setting (cfsetting) — safe, no HTTP side-effects
// ============================================================

setting requesttimeout="60";
assertTrue("setting requesttimeout parsed", true);

setting showdebugoutput="false";
assertTrue("setting showdebugoutput parsed", true);

// ============================================================
// log (cflog) — safe, no HTTP side-effects
// ============================================================

log text="Test log message" type="information";
assertTrue("log text/type parsed", true);

log text="Debug message" type="debug" file="testlog";
assertTrue("log with file parsed", true);

// Hash interpolation inside quoted log attributes. CFML supports the bare
// `log text=...` statement form and the parenthesized `cflog(...)` call form;
// both interpolate #expr# in quoted attributes. (The cf-prefixed bare form
// `cflog text=...` is NOT valid CFScript on Lucee, so it is not tested here.)
logMessage = "dynamic";
logBareError = "";
try {
    log text="cfml_literal_#logMessage#" type="information";
} catch (any e) {
    logBareError = e.message;
}
assert("log statement hash interpolation in quoted attr", logBareError, "");

logCallError = "";
try {
    cflog(text="cfml_literal_#logMessage#", type="information");
} catch (any e) {
    logCallError = e.message;
}
assert("cflog() call hash interpolation in quoted attr", logCallError, "");

// ============================================================
// thread (cfthread) — run/join/terminate actions
// ============================================================

// thread run with body
thread name="testThread1" action="run" {
    // Thread body - just needs to parse and run
    var x = 42;
}
thread name="testThread1" action="join" timeout="5";
assertTrue("thread run/join parsed", true);

// thread with default action (run)
thread name="testThread2" {
    var y = 100;
}
thread name="testThread2" action="join";
assertTrue("thread default action parsed", true);

// ============================================================
// HTTP-affecting statements (header, cookie, content, location)
// Tested via cfhttp to a target page so headers don't bleed
// into the test runner's own HTTP response.
//
// These tests require RustCFML to be running as a web server so the
// target page is reachable. We discover the live port from cgi.server_port
// (set by the server at request-time) and skip the HTTP subtests when the
// runner is invoked from the CLI with no server available.
// ============================================================

serverPort = structKeyExists(cgi, "server_port") ? trim(cgi.server_port) : "";
if (serverPort == "" || serverPort == "0") {
    writeOutput(chr(10) & "  skipped HTTP subtests (no cgi.server_port — run via rustcfml --serve)" & chr(10));
} else {
    baseUrl = "http://127.0.0.1:" & serverPort;
    targetPath = "/tests/tags/http_statements_target.cfm";

    // --- header ---
    http url=baseUrl & targetPath & "?test=header" method="GET" result="headerResult";
    assert("header target responds", headerResult.statuscode, "200 OK");
    assert("header body", trim(headerResult.filecontent), "header-ok");
    assertTrue("header X-Test-Header set",
        structKeyExists(headerResult.responseheader, "X-Test-Header")
        && headerResult.responseheader["X-Test-Header"] == "hello123");

    // --- header via parenthesized call form with direct named args (issue #141) ---
    http url=baseUrl & targetPath & "?test=header-named" method="GET" result="headerNamedResult";
    assert("header-named target responds", headerNamedResult.statuscode, "200 OK");
    assert("header-named body", trim(headerNamedResult.filecontent), "header-named-ok");
    assertTrue("cfheader(name=,value=) named-arg form sets header",
        structKeyExists(headerNamedResult.responseheader, "X-Script-Named")
        && headerNamedResult.responseheader["X-Script-Named"] == "snamed");
    assertTrue("cfheader(attributeCollection=) form sets header",
        structKeyExists(headerNamedResult.responseheader, "X-Script-AC")
        && headerNamedResult.responseheader["X-Script-AC"] == "sac");

    // --- cookie ---
    http url=baseUrl & targetPath & "?test=cookie" method="GET" result="cookieResult";
    assert("cookie target responds", cookieResult.statuscode, "200 OK");
    assert("cookie body", trim(cookieResult.filecontent), "cookie-ok");

    // --- content type ---
    http url=baseUrl & targetPath & "?test=content" method="GET" result="contentResult";
    assert("content target responds", contentResult.statuscode, "200 OK");
    assert("content body", trim(contentResult.filecontent), '{"status":"ok"}');
    assertTrue("content type is json",
        findNoCase("application/json", contentResult.responseheader["Content-Type"]) > 0);

    // --- content type via cfheader (issue #148: must REPLACE, not append) ---
    http url=baseUrl & targetPath & "?test=content-header" method="GET" result="ctHeaderResult";
    assert("content-header target responds", ctHeaderResult.statuscode, "200 OK");
    assert("content-header body", trim(ctHeaderResult.filecontent), '{"ok":1}');
    ctVal = ctHeaderResult.responseheader["Content-Type"];
    // A duplicate would surface as an array or a comma-joined string containing
    // the engine default; a correct singleton is just the cfheader value.
    assertTrue("cfheader Content-Type is a single (not duplicated) value",
        isSimpleValue(ctVal));
    assertTrue("cfheader Content-Type is the json type, not the html default",
        findNoCase("application/json", ctVal) > 0 && findNoCase("text/html", ctVal) == 0);

    // --- location (redirect) ---
    http url=baseUrl & targetPath & "?test=location" method="GET" redirect="false" result="locResult";
    assertTrue("location returns 3xx",
        left(locResult.statuscode, 1) == "3");
    assertTrue("location header set",
        structKeyExists(locResult.responseheader, "Location")
        && findNoCase("redirect-target", locResult.responseheader["Location"]) > 0);
}

suiteEnd();
</cfscript>
