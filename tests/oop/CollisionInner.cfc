component {
    // Method whose name collides with CollisionHost.getThing, and which
    // requires an argument — so a misresolved bare call surfaces as
    // "parameter [key] is required".
    function getThing( required key ) {
        return "Inner";
    }
}
