/**
 * A self-contained distillation of ColdBox/CacheBox's
 * ConcurrentSoftReferenceStore: a pool keyed by objectKey, where entries set
 * with a timeout > 0 are wrapped in a java.lang.ref.SoftReference (deref'd on
 * read via isInstanceOf), and a reverse soft-ref-key map keyed on the
 * SoftReference's hashCode(). Exercises every java.lang.ref.* call site the real
 * store uses, without needing the full ColdBox cache provider. See issue #218.
 */
component {

	public any function init() {
		variables.pool          = createObject( "java", "java.util.concurrent.ConcurrentHashMap" ).init();
		variables.softRefKeyMap = createObject( "java", "java.util.concurrent.ConcurrentHashMap" ).init();
		variables.referenceQueue = createObject( "java", "java.lang.ref.ReferenceQueue" ).init();
		return this;
	}

	public boolean function lookup( required any objectKey ) {
		if ( !variables.pool.containsKey( arguments.objectKey ) ) {
			return false;
		}
		return !isNull( get( arguments.objectKey ) );
	}

	public any function get( required any objectKey ) {
		var target = variables.pool.get( arguments.objectKey );
		if ( !isNull( target ) ) {
			if ( isInstanceOf( target, "java.lang.ref.SoftReference" ) ) {
				return target.get();
			}
			return target;
		}
	}

	public void function set( required any objectKey, required any object, required numeric timeout ) {
		var target = arguments.object;
		if ( arguments.timeout > 0 ) {
			target = createObject( "java", "java.lang.ref.SoftReference" ).init( arguments.object, variables.referenceQueue );
			variables.softRefKeyMap.put( "hc-#target.hashCode()#", arguments.objectKey );
		}
		variables.pool.put( arguments.objectKey, target );
	}

	public boolean function clear( required any objectKey ) {
		if ( !variables.pool.containsKey( arguments.objectKey ) ) {
			return false;
		}
		var softRef = variables.pool.get( arguments.objectKey );
		if ( !isNull( softRef ) && isInstanceOf( softRef, "java.lang.ref.SoftReference" ) ) {
			variables.softRefKeyMap.remove( "hc-#softRef.hashCode()#" );
		}
		variables.pool.remove( arguments.objectKey );
		return true;
	}
}
