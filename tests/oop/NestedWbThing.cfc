component {
	function init(required string klass) {
		variables.info = {};
		variables.info.klass = arguments.klass;
		return this;
	}
	function getKlass() {
		return variables.info.klass;
	}
	// A mutating method that writes the `variables` scope (like Wheels addError).
	function touch() {
		variables.info.touched = true;
	}
}
