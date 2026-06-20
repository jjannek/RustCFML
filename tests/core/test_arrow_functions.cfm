<cfscript>
// Test arrow functions with multiple parameters
suiteBegin("Arrow Functions");

// Test 1: Single parameter arrow function
double = (x) => x * 2;
assertTrue("single param arrow", double(5) eq 10);

// Test 2: Two parameter arrow function
add = (a, b) => a + b;
assertTrue("two param arrow", add(3, 4) eq 7);

// Test 3: Three parameter arrow function
sum3 = (a, b, c) => a + b + c;
assertTrue("three param arrow", sum3(1, 2, 3) eq 6);

// Test 4: Arrow function with method chaining
arr = [1, 2, 3, 4, 5];
result = arr.map((x) => x * 2).filter((x) => x gt 5);
assertTrue("chained arrow map/filter", arrayToList(result) eq "6,8,10");

// Test 5: Arrow function with reduce
total = arr.reduce((acc, x) => acc + x, 0);
assertTrue("arrow reduce", total eq 15);

// Test 6: Arrow function in array operations
squared = arr.map((n) => n * n);
assertTrue("arrow map square", arrayToList(squared) eq "1,4,9,16,25");

// Test 7: Block-bodied arrow function
addOne = (x) => { return x + 1; };
assertTrue("block-body arrow single statement", addOne(4) eq 5);

// Test 8: Block-bodied arrow with multiple statements
items = [{ n: 1 }, { n: 2 }, { n: 3 }];
mapped = items.map((it) => {
    it.doubled = it.n * 2;
    it.label = "n=" & it.n;
    return it;
});
assertTrue("block-body arrow multi-statement doubled", mapped[2].doubled eq 4);
assertTrue("block-body arrow multi-statement label", mapped[3].label eq "n=3");

// Issue #185: arrow functions with default parameter values.
withDefaults = (a = 1, b = 2) => { return a + b; };
assertTrue("arrow defaults - all omitted", withDefaults() eq 3);
assertTrue("arrow defaults - first supplied", withDefaults(10) eq 12);
assertTrue("arrow defaults - both supplied", withDefaults(10, 20) eq 30);

singleDefault = (a = 5) => { return a; };
assertTrue("arrow single default omitted", singleDefault() eq 5);
assertTrue("arrow single default supplied", singleDefault(9) eq 9);

// Expression-bodied arrow with a default.
exprDefault = (a = 7) => a * 2;
assertTrue("arrow expr-body default", exprDefault() eq 14);

// Default referencing an earlier param + a non-default trailing param.
mixedDefault = (a, b = 3) => { return a * b; };
assertTrue("arrow mixed default", mixedDefault(4) eq 12);

suiteEnd();
</cfscript>
