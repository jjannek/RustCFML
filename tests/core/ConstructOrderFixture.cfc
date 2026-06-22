component {
	// All evaluated DURING the pseudo-constructor: each must see the full
	// method table (Lucee/ACF assemble it before running the body).
	variables.bareResult   = helper();        // bare sibling call -> variables sees sibling
	variables.thisResult   = this.viaThis();  // this.method() dispatch during construction
	variables.invokeResult = doInvoke();      // cfinvoke method= with NO component

	function helper(){ return StructKeyExists(variables, "sibling") ? "sees" : "NO"; }
	function sibling(){ return "sib"; }
	function viaThis(){ return StructKeyExists(variables, "sibling") ? sibling() : "NO"; }
	function doInvoke(){
		cfinvoke(method = "sibling", returnVariable = "local.r", argumentCollection = {});
		return local.r;
	}
	function bareR(){ return variables.bareResult; }
	function thisR(){ return variables.thisResult; }
	function invokeR(){ return variables.invokeResult; }
}
