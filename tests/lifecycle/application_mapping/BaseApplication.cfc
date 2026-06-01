component {

    function onApplicationStart() {
        application.expanded = expandPath("/lib");
        application.widget = createObject("component", "/lib/widget").init();
        return true;
    }

    function onRequest(targetPage) {
        writeOutput(application.expanded & "|" & application.widget.ready);
    }

}
