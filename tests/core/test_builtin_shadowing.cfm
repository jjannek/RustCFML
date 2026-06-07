<cfscript>
suiteBegin("Core: builtin-name shadowing (Lucee parity)");

// `function abs(x) { ... }` is a hard error on Lucee 7
// ("The name [abs] is already used by a built in Function"). RustCFML
// must do the same — see `tests/core/fixtures/redef_abs.cfm`.
assertThrows("redefining abs() as a user function throws", function() {
    include "fixtures/redef_abs.cfm";
});

// Closures and arrow functions are anonymous, so they must NOT trip the
// guard even if assigned to a name that happens to match a builtin (the
// reassignment doesn't change `vm.user_functions`).
myAbs = function(x) { return x < 0 ? -x : x; };
assert("closure assigned to a name-shadowed local still works", myAbs(-3), 3);

myMax = (a, b) => a > b ? a : b;
assert("arrow function assigned to a builtin-named local still works", myMax(2, 5), 5);

// A normal, non-colliding user function should be untouched.
function safeName(n) { return n + 1; }
assert("non-colliding function decl is unaffected", safeName(41), 42);

suiteEnd();
</cfscript>
