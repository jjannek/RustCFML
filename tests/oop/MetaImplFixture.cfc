/**
 * Component that implements an interface — its metadata must expose an
 * `implements` struct keyed by the declared interface FQN (Wheels' interface
 * specs detect a declared contract this way).
 */
component implements="oop.MetaIFaceFixture" {
	public string function greet() {
		return "hi";
	}
}
