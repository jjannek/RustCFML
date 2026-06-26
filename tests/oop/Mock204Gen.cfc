/**
 * Mirrors TestBox MockGenerator: injects the generated method into the target
 * via a scope-prefixed `arguments.targetObject.$include(...)` call — the
 * multi-segment writeback path that regressed in v0.281.0 (issue #204).
 */
component {
    function gen( required targetObject ){
        arguments.targetObject.doInc( "mock204_generated.cfm" );
        return this;
    }
}
