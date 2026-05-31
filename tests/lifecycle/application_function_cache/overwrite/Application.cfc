component {
    this.name = "application-function-cache-test";

    function onRequestStart(targetPage) {
        application.factory = createObject("component", "Factory");
    }

    function onRequest(targetPage) {
        include "#targetPage#";
    }
}
