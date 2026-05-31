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

suiteEnd();
</cfscript>
