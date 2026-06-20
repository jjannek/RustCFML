component {
	// A method whose name is a reserved scope word (`local`). Previously the
	// codegen stored/loaded it through `local`, loading the local SCOPE instead
	// of the function, so it was unreachable. (Preside Config.cfc environment
	// methods: `function local(){}`.)
	public string function local() {
		return "local-ran";
	}
	public string function normal() {
		return "normal-ran";
	}
}
