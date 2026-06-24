<cfscript>
suiteBegin("NumberFormat mask integer-part padding (Lucee parity)");

// A `0` placeholder the number doesn't fill becomes '0'; a `9`/`_` placeholder
// becomes a space. (Wheels date-select helpers zero-pad minutes/seconds via
// NumberFormat(n, "09").)
assert("zero-pad single digit", NumberFormat(1, "09"), "01");
assert("zero-pad already two digits", NumberFormat(59, "09"), "59");
assert("space-pad with 9 placeholder", NumberFormat(5, "99"), " 5");
assert("zero-pad with 00", NumberFormat(5, "00"), "05");
assert("number longer than mask shows all digits", NumberFormat(123, "09"), "123");

// Existing behaviour unchanged: thousands + decimals.
assert("comma + decimals", NumberFormat(1234.5, "9,999.99"), "1,234.50");

suiteEnd();
</cfscript>
