<cfscript>
// This helper file deliberately declares NO user functions, so the included
// sub-program's function table has length 1 (just __main__). Calling a CFC
// method that defines a closure from here exercises the issue #70 path: the
// method's DefineFunction(idx) index is out of bounds for this tiny sub-program
// and must be resolved against the enclosing (top-level) program.
request._issue70_closure = obj.runClosure();
request._issue70_arrow   = obj.runArrow();
request._issue70_nested  = obj.runNested();
// Closure created under the swap, invoked under the swap.
adderUnderSwap = obj.makeAdder(5);
request._issue70_adder_under_swap = adderUnderSwap(10);
</cfscript>
