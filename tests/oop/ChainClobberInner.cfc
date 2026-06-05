/**
 * Fixture for test_chained_writeback_clobber.cfm — the inner CFC returned by
 * the outer's getDep(). A mutating setter (returns this) drives the chained
 * call `outer.getDep().setMark(x)`.
 */
component accessors="true" {

	property name="mark" default="";

	function init(){
		return this;
	}

	function whoAmI(){
		return "Inner";
	}

}
