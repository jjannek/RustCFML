/**
 * Mirrors Preside/ColdBox CacheBox's ConcurrentStore: a component that both
 * declares a `get( required objectKey )` method AND, in a sibling method, calls
 * `pool.get( arguments.objectKey )` on a java.util.concurrent.ConcurrentHashMap.
 *
 * A cache MISS makes the shim's `get` return null; the dispatch must treat that
 * as the map getter's real result, NOT as "method unhandled" (which used to fall
 * through and re-resolve the bare `get` against this component's own method
 * table, calling `get()` with zero args → "parameter [objectKey] is required").
 */
component {

	public any function init() {
		variables.pool = createObject( "java", "java.util.concurrent.ConcurrentHashMap" ).init();
		return this;
	}

	public any function get( required any objectKey ) {
		return getQuiet( arguments.objectKey );
	}

	public any function getQuiet( required any objectKey ) {
		return pool.get( arguments.objectKey );
	}

	public void function set( required any objectKey, required any object ) {
		pool.put( arguments.objectKey, arguments.object );
	}
}
