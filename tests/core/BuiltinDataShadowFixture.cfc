component {

	// Mirrors the failing Wheels shape (vendor/wheels/Global.cfc,
	// $convertToString): normalize the argument into local.val, then call the
	// Val() BUILTIN with that local in the frame. On a conforming engine the
	// data variable never shadows the builtin in call position.
	public struct function convertProbe(required any value) {
		var s = {ok = false, got = ""};
		local.val = arguments.value;
		try { s.got = Val(local.val); s.ok = true; } catch (any e) { s.err = e.message; }
		return s;
	}

}
