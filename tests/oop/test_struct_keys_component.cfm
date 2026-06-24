<cfscript>
// A CFC instance is materialised internally as a marker-bearing struct, but
// StructKeyList/StructKeyArray/StructKeyExists/StructCount must expose ONLY
// public members (Lucee/ACF parity) — never engine internals like __name,
// __variables, __properties, __source_file, __metadata, or the `this` alias.
// Leaking __variables made Wheels' toXML/$structToXML recurse without bound
// (renderWith(data=model) tripped the depth-256 guard). These also keep
// StructKeyList in agreement with for-in iteration over the same component.
suiteBegin("Component struct-key visibility");

obj = new oop.StructKeysFixture();

keyList = listToArray(StructKeyList(obj));
arraySort(keyList, "textnocase");

// Public methods are visible; private methods and ALL internals are hidden.
assert("StructKeyList exposes only public members", arrayToList(keyList), "greet,init");
assertFalse("StructKeyList hides __variables", listFindNoCase(StructKeyList(obj), "__variables") > 0);
assertFalse("StructKeyList hides __name", listFindNoCase(StructKeyList(obj), "__name") > 0);
assertFalse("StructKeyList hides this", listFindNoCase(StructKeyList(obj), "this") > 0);
assertFalse("StructKeyList hides private method", listFindNoCase(StructKeyList(obj), "secret") > 0);

// StructKeyArray agrees with StructKeyList.
ka = StructKeyArray(obj);
arraySort(ka, "textnocase");
assert("StructKeyArray matches StructKeyList", arrayToList(ka), "greet,init");

// StructKeyExists must not see internals.
assertFalse("StructKeyExists false for __variables", StructKeyExists(obj, "__variables"));
assertFalse("StructKeyExists false for __name", StructKeyExists(obj, "__name"));
assertTrue("StructKeyExists true for a public method", StructKeyExists(obj, "greet"));

// for-in iteration yields the same visible keys.
forInKeys = [];
for (k in obj) { arrayAppend(forInKeys, k); }
arraySort(forInKeys, "textnocase");
assert("for-in matches StructKeyList", arrayToList(forInKeys), "greet,init");

// StructCount counts only the visible members.
assert("StructCount counts only public members", StructCount(obj), 2);

suiteEnd();
</cfscript>
