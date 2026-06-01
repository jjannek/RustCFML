// Gap D fixture: a method whose parameter carries a DOTTED FQN type annotation
// (`wheels.system.TestResult`). On Lucee/Adobe CF/BoxLang a dotted FQN is a
// legal parameter type; RustCFML used to reject the first `.` ("Expected
// RParen, found Dot"), degrading the CFC to a non-object. The type is only an
// annotation here — the method is never called, so no instance of the (absent)
// type is required and no type-coercion is triggered.
//
// Mirrors the wheelstest runners: `required wheels.wheelstest.system.TestResult results`.
component {
	public string function handle( required wheels.system.TestResult result ) {
		return "ok";
	}
	public string function ping() {
		return "pong";
	}
}
