component {
	public string function probe() {
		return capture(a = "A", b = "B", c = "C");
	}

	private string function capture(required string a) {
		return StructKeyExists(arguments, "b") && StructKeyExists(arguments, "c")
			? "a=" & arguments.a & ",b=" & arguments.b & ",c=" & arguments.c
			: "MISSING (keys=" & StructKeyList(arguments) & ")";
	}
}
