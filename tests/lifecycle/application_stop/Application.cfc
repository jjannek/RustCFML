component {
    this.name = "application-stop-test";

    function onApplicationStart() {
        application.startedByLifecycle = true;
        application.seed = createUUID();
    }

    function onApplicationEnd(appScope) {
        // applicationStop() must fire this synchronously with the still-live
        // application scope. Record the seed so the test can prove it ran once.
        if (structKeyExists(url, "endlog")) {
            var seed = structKeyExists(arguments.appScope, "seed") ? arguments.appScope.seed : "n/a";
            fileAppend(url.endlog, "END:" & seed & chr(10));
        }
    }

    function onRequest(targetPage) {
        include "#targetPage#";
    }
}
