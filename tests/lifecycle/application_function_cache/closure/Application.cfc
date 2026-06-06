component {
    this.name = "application-function-cache-test";

    function onApplicationStart() {
        application.svc = createObject("component", "Service");
    }

    function onRequest(targetPage) {
        include "#targetPage#";
    }
}
