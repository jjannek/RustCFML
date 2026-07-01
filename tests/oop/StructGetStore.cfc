/**
 * Regression fixture for GH #223: a component that has its OWN get() method and
 * internally calls .get() on a plain struct held in variables scope. Before the
 * fix, `variables.pool.get(key)` mis-resolved to this component's get() and
 * recursed to the depth-256 abort instead of hitting the struct's java.util.Map
 * get() passthrough. Models ColdBox's ConcurrentStore.
 */
component {
    function get( required objectKey ){
        var results = variables.pool.get( arguments.objectKey );
        if ( !isNull( results ) ) {
            return results;
        }
        return "MISS";
    }
    function setPool( p ){
        variables.pool = arguments.p;
    }
}
