component {
    // Mimics wheels.Plugins.$initializeMixins(variablesScope): a FOREIGN object
    // receives another component's `variables` scope and appends mixins to both
    // the private scope and the public `variables.this` alias.
    public any function inject(required struct variablesScope) {
        StructAppend(arguments.variablesScope, { mixedViaVars = function(){ return "VIA_VARS"; } }, true);
        if (StructKeyExists(arguments.variablesScope, "this")) {
            StructAppend(arguments.variablesScope.this, { mixedViaThis = function(){ return "VIA_THIS"; } }, true);
            arguments.variablesScope.this.injectedProp = "PROP";
        }
        return true;
    }
}
