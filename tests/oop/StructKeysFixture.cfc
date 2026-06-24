component {
	property name="firstName";
	property name="lastName";

	public function init() {
		variables.firstName = "a";
		variables.lastName = "b";
		return this;
	}

	public function greet() {
		return "hi";
	}

	private function secret() {
		return "shh";
	}
}
