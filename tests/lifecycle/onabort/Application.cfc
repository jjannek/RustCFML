component {
    this.name = "rustcfml_onabort_test";

    public void function onAbort(string targetPage = "") {
        writeOutput("ONABORT_FIRED");
    }

    public void function onRequestEnd(string targetPage = "") {
        writeOutput("ONREQUESTEND_FIRED");
    }

    public boolean function onError(required any exception, string eventName = "") {
        writeOutput("ONERROR_FIRED:" & arguments.exception.message);
        return true;
    }
}
