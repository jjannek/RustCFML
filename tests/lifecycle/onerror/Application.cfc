component {
    this.name = "rustcfml_onerror_test";

    public boolean function onError(required any exception, string eventName = "") {
        writeOutput("ONERROR_FIRED:" & arguments.exception.message & ":EVENT[" & arguments.eventName & "]");
        return true;
    }
}
