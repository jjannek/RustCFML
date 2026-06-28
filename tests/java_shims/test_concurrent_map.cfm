<cfscript>
// Mirrors Preside/ColdBox Cachebox's ConcurrentStore usage pattern:
//   pool = createObject("java", "java.util.concurrent.ConcurrentHashMap").init();
//   collections = createObject("java", "java.util.Collections");
//   pool.put(key, obj); pool.get(key); pool.remove(key);
//   collections.list(pool.keys())  // Enumeration → List
suiteBegin( "Java Shims: ConcurrentHashMap + Collections" );

pool = createObject( "java", "java.util.concurrent.ConcurrentHashMap" ).init();
assertTrue( "ConcurrentHashMap init",
    isInstanceOf( pool, "java.util.concurrent.ConcurrentHashMap" ) );

pool.put( "k1", "v1" );
pool.put( "k2", "v2" );
pool.put( "k3", "v3" );

assertTrue( "ConcurrentHashMap size", pool.size() == 3 );
assertTrue( "ConcurrentHashMap containsKey hit", pool.containsKey( "k2" ) );
assertFalse( "ConcurrentHashMap containsKey miss", pool.containsKey( "nope" ) );
assertTrue( "ConcurrentHashMap get", pool.get( "k1" ) == "v1" );
assertTrue( "ConcurrentHashMap isEmpty false", pool.isEmpty() == false );

// remove(key) returns the removed value AND mutates the map
removed = pool.remove( "k2" );
assertTrue( "ConcurrentHashMap remove returns old value", removed == "v2" );
assertTrue( "ConcurrentHashMap size after remove", pool.size() == 2 );
assertFalse( "ConcurrentHashMap key gone after remove", pool.containsKey( "k2" ) );

// putIfAbsent — no-op when key present
pool.putIfAbsent( "k1", "OVERRIDE" );
assertTrue( "ConcurrentHashMap putIfAbsent keeps existing", pool.get( "k1" ) == "v1" );
pool.putIfAbsent( "k4", "v4" );
assertTrue( "ConcurrentHashMap putIfAbsent inserts missing", pool.get( "k4" ) == "v4" );

// keys() feeds into Collections.list() in real ColdBox code
collections = createObject( "java", "java.util.Collections" );
keyList = collections.list( pool.keys() );
assertTrue( "Collections.list count", arrayLen( keyList ) == 3 );

// Collections: a handful of static helpers. Note: Collections.sort/reverse
// are void (mutate in place) on real Java, so we don't test those here —
// callers that need in-place mutation get it engine-side; tests stick to
// pure-return helpers that behave identically on both engines.
empty = collections.emptyList();
assertTrue( "Collections.emptyList", arrayLen( empty ) == 0 );

// clear empties the map
pool.clear();
assertTrue( "ConcurrentHashMap clear", pool.size() == 0 );
assertTrue( "ConcurrentHashMap isEmpty true", pool.isEmpty() );

suiteEnd();

// Regression: a component-internal `pool.get( key )` on a MISSING key must
// return null, not mis-dispatch to the component's own `get( required objectKey )`
// method (the Preside CacheBox ConcurrentStore SIGSEGV-adjacent dispatch bug).
suiteBegin( "Java Shims: map getter null vs caller-method mis-dispatch" );

store = new CacheStoreFixture();

// MISS via the sibling method that does `pool.get( arguments.objectKey )`.
// Pre-fix this threw "The parameter [objectKey] to function [get] is required".
assertTrue( "getQuiet on missing key returns null", isNull( store.getQuiet( "absent" ) ) );
assertTrue( "get on missing key returns null", isNull( store.get( "absent" ) ) );

// HIT path still works through the same dispatch.
store.set( "k1", "v1" );
assertTrue( "getQuiet on present key returns value", store.getQuiet( "k1" ) == "v1" );
assertTrue( "get on present key returns value", store.get( "k1" ) == "v1" );

suiteEnd();
</cfscript>
