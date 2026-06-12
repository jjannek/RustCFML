component {

	// Fixture for core/test_bare_call_shadowing_semantics.cfm — the OWN-frame
	// companion to BareCallLeakProbe (PR #97). Lucee-probed semantics:
	// own-frame data shadows same-named methods (bare call throws), but
	// builtin names are never shadowed by data.

	public string function fnA() {
		return "FN_A_RESULT";
	}

	// Own struct param shadows the method: bare fnA() must throw.
	public string function ownParamShadows(struct fnA = {}) {
		try { fnA(); return "RESOLVED"; } catch (any e) { return "THREW"; }
	}

	// Own var shadows the method: bare fnA() must throw.
	public string function ownVarShadows() {
		var fnA = {iAmData = true};
		try { fnA(); return "RESOLVED"; } catch (any e) { return "THREW"; }
	}

	// Own struct param named after a builtin: the builtin still wins.
	public string function ownBuiltinParam(struct lcase = {}) {
		try { return lcase("ABC"); } catch (any e) { return "THREW: " & e.message; }
	}

	// Caller's struct param named after a builtin must not shadow the
	// callee's bare builtin call either.
	public string function viaInheritedBuiltinShadow(struct ucase = {}) {
		return calleeUsesUcase();
	}

	public string function calleeUsesUcase() {
		try { return ucase("abc"); } catch (any e) { return "THREW: " & e.message; }
	}
}
