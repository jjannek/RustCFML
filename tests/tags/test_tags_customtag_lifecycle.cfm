<cfscript>suiteBegin("Tags: Custom Tag Lifecycle");</cfscript>

<!--- A self-closing (XML-style) custom tag runs BOTH the start and end phases.
      Lucee/ACF treat <cf_foo /> as shorthand for <cf_foo></cf_foo>. --->
<cfset ltOut = "">
<cfset ltHet = false>
<cf_ltcap value="okvalue" outVar="ltOut" hetVar="ltHet" />
<cfscript>
    // End phase ran (caller write-back happened) AND a variable set in the
    // start phase survived into the end phase.
    assert("self-closing custom tag runs end phase with start locals", ltOut, "okvalue");
    // <cf_foo /> reports hasEndTag = true on both engines.
    assertTrue("self-closing custom tag reports hasEndTag", ltHet);
</cfscript>

<!--- Body custom tag: the body's generated content sits between the start-phase
      and end-phase output. --->
<cfsavecontent variable="ltLayoutOut"><cf_ltlayout>[MID]</cf_ltlayout></cfsavecontent>
<cfscript>
    assertTrue("body tag start output precedes generated content", findNoCase("[OPEN]", ltLayoutOut) LT findNoCase("[MID]", ltLayoutOut));
    assertTrue("body tag generated content precedes end output", findNoCase("[MID]", ltLayoutOut) LT findNoCase("[CLOSE]", ltLayoutOut));
</cfscript>

<!--- The custom tag template has its own local scope. --->
<cfset ltLocalOut = "">
<cf_ltlocal outVar="ltLocalOut" />
<cfscript>assert("custom tag template has local scope", ltLocalOut, "ok");</cfscript>

<!--- Start-phase cfreturn still preserves attributes/locals for the end phase. --->
<cfsavecontent variable="ltReturnOut"><cf_ltreturn>body</cf_ltreturn></cfsavecontent>
<cfscript>
    assert("body tag start cfreturn preserves start attributes", trim(ltReturnOut), '<main class="px-5 py-5">body</main>');
</cfscript>

<cfscript>suiteEnd();</cfscript>
