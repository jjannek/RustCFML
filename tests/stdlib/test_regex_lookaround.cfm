<cfscript>
suiteBegin("Regex lookaround & backreferences");

// Java/Lucee/ACF regex supports lookahead, lookbehind and backreferences. The
// Rust `regex` crate does NOT — it rejects those patterns at compile time, and
// RustCFML's regex callers silently swallow compile errors as "no match", so a
// pattern using them became a complete no-op. RustCFML now falls back to
// `fancy-regex` when the fast engine rejects a pattern, so these now work.
// (This was the root cause of Preside PresideObjectService test094: its
// `_escapeAlias` uses `\bas\b\s+(\w+)(?!\s*[`"\[])$`, a negative lookahead.)

// --- Negative lookahead ---
assertTrue("neg lookahead matches when assertion holds", reFind("foo(?!bar)", "foobaz") GT 0);
assert("neg lookahead fails when assertion violated", reFind("foo(?!bar)", "foobar"), 0);

// --- Positive lookahead ---
assertTrue("pos lookahead matches", reFind("foo(?=bar)", "foobar") GT 0);
assert("pos lookahead non-match", reFind("foo(?=bar)", "foobaz"), 0);

// --- Lookbehind ---
assertTrue("pos lookbehind matches", reFind("(?<=@)\w+", "user@example") GT 0);
assert("neg lookbehind", reFind("(?<!@)\bbar", "@bar"), 0);

// --- Backreference ---
assertTrue("backreference matches doubled word", reFind("(\w+)\s+\1", "hello hello world") GT 0);
assert("backreference non-match", reFind("(\w+)\s+\1", "hello world"), 0);

// --- The Preside _escapeAlias pattern: wrap a trailing `as <alias>` (when the
//     alias isn't already quoted) in backticks. ---
escaped = REReplaceNoCase( "`object_1`.`label` as labelAlias", '\bas\b\s+(\w+)(?!\s*[`"\[])$', "as `\1`" );
assert("escapeAlias wraps bare alias in backticks", escaped, "`object_1`.`label` as `labelAlias`");
// Already-quoted alias is left alone (lookahead sees the backtick).
alreadyQuoted = REReplaceNoCase( "`object_1`.`label` as `labelAlias`", '\bas\b\s+(\w+)(?!\s*[`"\[])$', "as `\1`" );
assert("escapeAlias leaves quoted alias untouched", alreadyQuoted, "`object_1`.`label` as `labelAlias`");

// --- reReplace with lookaround + CFML backreference replacement ---
assert("reReplace lookahead all", reReplace("a1 b2 cc", "([a-z])(?=[0-9])", "[\1]", "all"), "[a]1 [b]2 cc");

// --- reMatch with lookahead ---
m = reMatch("\d+(?=px)", "10px 20em 30px");
assert("reMatch lookahead count", arrayLen(m), 2);
assert("reMatch lookahead first", m[1], "10");
assert("reMatch lookahead second", m[2], "30");

// --- Patterns the fast engine handles must still work unchanged ---
assert("plain pattern unaffected", reFind("\d+", "abc123"), 4);
assert("plain reReplace unaffected", reReplace("a-b-c", "-", "_", "all"), "a_b_c");

suiteEnd();
</cfscript>
