<cfscript>
// UDF→UDF call graph — Phase-2 (v0.86.0) sweet spot. Mutual recursion +
// 3-cycle; should be ~50-100x faster than interpreter. Baseline.
function isEven(n) { if (n == 0) { return 1; } return isOdd(n - 1); }
function isOdd(n)  { if (n == 0) { return 0; } return isEven(n - 1); }

function fa(n) { if (n <= 0) { return 0; } return 1 + fb(n - 1); }
function fb(n) { if (n <= 0) { return 0; } return 1 + fc(n - 1); }
function fc(n) { if (n <= 0) { return 0; } return 1 + fa(n - 1); }

total = 0;
for (k = 1; k <= 200000; k++) { total = total + isEven(18); }
cycle  = 0;
for (k = 1; k <= 200000; k++) { cycle  = cycle  + fa(15); }
writeOutput(total & ":" & cycle);

</cfscript>
