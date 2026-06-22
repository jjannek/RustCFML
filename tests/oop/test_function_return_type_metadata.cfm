<cfscript>
suiteBegin("Function returnType metadata");

obj = createObject("component", "oop.ReturnTypeMetaFixture");

// getMetadata on a method reference (the EventInterfaceSpec / Wheels idiom)
// must expose the declared returnType.
mString = getMetadata(obj["doString"]);
assert("declared returnType string", mString.returnType ?: "any", "string");

mVoid = getMetadata(obj["doVoid"]);
assert("declared returnType void", mVoid.returnType ?: "any", "void");

// Undeclared returnType: key absent -> elvis falls back to "any".
mNone = getMetadata(obj["doNone"]);
assert("undeclared returnType -> any", mNone.returnType ?: "any", "any");

// Parameters must still be present alongside returnType.
assert("parameters preserved", arrayLen(mString.parameters), 2);
assert("first param name", mString.parameters[1].name, "exception");

// Accessor getter return type reflects the property type.
mGet = getMetadata(obj["getTitle"]);
assert("getter returnType from property", mGet.returnType ?: "any", "string");

suiteEnd();
</cfscript>
