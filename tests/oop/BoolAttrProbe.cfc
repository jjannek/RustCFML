/**
 * Fixture for test_component_bool_attr.cfm. Uses a bare boolean component
 * header attribute (`singleton`, equivalent to singleton="true") alongside a
 * valued attribute — the form that previously failed to parse (silent null
 * component) because the bare attribute was left stranded before the `{`.
 */
component accessors="true" singleton {

	function init(){
		return this;
	}

	function whoAmI(){
		return "BoolAttrProbe";
	}

}
