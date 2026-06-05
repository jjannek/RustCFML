/**
 * Fixture for the deep-path case in test_chained_writeback_clobber.cfm. A
 * mutating method (`put`) that returns `this` (a CFC) — the shape that drove
 * the WireBox app-scope clobber (injector.getScopeStorage().put(...)).
 */
component accessors="true" {

	function init(){
		variables.data = {};
		return this;
	}

	function put( required string k, required any v ){
		variables.data[ arguments.k ] = arguments.v;
		return this;
	}

	function whoAmI(){
		return "Store";
	}

}
