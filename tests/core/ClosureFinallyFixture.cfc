component {

	variables.finallyCount = 0;

	function getUtility(){
		return this;
	}

	function makeUtil( required who ){
		return "util-for-" & arguments.who;
	}

	/**
	 * A closure defined inside a `lock {}` whose name references the enclosing
	 * `arguments` scope. The closure must return its own value; the lock's
	 * generated finally (which re-evaluates the name via the enclosing
	 * arguments) must not be compiled into the closure body.
	 */
	function lockedClosure( required injector ){
		lock name="CFI.#arguments.injector#.x" type="exclusive" timeout="10" {
			var produce = function(){
				return makeUtil( injector );
			};
			return produce();
		}
	}

	/**
	 * Same shape with a bare `try {} finally {}` instead of `lock`.
	 */
	function tryFinallyClosure( required n ){
		try {
			var produce = function(){
				return n * 2;
			};
			return produce();
		} finally {
			// References the enclosing arguments scope; must not leak into the
			// closure body above.
			variables.lastN = arguments.n;
		}
	}

	/**
	 * The enclosing finally must still run exactly once (the closure stealing it
	 * would have double-counted, or run it inside the closure).
	 */
	function runWithFinallyProbe(){
		try {
			var produce = function(){
				return 1;
			};
			produce();
		} finally {
			variables.finallyCount++;
		}
	}

	function getFinallyCount(){
		return variables.finallyCount;
	}

}
