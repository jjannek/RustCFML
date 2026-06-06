<cfcomponent>
	<cfscript>
	// A method that defines an anonymous closure. Its bytecode carries a
	// DefineFunction(idx) op whose index is valid only against the program the
	// CFC was merged into (the request's top-level program) — NOT against a
	// swapped-in sub-program (issue #70).
	function runClosure() {
		var f = function() { return "closure-ok"; };
		return f();
	}

	// Arrow-function variant.
	function runArrow() {
		var g = () => "arrow-ok";
		return g();
	}

	// Nested closure (closure defined inside a closure).
	function runNested() {
		var outer = function() {
			var inner = function() { return "nested-ok"; };
			return inner();
		};
		return outer();
	}

	// Closure factory: returns a closure that is CALLED LATER by the caller,
	// possibly after the swapped program has been restored.
	function makeAdder(n) {
		return function(x) { return x + n; };
	}
	</cfscript>
</cfcomponent>
