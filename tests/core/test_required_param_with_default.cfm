<cfscript>
suiteBegin("Required param with a default value");

// A param declared `required` AND given a default must NOT be enforced as
// required when omitted — the default satisfies it (Lucee/ACF/BoxLang parity;
// TestBox's TestResult.cfc `incrementSuites( required count = 1 )` relies on it).
function f( required count = 1 ) { return count; }
assert("required+default, omitted -> uses default", f(), 1);
assert("required+default, supplied -> uses arg", f(5), 5);

// Plain required (no default) must still throw when omitted.
function g( required key ) { return key; }
assertThrows("plain required still enforced", function() { g(); });

// Required+default mixed with a following plain optional param.
function h( required a = 10, b = 20 ) { return a & "-" & b; }
assert("required+default first, both omitted", h(), "10-20");
assert("required+default first, first supplied", h(1), "1-20");
assert("required+default first, both supplied", h(1, 2), "1-2");

suiteEnd();
</cfscript>
