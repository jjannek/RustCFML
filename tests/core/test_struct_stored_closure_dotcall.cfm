<cfscript>
// Issue #169: an anonymous closure stored at a struct key must be invocable
// via dot-notation member-call (struct.key(args)), even when the key name
// collides with a struct member function (filter/map/each/find/append/...).
suiteBegin("Struct-stored closure dot-call (issue 169)");

// anonymous closure, dot-call, builtin-colliding key name
st = { filter = function(p){ return "got:" & p; } };
assert("anon closure dot-call (filter key)", st.filter("X"), "got:X");

// closure passed as an argument (the TestBox.cfc:834 shape)
function callFilter(d){ return d.filter("Y"); }
assert("closure-in-arg struct dot-call", callFilter({ filter = function(p){ return "ok:" & p; } }), "ok:Y");

// call-expression receiver
function getSt(){ return { filter = function(p){ return "got:" & p; } }; }
assert("call-expression receiver dot-call", getSt().filter("Z"), "got:Z");

// closure assigned to the key after creation
cb = function(p){ return "z:" & p; };
st2 = {}; st2.filter = cb;
assert("assigned-after-creation dot-call", st2.filter("Q"), "z:Q");

// bracket notation still works
assert("bracket-notation member-call", st["filter"]("B"), "got:B");

// pull into a variable, then call
pulled = st.filter;
assert("pull-then-call", pulled("Q"), "got:Q");

// named UDF reference in the same position (non-colliding key)
function greet(p){ return "hi:" & p; }
st3 = { g = greet };
assert("named UDF reference dot-call", st3.g("W"), "hi:W");

// REGRESSION: real struct member function still works when the key is NOT a
// function - the closure shadow must not break structFilter et al.
m = { a = 1, b = 2 };
filtered = m.filter(function(k, v){ return v GT 1; });
assert("real structFilter member still works", filtered.b, 2);
assert("real structCount member still works", m.count(), 2);

suiteEnd();
</cfscript>
