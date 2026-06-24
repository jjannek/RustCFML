component {
    function init() {
        // Hand OUR variables scope to a foreign object (the Wheels pattern:
        // new wheels.Plugins().$initializeMixins(variables)).
        new core.VarThisInjector().inject(variables);
        return this;
    }
    function probeKeyExists() {
        return StructKeyExists(variables, "this");
    }
}
