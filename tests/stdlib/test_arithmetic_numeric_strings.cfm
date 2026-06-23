<cfscript>
suiteBegin("Arithmetic + on numeric strings");

// CFML `+` is ARITHMETIC ONLY (`&` is concatenation). Two numeric strings must
// ADD, not concatenate. RustCFML previously had a String+String fast path in
// the Add op (and its JIT shim) that concatenated "2"+"1" → "21", inverting
// Wheels' if/unless method-mixin validations (`stupid_mixin(a,b){return a+b}`).
assert("two numeric strings add", "2" + "1", 3);
assert("multi-digit numeric strings add", "20" + "13", 33);
assert("numeric string + int", "2" + 5, 7);
assert("int + numeric string", 5 + "2", 7);
assert("decimal strings add", "1.5" + "2.5", 4);

// `&` still concatenates (unchanged).
assert("ampersand still concatenates", "2" & "1", "21");

// A function mixin returning a+b on string args (the Wheels pattern).
mixin = function(a, b) { return a + b; };
assert("mixin a+b on string args", mixin("2", "1"), 3);

// Genuinely non-numeric operands still fall back to concatenation (lenient).
assert("non-numeric + concatenates", "foo" + "bar", "foobar");

suiteEnd();
</cfscript>
