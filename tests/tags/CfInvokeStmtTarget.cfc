// Target component for the statement-form cfinvoke tests. Plain methods whose
// results let the caller verify component resolution, method dispatch, and
// argument passing (positional-by-name, argumentcollection, invokeargument).
component {

	function add(a, b) {
		return a + b;
	}

	function greet(name) {
		return "hi " & name;
	}

	function answer() {
		return 42;
	}

}
