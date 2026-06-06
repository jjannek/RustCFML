<cfscript>
suiteBegin("Tags: cfhttp body with control flow (issue ##55)");

// Under the pre-fix static body scan, <cfhttpparam> tags wrapped in a
// <cfloop> were silently dropped and the loop body never ran. v0.52.x
// switches cfhttp to runtime body assembly when control-flow tags
// appear in the body (same fix shape as cfquery 28af97d).

request._cfhttp_loop_seen = "";
</cfscript>

<cfhttp url="http://127.0.0.1:1/" method="get" result="r" timeout="1">
	<cfloop array="#['X-One','X-Two']#" index="h">
		<cfset request._cfhttp_loop_seen = h>
		<cfhttpparam type="header" name="#h#" value="v">
	</cfloop>
</cfhttp>

<cfscript>
assert("cfloop inside cfhttp body executed at runtime", request._cfhttp_loop_seen, "X-Two");

// ------------------------------------------------------------
// Script block form: cfhttp(...) { for (...) { cfhttpparam(...); } }
// The trailing { ... } must parse as a statement block (not a struct
// literal) so cfhttpparam declared inside control flow is collected at
// runtime — the script-syntax counterpart of the tag form above.
// ------------------------------------------------------------
request._cfhttp_script_loop_seen = "";
scriptHeaders = ["X-One", "X-Two"];
cfhttp(url = "http://127.0.0.1:1/", method = "get", result = "r2", timeout = "1") {
    for (sh in scriptHeaders) {
        request._cfhttp_script_loop_seen = sh;
        cfhttpparam(type = "header", name = sh, value = "v");
    }
    // param inside a conditional collects too
    if (arrayLen(scriptHeaders) GT 1) {
        cfhttpparam(type = "header", name = "X-Cond", value = "c");
    }
}
assert("script cfhttp body loop executed at runtime",
    request._cfhttp_script_loop_seen, "X-Two");
assertTrue("script cfhttp produced a result struct",
    isStruct(r2) AND structKeyExists(r2, "statusCode"));

suiteEnd();
</cfscript>
