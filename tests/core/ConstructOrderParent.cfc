component {
	function pHelper(){ return StructKeyExists(variables, "pSibling") ? "sees" : "NO"; }
	function pSibling(){ return "psib"; }
	function init(){ return this; }
}
