<cfscript>suiteBegin("Tags: Output buffer recovery on throw");</cfscript>

<!---
  Regression tests for the saved_output_buffers / custom_tag_stack imbalance
  that occurred when a cfsavecontent or custom-tag body threw before its
  matching end op. The throw is caught by an OUTER cftry, so execution
  continues -- but the abandoned body buffer was left active, so output
  written after the catch was mis-attributed (captured into the wrong buffer
  / lost from the page). The fix restores the capture stacks on unwind.

  Each test wraps the throwing region in an OUTER cfsavecontent so we can
  observe where post-recovery output actually lands.
--->

<!--- 1. cfsavecontent body throws, caught by outer cftry --->
<cfsavecontent variable="scOuter"><cftry><cfsavecontent variable="scInner"><cfoutput>PARTIAL</cfoutput><cfthrow message="boom"></cfsavecontent><cfcatch></cfcatch></cftry><cfoutput>AFTER</cfoutput></cfsavecontent>
<cfscript>
    // After the catch, "AFTER" must land in the outer buffer; the abandoned
    // inner body content ("PARTIAL") must not leak into it.
    assert("savecontent throw: output recovers to outer buffer", trim(scOuter), "AFTER");
    assertTrue("savecontent throw: partial body discarded", findNoCase("PARTIAL", scOuter) EQ 0);
</cfscript>

<!--- 2. Custom-tag body throws, caught by outer cftry --->
<cfsavecontent variable="ctOuter"><cftry><cf_wrapper><cfoutput>PARTIAL</cfoutput><cfthrow message="boom"></cf_wrapper><cfcatch></cfcatch></cftry><cfoutput>AFTER</cfoutput></cfsavecontent>
<cfscript>
    // The start-tag output (<div>) was emitted before the throw, so it may be
    // present; what matters is that post-recovery "AFTER" lands here and the
    // discarded body content does not.
    assertTrue("custom tag throw: output recovers to outer buffer", findNoCase("AFTER", ctOuter) GT 0);
    assertTrue("custom tag throw: partial body discarded", findNoCase("PARTIAL", ctOuter) EQ 0);
</cfscript>

<!--- 3. A normal savecontent after the recovery still captures correctly --->
<cftry><cfsavecontent variable="thrownAway"><cfoutput>LEAK</cfoutput><cfthrow message="boom"></cfsavecontent><cfcatch></cfcatch></cftry>
<cfsavecontent variable="clean"><cfoutput>CLEAN</cfoutput></cfsavecontent>
<cfscript>
    assert("savecontent after recovery captures cleanly", trim(clean), "CLEAN");
</cfscript>

<cfscript>suiteEnd();</cfscript>
