// Helper for test_hof_member_writeback.cfm — a tiny component used as a struct
// value so the higher-order callback invokes a method on it.
component {
	function init( flag ){
		variables.flag = arguments.flag;
		return this;
	}
	function isFlag(){
		return ( isBoolean( variables.flag ) ? variables.flag : false );
	}
}
