<cfif thisTag.executionMode EQ "start"><cftry><cfset result = application.svc.ping() /><cfoutput>#result#</cfoutput><cfcatch><cfoutput>FAILED:#cfcatch.message#</cfoutput></cfcatch></cftry></cfif>
