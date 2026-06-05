/**
 * Fixture for test_getmetadata_properties.cfm. Declares an injectable property
 * (with an `inject` annotation) and a typed property, so getMetadata() can be
 * asserted to surface declared property annotations.
 */
component accessors="true" {

	property name="propDep" inject="model:ServiceX";
	property name="count"   type="numeric" default="0";

	function init(){
		return this;
	}

}
