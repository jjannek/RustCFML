component accessors="true" {
	property name="foo";
	property name="bar"; // intentionally left unset

	// Mirrors ColdBox RequestContext.getMemento: filter the variables scope by a
	// 2-arg closure. An unset property must NOT be a null-valued key in the
	// variables scope (Lucee parity) — otherwise it is passed to the closure as
	// an undefined `value` arg and crashes.
	public struct function getMemento() {
		return variables.filter( function( key, value ) {
			return ( !isCustomFunction( value ) );
		} );
	}

	public boolean function hasBarKey() {
		return structKeyExists( variables, "bar" );
	}
}
