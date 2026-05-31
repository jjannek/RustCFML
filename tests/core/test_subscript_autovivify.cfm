<cfscript>
suiteBegin("Core: subscript auto-vivification + verbose operator aliases");

// ------------------------------------------------------------
// Subscript-assigning into a variable that does not yet exist creates it,
// matching Lucee/ACF/BoxLang. A string key vivifies a struct; a numeric
// index vivifies a (1-based, auto-growing) array.
// ------------------------------------------------------------
rcfmlAutoVivStruct["alpha"] = 1;
rcfmlAutoVivStruct["beta"]  = 2;
assertTrue("undefined var subscript-assigned with a string key becomes a struct",
    isStruct(rcfmlAutoVivStruct));
assert("auto-vivified struct keeps both keys", structKeyList(rcfmlAutoVivStruct), "alpha,beta");

rcfmlAutoVivArray[3] = "c";
assertTrue("undefined var subscript-assigned with a numeric index becomes an array",
    isArray(rcfmlAutoVivArray));
assert("auto-vivified array auto-grows to the index", arrayLen(rcfmlAutoVivArray), 3);

// ------------------------------------------------------------
// Verbose, multi-word comparison operator aliases (Lucee/ACF/BoxLang).
// ------------------------------------------------------------
assertTrue("IS NOT",                    1 IS NOT 2);
assertTrue("NOT EQUAL",                 1 NOT EQUAL 2);
assertTrue("EQUAL",                     2 EQUAL 2);
assertTrue("GREATER THAN",              5 GREATER THAN 3);
assertTrue("LESS THAN",                 3 LESS THAN 5);
assertTrue("GREATER THAN OR EQUAL TO",  5 GREATER THAN OR EQUAL TO 5);
assertTrue("LESS THAN OR EQUAL TO",     4 LESS THAN OR EQUAL TO 4);
assertTrue("DOES NOT CONTAIN",          "abc" DOES NOT CONTAIN "z");

// The operator words must remain usable as ordinary identifiers.
greater = 7; than = 8; equal = 9; less = 10; does = 11; contain = 12;
assert("operator words still usable as variable names",
    greater & than & equal & less & does & contain, "789101112");

suiteEnd();
</cfscript>
