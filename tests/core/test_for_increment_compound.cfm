<cfscript>
suiteBegin("Core: compound assignment in for-increment");

// ============================================================
// Background
// ============================================================
// The increment clause of a C-style for loop is parsed as an expression, which
// did not accept compound-assignment operators, so `for (i=1; i<=10; i+=2)`
// failed ("Expected RParen, found PlusEqual") even though a standalone
// `i += 2;` statement parses and runs. Lucee/Adobe CF/BoxLang all accept the
// compound form in the increment. Used in vendor/wheels/model/bulk.cfc.
// ============================================================

// += in the increment
plusSum = 0;
for (i = 1; i <= 10; i += 2) { plusSum += i; }
assert("+= increment iterates 1,3,5,7,9", plusSum, 25);

// -= in the increment
minusSeq = "";
for (j = 3; j > 0; j -= 1) { minusSeq &= j; }
assert("-= increment counts down", minusSeq, "321");

// *= in the increment
lastK = 0;
for (k = 1; k <= 4; k *= 2) { lastK = k; }
assert("*= increment doubles (1,2,4)", lastK, 4);

// &= (string concat) in the increment position
concatStr = "x";
for (n = 0; n < 3; n += 1) { concatStr &= "-"; }
assert("loop body runs the expected number of times", concatStr, "x---");

suiteEnd();
</cfscript>
