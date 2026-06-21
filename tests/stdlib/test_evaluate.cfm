<cfscript>
suiteBegin("evaluate");

// --- Basic arithmetic ---
assert("addition", evaluate("4 + 5"), 9);
assert("precedence", evaluate("2 + 3 * 4"), 14);
assert("parentheses", evaluate("(2 + 3) * 4"), 20);

// --- Variable references resolve against the calling scope ---
x = 10;
y = 3;
assert("vars in expression", evaluate("x * y + 1"), 31);

// --- Dynamic variable-name resolution (the classic evaluate() use) ---
mySecret = "hidden";
theName = "mySecret";
assert("dynamic var name", evaluate(theName), "hidden");

// --- Boolean expressions (Preside feature-flag shape) ---
featureEnabled = true;
otherFeature = false;
assert("boolean and/not", evaluate("featureEnabled AND NOT otherFeature"), true);
assert("boolean or", evaluate("featureEnabled OR otherFeature"), true);

// --- Built-in function calls inside the expression ---
assert("fn call in expr", evaluate("ucase('hi') & len('abcd')"), "HI4");

// --- Nested struct / array access ---
data = { a: { b: 42 } };
assert("nested struct access", evaluate("data.a.b"), 42);

list = [10, 20, 30];
assert("array index access", evaluate("list[2]"), 20);

// --- Multiple arguments: returns the value of the last ---
assert("multi-arg last wins", evaluate("1+1", "2+2", "3+3"), 6);

// --- Works inside a function: sees arguments + local scope ---
evalInFn = function(p) {
    var q = 5;
    return evaluate("p + q");
};
assert("arguments + local scope", evalInFn(100), 105);

// --- Parse error surfaces as a catchable exception ---
assertThrows("invalid expression throws", function() {
    evaluate("4 +* 5");
});

suiteEnd();
</cfscript>
