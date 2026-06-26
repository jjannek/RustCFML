<cfscript>
suiteBegin("Strict Equality (=== / !==)");

// Strict equality (Lucee/ACF/BoxLang): same-type equality, no cross-type
// coercion. Verified against Lucee 7.

// --- Strings: case-insensitive, but no numeric/bool coercion or trimming ---
assertTrue("str equal", "a" === "a");
assertTrue("str case-insensitive", "a" === "A");
assertFalse("str trailing space differs", "abc" === "abc ");
assertFalse("numeric strings not coerced", "1" === "1.0");
assertTrue("empty strings equal", "" === "");

// --- Numbers: int/double are the same (numeric) type ---
assertTrue("int equal", 1 === 1);
assertTrue("int vs double numeric", 1 === 1.0);
assertFalse("double not equal", 1.0 === 1.5);

// --- Booleans ---
assertTrue("bool equal", true === true);
assertFalse("bool not equal", true === false);

// --- Cross-type is always false (the whole point of ===) ---
assertFalse("int vs string", 1 === "1");
assertFalse("string vs int", "1" === 1);
assertFalse("bool vs int", true === 1);
assertFalse("bool vs string", true === "true");

// --- Null ---
assertTrue("null === null", nullValue() === nullValue());

// --- !== strict inequality ---
assertTrue("strict neq differing strings", "a" !== "b");
assertFalse("strict neq equal strings", "a" !== "a");
assertTrue("strict neq cross-type", 1 !== "1");
// The Preside case that motivated the issue: `x !== "none"`
x = "something";
assertTrue("preside-style neq", x !== "none");

// --- Reference types compare by identity (same backing store) ---
a = [1,2];
b = [1,2];
c = a;
assertTrue("same array ref", a === c);
assertFalse("distinct equal arrays", a === b);
s1 = {k:1};
s3 = s1;
assertTrue("same struct ref", s1 === s3);
assertFalse("distinct equal structs", s1 === {k:1});

suiteEnd();
</cfscript>
