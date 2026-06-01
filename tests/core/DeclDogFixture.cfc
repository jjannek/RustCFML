// A component implementing the child interface (which itself extends a base
// interface). Proves the parsed interface chain is usable: the component
// instantiates and its declared methods run on every engine.
component implements="IDeclDog" {
	public string function species() { return "canine"; }
	public string function bark() { return "woof"; }
}
