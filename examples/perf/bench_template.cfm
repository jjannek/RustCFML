<cfsilent>
<!--- Typical web template: cfloop + cfif + cfoutput into a captured buffer.
      Exercises the full tag pipeline + output buffering (saved_output_buffers)
      rather than a single op. Output is captured with cfsavecontent so timing
      measures rendering, not stdout flushing. Kept in tag syntax deliberately:
      this is the only bench that stresses the tag preprocessor's emitted code. --->
</cfsilent>
<cfset iterations = 200000>

<cfset warm = 1000>
<cfsavecontent variable="warmup"><cfloop from="1" to="#warm#" index="i"><cfif i mod 2 eq 0><cfoutput><div class="even">#i#</div></cfoutput><cfelse><cfoutput><div class="odd">#i#</div></cfoutput></cfif></cfloop></cfsavecontent>

<cfset start = getTickCount()>
<cfsavecontent variable="html"><cfloop from="1" to="#iterations#" index="i"><cfif i mod 2 eq 0><cfoutput><div class="even">#i#</div></cfoutput><cfelse><cfoutput><div class="odd">#i#</div></cfoutput></cfif></cfloop></cfsavecontent>
<cfset elapsed = getTickCount() - start>

<cfoutput>RESULT #elapsed#
CHECK #len( html )#
</cfoutput>
