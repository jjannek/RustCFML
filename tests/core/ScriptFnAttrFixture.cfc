// Gap F fixture: a CFC method carrying a trailing attribute after its ()
// with an unquoted value (`output=false`). RustCFML used to parse the body `{`
// as a struct literal ("Expected RBrace, found ..."), degrading the CFC to a
// non-object.
component {
	public numeric function calc() output=false {
		return 42;
	}
	public string function ping() {
		return "pong";
	}
}
