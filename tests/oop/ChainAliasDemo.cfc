component {
    // Pseudo-constructor chained assignment — both names must reference ONE
    // object (GitHub #221). Pre-fix, the leftmost target got a deep COPY.
    variables.obj = this.obj = new ChainAliasInner();

    // Non-chained body-level aliasing across this/variables — same root cause.
    this.alias = new ChainAliasInner();
    variables.alias = this.alias;

    // Distinct objects assigned to this.x and variables.y must STAY distinct.
    this.distinctA = new ChainAliasInner();
    variables.distinctB = new ChainAliasInner();

    function mutateChained(){
        this.obj[ "added" ] = function(){ return "hi"; };
        return structKeyExists( this.obj, "added" ) & "/" & structKeyExists( variables.obj, "added" );
    }
    function callViaVariables(){
        return variables.obj.added();
    }
    function mutateAlias(){
        variables.alias[ "viaVars" ] = function(){ return "yo"; };
        return structKeyExists( this.alias, "viaVars" ) & "/" & structKeyExists( variables.alias, "viaVars" );
    }
    function mutateDistinct(){
        this.distinctA[ "m" ] = function(){ return 1; };
        return structKeyExists( this.distinctA, "m" ) & "/" & structKeyExists( variables.distinctB, "m" );
    }
}
