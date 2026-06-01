// Fixture: a component with a method literally named `new`. On Lucee/Adobe CF/
// BoxLang `new` is a SOFT keyword — it introduces the `new Foo()` operator, but
// is equally legal as a function name, and calling it via `this.new()` works.
// Wheels' core object-creation API depends on this: `model("User").new()` is
// backed by `public any function new(...)` in vendor/wheels/model/create.cfc.
// (Originally from PR #32 by bpamiri; the underlying support landed in PR #30.)
component {
	public string function new() {
		return "made";
	}
	public string function probe() {
		return this.new();
	}
}
