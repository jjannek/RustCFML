/**
 * Fixture for the deep-path case in test_chained_writeback_clobber.cfm. Holds a
 * DeepStore and exposes it via an accessor-generated getStore().
 */
component accessors="true" {

	property name="store";

	function init(){
		return this;
	}

	function whoAmI(){
		return "Inner";
	}

}
