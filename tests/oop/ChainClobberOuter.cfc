/**
 * Fixture for test_chained_writeback_clobber.cfm. Holds an inner CFC and hands
 * it back via getDep(), so `outer.getDep().setMark(x)` is a chained call whose
 * outer mutating method runs on a DIFFERENT CFC than the base variable.
 */
component accessors="true" {

	function init(){
		variables.dep = new ChainClobberInner();
		return this;
	}

	function getDep(){
		return variables.dep;
	}

	function whoAmI(){
		return "Outer";
	}

}
