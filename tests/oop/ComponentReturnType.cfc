component {

    // A function may declare a `component` return type. This must not affect
    // whether the CFC itself can be resolved / instantiated.
    public component function init() {
        return this;
    }

    public string function ping() {
        return "pong";
    }

}
