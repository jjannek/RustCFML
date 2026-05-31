<cfscript>
suiteBegin("Tags: cfhttpparam interpolation");

serverPort = structKeyExists(cgi, "server_port") ? trim(cgi.server_port) : "";
skip = serverPort == "" || serverPort == "0";

if (skip) {
    assertTrue("tag cfhttpparam interpolation skipped (no cgi.server_port)", true);
} else {
    requestUrl = "http://127.0.0.1:" & serverPort & "/tests/tags/http_statements_target.cfm?test=url-echo";
    paramValue = "dynamic";
    paramError = "";
}
</cfscript>

<cfif NOT skip>
    <cftry>
        <cfhttp url="#requestUrl#" method="GET" result="paramHttpResult">
            <cfhttpparam type="url" name="probe" value="value-#paramValue#">
        </cfhttp>
        <cfcatch type="any">
            <cfset paramError = cfcatch.message>
        </cfcatch>
    </cftry>
    <cfscript>
        assert("tag cfhttpparam interpolated value error", paramError, "");
        assert("tag cfhttpparam interpolated value body",
            structKeyExists(variables, "paramHttpResult") ? trim(paramHttpResult.filecontent) : "",
            "value-dynamic");
    </cfscript>
</cfif>

<cfscript>
suiteEnd();
</cfscript>
