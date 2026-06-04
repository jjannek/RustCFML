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

suiteEnd();
</cfscript>
