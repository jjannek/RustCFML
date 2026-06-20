component {
	// Chained assignment with `this.X` as the inner target: the value must land
	// in BOTH variables.greeting and this.greeting. (TestBox/Wheels BaseSpec does
	// `variables.$assert = this.$assert = new Assertion()`.)
	function init() {
		variables.greeting = this.greeting = "hello";
		return this;
	}
	function report() {
		return "variables=" & variables.greeting & "|this=" & this.greeting;
	}
}
