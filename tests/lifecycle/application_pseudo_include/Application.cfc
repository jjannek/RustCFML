component {
    this.name = "rustcfml-app-pseudo-include-test";
    // Relative include in the pseudo-constructor MUST resolve against this
    // Application.cfc's own directory, regardless of which (possibly deep)
    // page triggered the request.
    include "shared_config.cfm";

    function onRequest(targetPage) {
        writeOutput("included=" & (structKeyExists(variables, "sharedFlag") ? variables.sharedFlag : "MISSING"));
    }
}
