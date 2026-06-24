<cfscript>
suiteBegin("Boolean-literal string equality coercion (Lucee parity)");

// CFML loose equality coerces a boolean-literal string (true/false/yes/no) to
// 1/0 and compares numerically. Wheels' urlFor checks
// `request.cgi.server_port_secure == "true"` against a value of 1.
assertTrue("1 == 'true'", 1 == "true");
assertTrue("'1' == 'true'", "1" == "true");
assertTrue("1 == 'yes'", 1 == "yes");
assertTrue("0 == 'false'", 0 == "false");
assertTrue("'no' == 0", "no" == 0);
assertTrue("'no' == 'false'", "no" == "false");
assertTrue("'yes' == 'true'", "yes" == "true");
assertTrue("true == 'true'", true == "true");
assertTrue("'true' == 1.0", "true" == 1.0);

// A bool literal coerces to exactly 1/0 — other numbers are NOT equal to it.
assertFalse("2 == 'true' (2 != 1)", 2 == "true");
assertFalse("2 == 'yes' (2 != 1)", 2 == "yes");
assertFalse("'yes' == 'no'", "yes" == "no");

// Empty string is not a boolean literal.
assertFalse("'' == 'false'", "" == "false");

// Plain (non-bool-literal) string equality is unchanged.
assertTrue("'abc' == 'ABC' (case-insensitive)", "abc" == "ABC");
assertFalse("'abc' == 'abd'", "abc" == "abd");
assertTrue("'5' == 5 (numeric string)", "5" == 5);

suiteEnd();
</cfscript>
