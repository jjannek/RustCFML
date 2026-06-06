component {
    this.name = "application-function-cache-test";

    function onApplicationStart() {
        // A closure stored in application scope that CAPTURES a local variable
        // from onApplicationStart. The captured value must survive to later
        // (warm) requests — the redesign carries the function Arc and preserves
        // its captured_scope, so `base` resolves on every request.
        var base = 100;
        application.adder = function(n) { return n + base; };
    }

    function onRequest(targetPage) {
        include "#targetPage#";
    }
}
