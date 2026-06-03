<cfscript>
suiteBegin("as_string cycle safety");

// Regression: structs are reference types, so an object graph can contain
// cycles (e.g. parent.kid = child; child.dad = parent). Stringifying such a
// value (string concatenation, arrayToList, etc.) used to recurse until the
// native stack overflowed — a hard process abort, not a catchable CFML error.
// This bit WireBox boot: arrayToList() over a collection that transitively
// reached the cyclic injector graph crashed the VM. The fix adds a cycle guard
// to CfmlValue::as_string. Cross-engine note: Lucee throws "can't cast complex
// type to string" instead of producing a value — either way the operation must
// TERMINATE (the point of this test) rather than crash/hang. Reaching the
// assertion at all proves there was no infinite recursion.

parent = { label = "P" };
child  = { label = "C" };
parent.kid = child;
child.dad  = parent;          // cycle: parent <-> child

terminated = false;
try {
	junk = arrayToList( [ parent ] );   // RustCFML: bounded string; Lucee: throws
	terminated = true;
} catch ( any e ) {
	terminated = true;                  // a catchable error is also fine — it terminated
}
assertTrue("stringifying a self-referential struct terminates (no infinite recursion)", terminated);

// Direct self-reference
selfRef = { name = "loop" };
selfRef.me = selfRef;
terminated2 = false;
try {
	junk2 = "" & selfRef;
	terminated2 = true;
} catch ( any e ) {
	terminated2 = true;
}
assertTrue("stringifying a directly self-referential struct terminates", terminated2);

// Non-cyclic stringification is unaffected (works on both engines)
assert("arrayToList of simple values still works", arrayToList( [ 1, 2, 3 ] ), "1,2,3");

suiteEnd();
</cfscript>
