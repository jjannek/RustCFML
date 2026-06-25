component {

	function saver() {
		// unscoped write — must land in the RECEIVER's variables scope
		hasObjectChanged = "yes";
	}

	function getter() {
		// unscoped read — must see the write made by saver() above
		return hasObjectChanged;
	}

	// Graft this component's own methods onto a different component, run one via
	// `cfinvoke component=obj`, then read back via a direct dot-call. The write
	// (cfinvoke) and read (dot-call) must resolve the SAME variables scope.
	function runWithComponent() {
		var obj = new CfinvokeOverlayModel();
		obj.saver = this.saver;
		obj.getter = this.getter;
		cfinvoke(component = obj, method = "saver");
		return obj.getter();
	}

	// Same, but via the invoke() BIF instead of the cfinvoke tag.
	function runWithInvokeBif() {
		var obj = new CfinvokeOverlayModel();
		obj.saver = this.saver;
		obj.getter = this.getter;
		invoke(obj, "saver");
		return obj.getter();
	}

}
