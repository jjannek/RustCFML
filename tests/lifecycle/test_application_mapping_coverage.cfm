<cfscript>
suiteBegin("Lifecycle: application mappings");

serverPort = structKeyExists(cgi, "server_port") ? trim(cgi.server_port) : "";

if (serverPort == "" || serverPort == "0") {
    assertTrue("application mapping lifecycle skipped (no cgi.server_port)", true);
} else {
    targetPath = "/tests/lifecycle/application_mapping/index.cfm";
    http url="http://127.0.0.1:#serverPort##targetPath#" method="GET" result="mappingResult";
    assert("application mapping status", mappingResult.statuscode, "200 OK");
    assertTrue("inherited lifecycle sees child mapping",
        findNoCase("/tests/lifecycle/application_mapping/lib|ok", replace(trim(mappingResult.filecontent), "\", "/", "all")) > 0);
}

suiteEnd();
</cfscript>
