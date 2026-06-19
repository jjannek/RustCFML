// Fixture for GitHub #180: an UNSCOPED compound (dotted) write `x.y = v`
// inside a method, where `x` already exists in the `variables` scope (but not
// in `local`/`arguments`), must resolve `x` to `variables.x` and mutate it —
// NOT fork a phantom `local.x` that is discarded on return. This is the
// classic ColdBox `instance` struct pattern, used pervasively in ColdBox/Preside.
component {

	variables.instance = { existing = "orig" };

	// `instance.newkey = v` — `instance` lives in variables scope only.
	public struct function writeUnscoped() {
		instance.newkey = "written";
		return {
			localHasInstance: structKeyExists( local, "instance" ),
			varsHasNewKey: structKeyExists( variables.instance, "newkey" )
		};
	}

	public struct function getInstance() {
		return variables.instance;
	}

	// A local var of the same name must SHADOW the variables-scope container:
	// the write lands in the local and `variables.instance` is untouched.
	public struct function writeShadowedByLocal() {
		var instance = { local = true };
		instance.shadowkey = "to-local";
		return {
			localHasShadowKey: structKeyExists( instance, "shadowkey" ),
			varsUntouched: !structKeyExists( variables.instance, "shadowkey" )
		};
	}
}
