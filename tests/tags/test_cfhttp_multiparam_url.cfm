<cfscript>
suiteBegin("cfhttp: literal query string in url");

serverPort = structKeyExists(cgi, "server_port") ? trim(cgi.server_port) : "";
skip = serverPort == "" || serverPort == "0";

if (skip) {
    assertTrue("cfhttp literal query string skipped (no cgi.server_port)", true);
} else {
    target = "http://127.0.0.1:" & serverPort & "/tests/tags/cfhttp_multiparam_target.cfm";

    cfhttp(url="#target#", result="r1", timeout=15) {
        cfhttpparam(type="url", name="a", value="1");
        cfhttpparam(type="url", name="b", value="2");
    }
    assertTrue("control: cfhttpparam type=url params arrive", find("a=[1];b=[2]", r1.filecontent) GT 0);

    cfhttp(url="#target#?a=1", result="r2", timeout=15);
    assertTrue("single-param literal url returns 200", find("200", r2.statuscode ?: "") GT 0);
    assertTrue("single literal param arrives", find("a=[1]", r2.filecontent) GT 0);

    cfhttp(url="#target#?a=1&b=2", result="r3", timeout=15);
    assertTrue("multi-param literal url returns 200", find("200", r3.statuscode ?: "") GT 0);
    assertTrue("both literal params arrive", find("a=[1];b=[2]", r3.filecontent) GT 0);
}

suiteEnd();
</cfscript>
