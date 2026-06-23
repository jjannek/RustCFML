component {
	// Reproduces the TestBox beforeEach/it shape that exposed the bug:
	//   beforeEach(function(){ auth = new X(); });     // unscoped RESET
	//   it("...", function(){ auth.register(...); ...}) // reads/mutates it
	// TestBox stores the bodies and invokes them through the ARGUMENTS scope
	// (`arguments.body()` / `arguments.spec.body()`). The bug: the unscoped reset
	// lands in the component (__variables) scope, but a method call on the var
	// inside the spec (`auth.register()`), plus the closure write-back, routed
	// through scope_aware_store — which forked a TOP-LEVEL `locals` shadow instead
	// of updating __variables. A bare read checks top-level locals before
	// __variables, so that shadow — pinned on the first dispatch and persisted in
	// the looping frame's locals — masked every later reset, and the object
	// accumulated across iterations.

	// `runTrial` mirrors TestBox: a `before` closure RESETS an unscoped var each
	// iteration; the spec (passed in, invoked via the arguments scope, in the SAME
	// looping frame) reads/mutates it. The loop + arguments-scope invocation in one
	// frame is what persisted the stale shadow.
	private function runTrial(spec, numeric iterations = 4) {
		variables.before = function() { obj = new ClosureResetRegistry(); };
		var out = "";
		for (var i = 1; i <= arguments.iterations; i++) {
			variables.before();           // reset obj to a fresh instance
			out = listAppend(out, arguments.spec());
		}
		return out;
	}

	// THE bug: each iteration the spec MUTATES then reads a freshly-reset object,
	// so the count must be 1 every time. The bug accumulated → "1,2,3,4".
	function mutateEachIteration() {
		return runTrial(function() { obj.add("x"); return obj.count(); });
	}

	// Read-only spec on a freshly-reset object: always sees an empty instance.
	function readOnlyEachIteration() {
		return runTrial(function() { return obj.count(); });
	}

	// Distinct specs run in sequence against the per-iteration reset object, the
	// AuthenticatorSpec "register / register-more / replace / remove" shape.
	// Each spec adds a known number of items to its OWN fresh object.
	function distinctSpecsSequence() {
		var s1 = function() { return obj.count(); };                              // 0
		var s2 = function() { obj.add("a"); return obj.count(); };                // 1
		var s3 = function() { obj.add("b"); obj.add("c"); return obj.count(); };  // 2
		var s4 = function() { obj.add("d"); return obj.count(); };                // 1
		var out = "";
		out = listAppend(out, runOne(s1));
		out = listAppend(out, runOne(s2));
		out = listAppend(out, runOne(s3));
		out = listAppend(out, runOne(s4));
		return out;
	}
	private function runOne(spec) {
		variables.hooks = { before = function() { obj = new ClosureResetRegistry(); } };
		variables.hooks.before();
		return arguments.spec();
	}

	// Control: an EXPLICIT variables-scoped reset/read always worked (explicit
	// scope bypasses the unscoped-routing path). Must still pass.
	function explicitScopeControl() {
		variables.vbefore = function() { variables.vobj = new ClosureResetRegistry(); };
		var out = "";
		for (var i = 1; i <= 4; i++) {
			variables.vbefore();
			var spec = function() { variables.vobj.add("x"); return variables.vobj.count(); };
			out = listAppend(out, spec());
		}
		return out;
	}

	// Two sibling closures defined together must SHARE the captured (unscoped) var:
	// a setter writes it, a getter (stored in a nested struct) reads it.
	function siblingShare() {
		setter = function() { x = "SET"; };
		getter = function() { return isNull(x) ? "NULL" : x; };
		variables.st = { s = setter, g = getter };
		variables.st.s();
		return variables.st.g();
	}
}
