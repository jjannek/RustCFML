component {
    this.name = "application-function-cache-test";

    function onApplicationStart() {
        application.persistentFactory = createObject("component", "Factory");
    }

    function onRequestStart(targetPage) {
        application.requestFactory = createObject("component", "RequestFactory");
    }

    function onRequest(targetPage) {
        include "#targetPage#";
    }
}
