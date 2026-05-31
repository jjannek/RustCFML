<cfscript>
suiteBegin("Functions");

// --- Basic UDF ---
function add(a, b) {
    return a + b;
}
assert("basic UDF", add(3, 4), 7);

// --- Function with no return (returns null implicitly) ---
function doNothing() {
    var x = 1;
}
assertTrue("function with no explicit return", isNull(doNothing()));

// --- Recursion (factorial) ---
function factorial(n) {
    if (n <= 1) return 1;
    return n * factorial(n - 1);
}
assert("recursion factorial(5)", factorial(5), 120);
assert("recursion factorial(1)", factorial(1), 1);
assert("recursion factorial(0)", factorial(0), 1);

// --- Default argument values ---
function greet(name, greeting="Hello") {
    return greeting & " " & name;
}
assert("default arg used", greet("World"), "Hello World");
assert("default arg overridden", greet("World", "Hi"), "Hi World");

// --- Optional argument omission ---
function checkOffset(offset) {
    if (structKeyExists(arguments, "offset")) {
        return "present:" & arguments.offset;
    }
    return "missing";
}
assert("omitted optional argument absent from arguments", checkOffset(), "missing");
assert("provided optional argument present in arguments", checkOffset(5), "present:5");

function pageArgs(table_name, offset, limit=250) {
    return (structKeyExists(arguments, "offset") ? "offset" : "no-offset") & ":" & arguments.limit;
}
assert("named later argument does not materialize omitted optional argument", pageArgs(table_name="moo_role", limit=20), "no-offset:20");

// --- Closures ---
multiply = function(a, b) {
    return a * b;
};
assert("closure basic", multiply(3, 5), 15);

// --- Arrow functions ---
double = (x) => x * 2;
assert("arrow function", double(7), 14);

// --- Arrow function with block body ---
clamp = function(val, lo, hi) {
    if (val < lo) return lo;
    if (val > hi) return hi;
    return val;
};
assert("closure block body - clamped low", clamp(-5, 0, 10), 0);
assert("closure block body - clamped high", clamp(15, 0, 10), 10);
assert("closure block body - in range", clamp(5, 0, 10), 5);

// --- Function as argument (higher-order) ---
function applyOp(a, b, op) {
    return op(a, b);
}
result = applyOp(10, 3, function(x, y) { return x - y; });
assert("function as argument", result, 7);

arrowResult = applyOp(4, 5, function(x, y) { return x + y; });
assert("closure as argument", arrowResult, 9);

// --- Nested function calls ---
function square(n) { return n * n; }
function sumOfSquares(a, b) { return square(a) + square(b); }
assert("nested function calls", sumOfSquares(3, 4), 25);

// --- Access modifiers ---
public function pubFn() { return "public"; }
private function privFn() { return "private"; }
assert("public function", pubFn(), "public");
assert("private function (direct call)", privFn(), "private");

// --- Function returning function (closure factory) ---
function makeAdder(n) {
    return function(x) {
        return x + n;
    };
}
addFive = makeAdder(5);
assert("closure factory", addFive(10), 15);

addTen = makeAdder(10);
assert("closure factory second instance", addTen(3), 13);

// --- Closure captures variable by reference ---
base = 100;
addToBase = function(x) { return base + x; };
assert("closure captures outer var", addToBase(5), 105);

suiteEnd();
</cfscript>
