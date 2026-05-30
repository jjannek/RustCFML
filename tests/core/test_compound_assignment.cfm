<cfscript>
suiteBegin("Compound assignment with multi-op RHS");

// Regression: `x op= <expr>` where the RHS compiles to more than one bytecode
// op (array index, struct member, function call) used to be miscompiled — the
// compiler swapped instructions at compile time assuming a single-push RHS,
// which corrupted the bytecode (wrong result, and a hang inside a loop).

arr = [10, 20, 30, 40];

// --- += with array-index RHS ---
total = 0;
total += arr[1];
assert("+= arr[1] single", total, 10);

// --- += array index in a loop (the case that used to hang) ---
sum = 0;
for (i = 1; i <= arrayLen(arr); i++) {
    sum += arr[i];
}
assert("+= arr[i] loop sum", sum, 100);

// --- -= with array-index RHS (non-commutative: order matters) ---
x = 100;
x -= arr[2];
assert("-= arr[2] order", x, 80);

// --- *= with array-index RHS ---
y = 3;
y *= arr[1];
assert("*= arr[1]", y, 30);

// --- /= with array-index RHS (non-commutative) ---
z = 100;
z /= arr[1];
assert("/= arr[1] order", z, 10);

// --- %= with array-index RHS (non-commutative) ---
m = 17;
m %= arr[1];
assert("%= arr[1] order", m, 7);

// --- &= (concat) with array-index RHS ---
str = "n=";
str &= arr[3];
assert("&= arr[3] concat", str, "n=30");

// --- RHS is a struct member access (also multi-op) ---
cfg = { factor = 5, label = "x" };
p = 2;
p += cfg.factor;
assert("+= struct.member", p, 7);
q = "id:";
q &= cfg.label;
assert("&= struct.member", q, "id:x");

// --- RHS is a function call (multi-op) ---
function five() { return 5; }
fc = 10;
fc -= five();
assert("-= function() order", fc, 5);

// --- nested array index RHS ---
matrix = [[1, 2], [3, 4]];
n = 0;
n += matrix[2][1];
assert("+= nested arr[][] ", n, 3);

// --- chained in a loop, non-commutative ---
acc = 1000;
prices = [1, 2, 3, 4, 5];
for (i = 1; i <= arrayLen(prices); i++) {
    acc -= prices[i];
}
assert("-= arr[i] loop", acc, 985);

suiteEnd();
</cfscript>
