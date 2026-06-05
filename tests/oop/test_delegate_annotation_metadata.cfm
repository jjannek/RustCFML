<cfscript>
suiteBegin("Metadata: bare + arbitrary annotations, top-level component annotations");

// The metadata surface DI frameworks (WireBox delegation) read:
//  1. Property annotations written WITHOUT a value (`property name="x" inject;`)
//     must be captured — previously a bare annotation halted attribute parsing,
//     dropping everything after it.
//  2. Arbitrary-named property annotations (delegateSuffix, delegateExcludes)
//     must be captured, not just a fixed inject/type allow-list.
//  3. Component-level custom annotations (`component delegates="...">`) must
//     appear as TOP-LEVEL keys in getComponentMetadata() — matching getMetadata()
//     and Lucee/ACF — because that's where getAnnotationValue() looks.

// helper: find a property struct by name in a metadata properties array
function propByName( required array props, required string name ){
	for ( p in arguments.props ) {
		if ( structKeyExists( p, "name" ) && p.name == arguments.name ) {
			return p;
		}
	}
	return {};
}

instance = createObject( "component", "oop.DelegateAnnoProbe" ).init();

// Both reflection entry points must agree.
mdInst = getMetadata( instance );                                  // by instance
mdPath = getComponentMetadata( "oop.DelegateAnnoProbe" );          // by path

// ---- (3) component-level annotation surfaced top-level ----
assertTrue( "getMetadata: delegates is a top-level key",          structKeyExists( mdInst, "delegates" ) );
assert(     "getMetadata: delegates value",                       mdInst.delegates, "Memory, >Cache" );
assertTrue( "getComponentMetadata: delegates is a top-level key", structKeyExists( mdPath, "delegates" ) );
assert(     "getComponentMetadata: delegates value",              mdPath.delegates, "Memory, >Cache" );

// ---- (1)+(2) property annotations, checked on getComponentMetadata ----
memory = propByName( mdPath.properties, "memory" );
assertTrue( "memory has bare 'inject'",         structKeyExists( memory, "inject" ) );
assertTrue( "memory has bare 'delegate'",       structKeyExists( memory, "delegate" ) );
assertTrue( "memory has bare 'delegatePrefix'", structKeyExists( memory, "delegatePrefix" ) );
// bare annotations carry an empty-string value (so `getAnnotationValue(...).listToArray()` is empty)
assert( "bare 'delegate' is empty string", memory.delegate, "" );

store = propByName( mdPath.properties, "store" );
assert(     "store inject value",                store.inject, "Cache" );
assertTrue( "store has bare 'delegate'",         structKeyExists( store, "delegate" ) );
assert(     "store delegateSuffix value",        store.delegateSuffix, "store" );
assert(     "store delegateExcludes value",      store.delegateExcludes, "flush" );

// ---- same property annotations visible via getMetadata(instance) ----
memoryI = propByName( mdInst.properties, "memory" );
assertTrue( "getMetadata: memory has bare 'delegatePrefix'", structKeyExists( memoryI, "delegatePrefix" ) );
storeI  = propByName( mdInst.properties, "store" );
assert(     "getMetadata: store delegateExcludes value",     storeI.delegateExcludes, "flush" );

suiteEnd();
</cfscript>
