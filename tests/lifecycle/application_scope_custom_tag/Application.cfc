component {
    this.name = "rustcfml_appscope_customtag";
    function onApplicationStart() {
        application.svc = createObject("component", "Svc").init();
        return true;
    }
}
