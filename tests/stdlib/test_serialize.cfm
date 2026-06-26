<cfscript>
suiteBegin("serialize");

// Serialize() — CFML-literal serialisation (Lucee/ACF), inverse of Evaluate().
// Output is a CFML expression literal, NOT JSON: strings escape `"` by
// doubling it (`""`), no backslash escaping.

// --- Scalars ---
assert("integer", serialize(42), "42");
assert("double", serialize(3.5), "3.5");
assert("boolean true", serialize(true), "true");
assert("boolean false", serialize(false), "false");
assert("string", serialize("hello"), '"hello"');
assert("empty string", serialize(""), '""');
assert("null", serialize(nullValue()), "nullValue()");

// --- String escaping: embedded double-quote is doubled, not backslashed ---
assert("quote doubled", serialize('he"llo'), '"he""llo"');
// Newlines / tabs / backslashes are emitted verbatim (CFML-literal form)
assert("backslash literal", serialize("a\b"), '"a\b"');
assert("newline literal", serialize("a" & chr(10) & "b"), '"a' & chr(10) & 'b"');

// --- Arrays ---
assert("array", serialize([1,2,3]), "[1,2,3]");
assert("empty array", serialize([]), "[]");
assert("array of strings", serialize(["a","b"]), '["a","b"]');

// --- Structs ---
assert("empty struct", serialize({}), "{}");
// Lucee uppercases + reorders struct keys (a side effect of its internal
// storage); RustCFML preserves key case and insertion order engine-wide, the
// same convention serializeJSON follows. Exact-format struct assertions are
// therefore RustCFML-only; cross-engine coverage is via the round-trips below.
if (isRustCFML()) {
    assert("struct", serialize({a:1, b:"x"}), '{"a":1,"b":"x"}');
    assert("struct key with space", serialize({"a b":1}), '{"a b":1}');
    assert("nested", serialize([1, {x:[2,3]}, "a"]), '[1,{"x":[2,3]},"a"]');
}

// --- Round-trips via evaluate() ---
assertTrue("roundtrip array", isArray(evaluate(serialize([1,2,3]))));
rt = evaluate(serialize({a:1, b:"x"}));
assert("roundtrip struct a", rt.a, 1);
assert("roundtrip struct b", rt.b, "x");
rtStr = evaluate(serialize('say "hi"'));
assert("roundtrip quoted string", rtStr, 'say "hi"');
rtNested = evaluate(serialize([1, {x:[2,3]}, "a"]));
assert("roundtrip nested element", rtNested[2].x[2], 3);

suiteEnd();
</cfscript>
