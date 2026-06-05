/**
 * Fixture for test_dynamic_lhs_assign.cfm. `put` uses a dynamic quoted-string
 * LHS to write into the component's private (variables) scope — the construct
 * WireBox's MixerUtil.injectPropertyMixin relies on.
 */
component {

	function init(){
		return this;
	}

	function put( required string name, required any value ){
		"variables.#arguments.name#" = arguments.value;
		return this;
	}

	function read( required string name ){
		return variables[ arguments.name ] ?: "UNSET";
	}

}
