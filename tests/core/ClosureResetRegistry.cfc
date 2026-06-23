component {
	// A tiny reference-typed fixture: each instance owns its own array, so two
	// instances are independent. Used to prove that a `beforeEach`-style closure
	// that REASSIGNS an unscoped variable to a fresh instance is seen by sibling
	// spec closures (rather than them sharing one accumulating instance).
	function init() {
		variables.arr = [];
		return this;
	}
	function add(name) {
		arrayAppend(variables.arr, name);
		return this;
	}
	function count() {
		return arrayLen(variables.arr);
	}
	function dump() {
		return arrayToList(variables.arr, ",");
	}
}
