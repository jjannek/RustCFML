/**
 * Probe for getFunctionCalledName(): one private UDF is exposed under several
 * public aliases (the pattern WireBox uses for delegated methods). Each alias,
 * when called, must report the name it was invoked under — not the underlying
 * function's declared name.
 */
component {

	function init(){
		// bind the same underlying function under three different public names
		this.alpha = variables.byName;
		this.beta  = variables.byName;
		this.gamma = variables.byName;
		return this;
	}

	function byName(){
		return getFunctionCalledName();
	}

	function named(){
		return getFunctionCalledName();
	}

}
