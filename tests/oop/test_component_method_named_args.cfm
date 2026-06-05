<cfscript>
suiteBegin("OOP: component method named arguments");

widget = createObject("component", "oop.NamedArgWidget");
args = { first = "A", second = "B", third = "C" };

assert("named arguments bind by parameter name", widget.combine(third="C", first="A", second="B"), "ABC");
assert("argumentCollection expands by parameter name", widget.combine(argumentCollection=args), "ABC");

// Mixing positional and named arguments is an error (matches Lucee): once any
// argument is named, all must be named.
function adder(a, b, c) { return a & b & c; }
assertThrows("mixing positional + named method args is rejected", function() {
    widget.combine("A", second="B", third="C");
});
assertThrows("mixing positional + named UDF args is rejected", function() {
    adder("A", c="C", b="B");
});

// Undeclared named arguments must populate the arguments scope BY NAME ONLY.
// A function with no declared parameters, called with named args, should have an
// arguments scope containing exactly those named keys — no spurious positional
// (numeric) entries. So StructCount(arguments) == the number of named args passed.
function argScopeCount() { return StructCount(arguments); }
assert("undeclared named args yield exactly n keys", argScopeCount(alpha="1", beta="2"), 2);
function argScopeKeys() { return listSort(structKeyList(arguments), "textnocase"); }
assert("undeclared named args are keyed by name only", argScopeKeys(alpha="1", beta="2"), "alpha,beta");

suiteEnd();
</cfscript>
