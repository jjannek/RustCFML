// Target for the canonical invoke() forms test.
component {
	public string function greet(string who = "world") {
		return "hi " & arguments.who;
	}
}
