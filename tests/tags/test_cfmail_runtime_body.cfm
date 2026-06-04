<cfscript>
suiteBegin("Tags: cfmail body with control flow");

// Same fix shape as issue ##55 / cfhttp: when a <cfmail> body wraps
// <cfmailparam>/<cfmailpart> tags in <cfif>/<cfloop>, build the params
// list and body text at runtime via savecontent — not via a static
// body scan. We don't actually deliver mail here; we just confirm the
// body executes by checking that a loop variable bled into caller scope.

request._cfmail_loop_seen = "";
</cfscript>

<cftry>
<cfmail to="x@example.com" from="y@example.com" subject="t" server="127.0.0.1" port="1" timeout="1">
	<cfloop array="#['A','B','C']#" index="hdr">
		<cfset request._cfmail_loop_seen = hdr>
		<cfmailparam name="#hdr#" value="v">
	</cfloop>
	body text
</cfmail>
<cfcatch type="any"></cfcatch>
</cftry>

<cfscript>
assert("cfloop inside cfmail body executed at runtime", request._cfmail_loop_seen, "C");

suiteEnd();
</cfscript>
