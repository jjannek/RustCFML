<cfscript>
suiteBegin("Core: canonical invoke() forms");

// ============================================================
// Background
// ============================================================
// CFML exposes two portable ways to invoke a component method dynamically, both
// of which RustCFML and Lucee/Adobe CF/BoxLang agree on:
//
//   1. The positional invoke() BIF:  invoke(objectOrName, methodName, args)
//   2. The script statement form:    invoke component=".." method=".." returnvariable=".." { invokeargument ... }
//
// (The named-argument FUNCTION-CALL form `invoke(component=.., method=..)` /
// the cf-prefixed `cfinvoke(..)` call are intentionally NOT covered: Lucee
// rejects them at compile time — the invoke() BIF is positional — so they are
// not a cross-engine contract. This suite pins the two forms that ARE.)
// ============================================================

// 1a. positional BIF, by component name + argument-collection struct
assert("invoke(name, method, argStruct)", invoke("InvokeTargetFixture", "greet", { who = "A" }), "hi A");

// 1b. positional BIF, by component instance
target = createObject("component", "InvokeTargetFixture");
assert("invoke(instance, method, argStruct)", invoke(target, "greet", { who = "B" }), "hi B");

// 1c. positional BIF, default argument
assert("invoke(name, method) uses the method default", invoke("InvokeTargetFixture", "greet"), "hi world");

// 2. statement form with returnvariable + invokeargument
invoke component="InvokeTargetFixture" method="greet" returnvariable="stmtResult" {
	invokeargument name="who" value="S";
}
assert("invoke statement form with invokeargument", stmtResult, "hi S");

suiteEnd();
</cfscript>
