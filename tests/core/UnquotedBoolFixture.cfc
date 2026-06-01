// Gap B fixture: an UNQUOTED boolean keyword as a component attribute value
// (`output=false`). On Lucee/Adobe CF/BoxLang an unquoted boolean (or bare
// identifier) is a legal attribute value, so this parses and instantiates.
component output=false {
	public string function ping() {
		return "pong";
	}
}
