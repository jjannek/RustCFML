component {
    // Method whose body DEFINES a closure at call time. The closure is reachable
    // only via a DefineFunction op inside greet()'s bytecode, never as a stored
    // value — so it must be carried transitively for warm-request dispatch.
    function greet() {
        var make = function(n) { return "ok-" & n; };
        return make( len("abc") );
    }
}
