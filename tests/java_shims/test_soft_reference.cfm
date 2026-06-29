<cfscript>
// Mirrors Preside/ColdBox CacheBox's ConcurrentSoftReferenceStore usage of the
// JVM GC primitives java.lang.ref.{ReferenceQueue,SoftReference}. RustCFML has
// no JVM, so these are shimmed as STRONG, never-cleared references: get() always
// returns the held referent and the queue stays permanently empty. See issue #218.
suiteBegin( "Java Shims: SoftReference + ReferenceQueue (##218)" );

// ReferenceQueue: constructs and polls empty (nothing is ever GC-enqueued).
rq = createObject( "java", "java.lang.ref.ReferenceQueue" ).init();
assertTrue( "ReferenceQueue isInstanceOf",
    isInstanceOf( rq, "java.lang.ref.ReferenceQueue" ) );
assertTrue( "ReferenceQueue.poll() empty -> null", isNull( rq.poll() ) );

// SoftReference: holds its referent strongly; get() returns it.
payload = { value = "v123" };
sr = createObject( "java", "java.lang.ref.SoftReference" ).init( payload, rq );
assertTrue( "SoftReference isInstanceOf",
    isInstanceOf( sr, "java.lang.ref.SoftReference" ) );
got = sr.get();
assertTrue( "SoftReference.get() returns referent", got.value == "v123" );

// hashCode() keys CacheBox's reverse soft-ref map ("hc-#softRef.hashCode()#"):
// must be stable per-reference and distinct across references.
h1 = sr.hashCode();
assertTrue( "SoftReference.hashCode() stable", h1 == sr.hashCode() );
sr2 = createObject( "java", "java.lang.ref.SoftReference" ).init( payload, rq );
assertTrue( "SoftReference.hashCode() distinct per instance", h1 != sr2.hashCode() );

// clear() drops the referent (Java semantics); get() then returns null.
sr.clear();
assertTrue( "SoftReference.get() null after clear", isNull( sr.get() ) );

suiteEnd();

// End-to-end: the real CacheBox store builds + runs on these shims. Mirrors
// ConcurrentSoftReferenceStore's set(timeout>0) -> SoftReference, get() ->
// isInstanceOf + .get(), lookup hit/miss, clear() -> softRefKeyMap.remove(hash).
suiteBegin( "Java Shims: CacheBox soft-store round-trip (##218)" );

store = new SoftRefStoreFixture();
store.set( "k1", { value = "cached" }, 10 );   // timeout>0 => stored as SoftReference
assertTrue( "soft store lookup hit", store.lookup( "k1" ) );
assertFalse( "soft store lookup miss", store.lookup( "absent" ) );
assertTrue( "soft store get derefs SoftReference", store.get( "k1" ).value == "cached" );
store.set( "k2", "plain", 0 );                 // timeout=0 => stored eternal (not soft)
assertTrue( "soft store eternal get", store.get( "k2" ) == "plain" );
store.clear( "k1" );
assertFalse( "soft store lookup after clear", store.lookup( "k1" ) );

suiteEnd();
</cfscript>
