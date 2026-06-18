<cfscript>
suiteBegin("Super dispatch: case-insensitive resolution binds this");

// Regression: a lowercase super.onApplicationStart() must resolve to the
// parent's capital-O OnApplicationStart and run it with the child bound as
// `this`. Before the fix the __is_super path used an exact-case lookup, missed,
// fell through to generic struct dispatch, and ran the parent without `this`
// -> "Variable 'this' is undefined". Lucee binds `this` regardless.
child = createObject("component", "oop.SuperThisChild");

assert("child override runs and returns", child.onApplicationStart(), "child-ran");
assert("lowercase super call reached capital-O parent with this bound",
	child.getParentResult(), "parent-this");

suiteEnd();
</cfscript>
