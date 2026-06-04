<cfscript>
suiteBegin("Tags: cfstoredproc body with control flow");

// Same fix shape as issue ##55 / cfhttp: <cfprocparam> tags wrapped in
// <cfif>/<cfloop> were silently dropped under the static body scan. The
// runtime path now executes the body to build __cfproc_params and feeds
// queryExecute. We don't actually call a procedure here — the
// connection fails — we just confirm the body runs by checking a loop
// variable bled into caller scope.

request._cfproc_loop_seen = "";
</cfscript>

<cftry>
<cfstoredproc procedure="someProc">
	<cfloop array="#[10, 20, 30]#" index="val">
		<cfset request._cfproc_loop_seen = val>
		<cfprocparam value="#val#" cfsqltype="cf_sql_integer">
	</cfloop>
	<cfprocresult name="qOut">
</cfstoredproc>
<cfcatch type="any"></cfcatch>
</cftry>

<cfscript>
assert("cfloop inside cfstoredproc body executed at runtime", request._cfproc_loop_seen, 30);

suiteEnd();
</cfscript>
