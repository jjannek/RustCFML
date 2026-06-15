<cfscript>
// Target page for CFScript tag statement tests.
// Called via cfhttp from test_tags_cfscript_statements.cfm.
// Sets HTTP headers, cookies, and handles location redirect.

param name="url.test" default="";

switch (url.test) {
    case "header":
        header name="X-Test-Header" value="hello123";
        header statuscode="200" statustext="OK";
        writeOutput("header-ok");
        break;

    case "header-named":
        // Parenthesized call form with direct named args (issue #141).
        // Previously these were silent no-ops; only attributeCollection worked.
        cfheader(name="X-Script-Named", value="snamed");
        cfheader(attributeCollection={name:"X-Script-AC", value:"sac"});
        writeOutput("header-named-ok");
        break;

    case "cookie":
        cookie name="testcookie" value="cookievalue";
        cookie name="securecookie" value="secret" httponly="true" secure="true";
        writeOutput("cookie-ok");
        break;

    case "location":
        location url="/redirect-target" statuscode="301";
        break;

    case "content":
        content type="application/json";
        writeOutput('{"status":"ok"}');
        break;

    case "content-header":
        // Issue #148: cfheader(name="Content-Type") must REPLACE the engine
        // default, not append a second Content-Type header.
        cfheader(name="Content-Type", value="application/json; charset=utf-8");
        writeOutput('{"ok":1}');
        break;

    case "url-echo":
        writeOutput(url.probe ?: "");
        break;

    case "echo":
        header name="X-Echo-Method" value=cgi.request_method ?: "";
        writeOutput("echo-ok");
        break;

    default:
        writeOutput("unknown-test");
}
</cfscript>
