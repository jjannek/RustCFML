component {
    // Host method shares the name `getThing` with CollisionInner.getThing,
    // and takes no required args. Returns a CollisionInner instance.
    function getThing() {
        if (isNull(variables.inner)) variables.inner = new CollisionInner();
        return variables.inner;
    }

    // Two sequential BARE calls. Before the fix, the first call leaked
    // CollisionInner's methods into this frame's locals, so the second bare
    // getThing() resolved to CollisionInner.getThing (required key) -> error.
    function twoBareCalls() {
        var a = getThing();
        var b = getThing();
        return getMetadata(a).name & "-" & getMetadata(b).name;
    }

    // Extract a method reference, then a bare call.
    function extractThenCall() {
        var ref = getThing().getThing;   // extracts CollisionInner.getThing
        var c = getThing();              // bare call must stay Host.getThing
        return getMetadata(c).name;
    }

    // Force a fresh inner each call (no caching).
    function newEachCall() {
        var a = makeNew();
        var b = makeNew();
        return getMetadata(b).name;
    }

    function makeNew() {
        return new CollisionInner();
    }
}
