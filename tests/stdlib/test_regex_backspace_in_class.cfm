<cfscript>
suiteBegin("Regex \b inside a character class");

// In Java/Lucee/ACF regex, `\b` INSIDE a character class `[...]` means the
// backspace char (U+0008); outside a class it's a word boundary. The Rust regex
// crate rejects `\b` inside a class as a compile error, and RustCFML's regex
// callers silently swallow compile errors as "no match" — so a pattern like
// Wheels autoLink's `[^\s\b]+` became a complete no-op (URLs never linked).
// RustCFML now rewrites in-class `\b` to `\x08` before compiling.

// The autoLink-style pattern matches non-whitespace runs.
assert("reFind [^\s\b]+ matches", reFind("[^\s\b]+", "abc"), 1);
assert("reFind [^\s\b]+ skips leading space", reFind("[^\s\b]+", " abc"), 2);
assert("reReplace [^\s\b]+ all", reReplace("a b c", "[^\s\b]+", "X", "all"), "X X X");

// A real URL inside surrounding text (the autoLink trigger).
assertTrue("URL run found via [^\s\b]+", reFind("(https?://[^\s\b]+)", "go http://wheels.dev now") GT 0);

// Word-boundary \b OUTSIDE a class is unaffected.
assert("word boundary \b still works", reFind("\bfoo\b", "a foo b"), 3);

suiteEnd();
</cfscript>
