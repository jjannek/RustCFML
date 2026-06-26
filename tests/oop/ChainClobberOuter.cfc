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

	// Returns a fresh array — used to prove that `outer.getItems().sort()`
	// (an in-place array member fn chained on a method that returns an array)
	// does not write the sorted array back onto `outer`.
	function getItems(){
		return [ "b", "a", "c" ];
	}

}
