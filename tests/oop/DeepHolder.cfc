/**
 * Fixture for the deep-path case in test_chained_writeback_clobber.cfm. Holds a
 * DeepInner under a 2-segment path (variables.inner) and runs a deep chained
 * mutating call against it.
 */
component accessors="true" {

	property name="inner";

	function init(){
		return this;
	}

	function probe(){
		// Deep chained mutating call: `put` runs on the foreign DeepStore and
		// returns it; codegen propagates write_back=["variables","inner"], so the
		// deep result-writeback would clobber variables.inner with the DeepStore.
		variables.inner.getStore().put( "k", "v" );
		return variables.inner.whoAmI();
	}

}
