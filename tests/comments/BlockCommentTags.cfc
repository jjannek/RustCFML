component {
    /* Documentation, NOT markup — these literal tags must stay inert:
       <cfset commentSideEffect = 1>
       <cfoutput>#commentSideEffect#</cfoutput>
    */
    function ping() {
        return "pong";
    }
}
