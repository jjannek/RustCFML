<cfscript>
suiteBegin("JSON Functions");

// --- serializeJSON ---
assert("serializeJSON integer", serializeJSON(42), "42");
assert("serializeJSON string", serializeJSON("hello"), '"hello"');
assert("serializeJSON boolean", serializeJSON(true), "true");

jsonArr = serializeJSON([1, 2, 3]);
assertTrue("serializeJSON array is valid JSON", isJSON(jsonArr));
assertTrue("serializeJSON array contains brackets", find("[", jsonArr) > 0);

jsonObj = serializeJSON({name: "test"});
assertTrue("serializeJSON struct is valid JSON", isJSON(jsonObj));
assertTrue("serializeJSON struct contains brace", find("{", jsonObj) > 0);

// --- deserializeJSON ---
assert("deserializeJSON integer", deserializeJSON("42"), 42);
assert("deserializeJSON string", deserializeJSON('"hello"'), "hello");

parsedArr = deserializeJSON("[1,2,3]");
assertTrue("deserializeJSON array isArray", isArray(parsedArr));
assert("deserializeJSON array length", arrayLen(parsedArr), 3);

parsedObj = deserializeJSON('{"name":"test"}');
assertTrue("deserializeJSON struct has key", structKeyExists(parsedObj, "name"));
assert("deserializeJSON struct value", parsedObj.name, "test");

// --- isJSON ---
assertTrue("isJSON number", isJSON("42"));
assertTrue("isJSON array", isJSON("[1,2,3]"));
assertTrue("isJSON object", isJSON('{"a":1}'));
assertFalse("isJSON invalid", isJSON("not json {"));

// --- round-trip ---
original = {a: 1, b: [2, 3]};
roundTrip = deserializeJSON(serializeJSON(original));
assert("round-trip struct key a", roundTrip.a, 1);
assertTrue("round-trip struct key b is array", isArray(roundTrip.b));
assert("round-trip array length", arrayLen(roundTrip.b), 2);

// --- circular references must NOT overflow the native stack (GitHub #178) ---
// Reference-typed structs/arrays can alias and form cycles (e.g. a TestBox mock
// holds this.mockBox, whose generator holds the mock back). Before the fix this
// recursed until the process aborted with an uncatchable SIGABRT. The cycle is
// broken with null so serialization stays total and non-crashing.
circStruct = {}; circStruct.name = "root"; circStruct.self = circStruct;
circJson = serializeJSON(circStruct);
assertTrue("circular struct serializes without crashing", len(circJson) > 0);
assertTrue("circular struct keeps non-cyclic data", findNoCase('"name":"root"', circJson) > 0);

circArr = []; arrayAppend(circArr, "x"); arrayAppend(circArr, circArr);
circArrJson = serializeJSON(circArr);
assertTrue("circular array serializes without crashing", len(circArrJson) > 0);

suiteEnd();
</cfscript>
