<cfscript>
suiteBegin("CFLoop negative step");

descending = "";
</cfscript>
<cfloop from="3" to="1" step="-1" index="i">
    <cfset descending = descending & i />
</cfloop>
<cfscript>
ascending = "";
</cfscript>
<cfloop from="1" to="3" index="i">
    <cfset ascending = ascending & i />
</cfloop>
<cfscript>
assert("counted cfloop supports literal negative step", descending & "|" & ascending, "321|123");

// Dynamic (runtime-valued) negative step: direction must be decided at
// runtime, not from the source text. Matches Lucee (-> "321").
dynStep = -1;
dynDescending = "";
</cfscript>
<cfloop from="3" to="1" step="#dynStep#" index="i">
    <cfset dynDescending = dynDescending & i />
</cfloop>
<cfscript>
assert("counted cfloop supports dynamic negative step", dynDescending, "321");

// Negative step larger than 1.
neg2 = "";
</cfscript>
<cfloop from="10" to="2" step="-2" index="i">
    <cfset neg2 = neg2 & i & "," />
</cfloop>
<cfscript>
assert("counted cfloop honours negative step magnitude", neg2, "10,8,6,4,2,");

// from == to with a negative step runs exactly once (Lucee -> "5").
equalNeg = "";
</cfscript>
<cfloop from="5" to="5" step="-1" index="i">
    <cfset equalNeg = equalNeg & i />
</cfloop>
<cfscript>
assert("counted cfloop with equal bounds and negative step runs once", equalNeg, "5");

// Positive step with from > to never runs (Lucee -> "").
posNoRun = "";
</cfscript>
<cfloop from="3" to="1" step="1" index="i">
    <cfset posNoRun = posNoRun & i />
</cfloop>
<cfscript>
assert("counted cfloop with positive step and from > to does not run", posNoRun, "");

suiteEnd();
</cfscript>
