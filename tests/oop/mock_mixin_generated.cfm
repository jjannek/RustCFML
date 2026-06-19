<cfscript>
    // Mirrors MockGenerator.generate(): declare a temp function, then expose it
    // under the requested public/private name on the including object's scopes.
    variables[ "injected" ] = variables[ "tmpMockFn" ];
    this[ "injected" ]      = variables[ "tmpMockFn" ];
    structDelete( variables, "tmpMockFn" );
    function tmpMockFn() output=true {
        return "INJECTED-RESULT";
    }
</cfscript>
