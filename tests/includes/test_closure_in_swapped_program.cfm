<cfscript>
suiteBegin("Closure defined in CFC method under swapped program (issue 70)");

// Instantiate a CFC in the top-level program, then call its closure-defining
// methods from inside a cfinclude whose sub-program has no user functions.
// Before the fix this panicked: "index out of bounds: the len is 1".
obj = new Issue70Closure();
include "helper_issue70_closure.cfm";

assert("closure defined in CFC method, called under include swap", request._issue70_closure, "closure-ok");
assert("arrow defined in CFC method, called under include swap", request._issue70_arrow, "arrow-ok");
assert("nested closure under include swap", request._issue70_nested, "nested-ok");
assert("closure factory invoked under the swap", request._issue70_adder_under_swap, 15);

// Closure created and invoked AFTER the include has restored the program.
adderAfter = obj.makeAdder(100);
assert("closure factory invoked after swap restored", adderAfter(1), 101);

suiteEnd();
</cfscript>
