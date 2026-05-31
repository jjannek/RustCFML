<cfscript>
suiteBegin("CFLoop array item and index");

values = ["alpha", "beta"];
arrayLoopResult = "";
</cfscript>
<cfloop array="#values#" item="value" index="i">
    <cfset arrayLoopResult = arrayLoopResult & i & ":" & value & ";" />
</cfloop>
<cfscript>
assert("array cfloop item and one-based index", arrayLoopResult, "1:alpha;2:beta;");

suiteEnd();
</cfscript>
