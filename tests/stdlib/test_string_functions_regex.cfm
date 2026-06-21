<cfscript>
suiteBegin("String Functions: Regex");

// --- reFind ---
assertTrue("reFind digits found", reFind("[0-9]+", "abc123def") > 0);
assert("reFind no digits", reFind("[0-9]+", "abcdef"), 0);

// --- reFindNoCase ---
assertTrue("reFindNoCase letters", reFindNoCase("[A-Z]+", "hello") > 0);

// --- reReplace ---
assert("reReplace first", reReplace("abc123def", "[0-9]+", "NUM"), "abcNUMdef");
assert("reReplace all", reReplace("abc123def456", "[0-9]+", "NUM", "all"), "abcNUMdefNUM");

// --- reReplace capture-group backreferences ---
// The replacement string may reference captured groups with \1..\9. Standard CFML
// (Lucee/Adobe/BoxLang) substitutes the captured text; a literal "\1" must not survive.
assert("reReplace backref single", reReplace("foo123bar", "([0-9]+)", "[\1]"), "foo[123]bar");
assert("reReplace backref swap", reReplace("John Smith", "(\w+) (\w+)", "\2 \1"), "Smith John");
assert("reReplace backref all", reReplace("a1b2c3", "([a-z])([0-9])", "\2\1", "all"), "1a2b3c");
// Case-modifier escapes in the replacement: \u/\l upper/lower the next char, \U/\L the rest.
assert("reReplace backref upper-first", reReplace("hello", "(h)(ello)", "\u\1\2"), "Hello");
assert("reReplace backref title-case all", reReplace("route-tester", "(^|-)([a-z])", "\u\2", "all"), "RouteTester");
assert("reReplace backref upper-rest", reReplace("abc", "(.*)", "\U\1"), "ABC");

// --- reReplaceNoCase ---
assert("reReplaceNoCase all", reReplaceNoCase("Hello World", "[a-z]+", "X", "all"), "X X");

// --- reMatch ---
matches = reMatch("[0-9]+", "abc123def456");
assert("reMatch count", arrayLen(matches), 2);
assert("reMatch first", matches[1], "123");
assert("reMatch second", matches[2], "456");

// --- reMatchNoCase ---
wordMatches = reMatchNoCase("[a-z]+", "Hello World");
assert("reMatchNoCase count", arrayLen(wordMatches), 2);
assert("reMatchNoCase first", wordMatches[1], "Hello");
assert("reMatchNoCase second", wordMatches[2], "World");

// --- compiled-regex cache correctness (v0.189.0) ---
// Compiled regexes are cached keyed by the final pattern string. The case-
// insensitive variants fold a "(?i)" prefix into that key, so the SAME base
// pattern must NOT collide between the case-sensitive and case-insensitive
// forms, and repeated calls (cache hits) must return identical results.
assert("cache: case-sensitive miss", reFind("[a-z]+", "ABC"), 0);
assert("cache: case-insensitive hit", reFindNoCase("[a-z]+", "ABC"), 1);
assert("cache: case-sensitive still miss after NoCase", reFind("[a-z]+", "ABC"), 0);
assert("cache: repeated call stable 1", reReplace("a1b2", "[0-9]", "X", "all"), "aXbX");
assert("cache: repeated call stable 2", reReplace("a1b2", "[0-9]", "X", "all"), "aXbX");
assert("cache: NoCase vs sensitive distinct", reReplaceNoCase("AbC", "b", "X"), "AXC");
assert("cache: sensitive same pattern matches lower", reReplace("AbC", "b", "X"), "AXC");
assert("cache: sensitive pattern no match on upper", reReplace("ABC", "b", "X"), "ABC");

// --- reReplace replacement-string backslash semantics (Lucee 7 parity) ---
// Only \0-\9 (backreferences) and \u \l \U \L \E (case modifiers) are special
// in a replacement string. Every OTHER escape keeps its backslash VERBATIM —
// \n \t \r stay literal two-char sequences, and \\, \d, \/ etc. are not
// interpreted. (Previously RustCFML expanded \n/\t/\r and dropped the backslash
// on unknown escapes, breaking Wheels routing/validation replacements.)
assert("reReplace backref kept", reReplace("abc", "(b)", "[\1]"), "a[b]c");
assert("reReplace \d keeps backslash", reReplace("abc", "b", "X\dY"), "aX\dYc");
assert("reReplace \\ keeps both", reReplace("abc", "b", "X\\Y"), "aX\\Yc");
assert("reReplace \/ keeps backslash", reReplace("a-b", "-", "\/"), "a\/b");
assert("reReplace \n stays literal", reReplace("ab", "b", "[\n]"), "a[\n]");
assert("reReplace \t stays literal", reReplace("ab", "b", "[\t]"), "a[\t]");
assert("reReplace \w keeps backslash", reReplace("abc", "b", "\w"), "a\wc");
assert("reReplace \U..\E uppercases backref", reReplace("a-hello-z", "(hello)", "\U\1\E"), "a-HELLO-z");
assert("reReplace \l lowercases next", reReplace("aBc", "(B)", "\l\1"), "abc");

suiteEnd();
</cfscript>
