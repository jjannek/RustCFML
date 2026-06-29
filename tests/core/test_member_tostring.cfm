<!---
  Regression: `toString()` called on a non-component value (struct, array, query,
  number) must return a string representation, NOT a silent Null. A struct/array
  member method that resolved to nothing used to return Null; combined with the
  null-delete assignment guard (PR #112), `x = obj.toString()` then DELETED the
  pre-existing local `x`, surfacing later as the misleading "Variable x is
  undefined". This was the single biggest Wheels TestBox blocker — TestBox's
  Assertion.getStringName() builds object descriptions via `obj.toString()` and
  crashed for ~110 specs. Passes identically on Lucee 7 (which returns the same
  shape of repr; we only assert non-empty + simple, not the exact format).
--->
<cfscript>
suiteBegin("Member toString() on non-component values");

s = { a = 1, b = 2 };
sv = s.toString();
assertTrue("struct.toString() is a simple value", isSimpleValue(sv));
assertTrue("struct.toString() is non-empty", len(sv) gt 0);

a = [ 1, 2, 3 ];
av = a.toString();
assertTrue("array.toString() is a simple value", isSimpleValue(av));
assertTrue("array.toString() is non-empty", len(av) gt 0);

n = 42;
assertTrue("number.toString() round-trips", n.toString() == "42");

// The exact failure shape: a pre-declared local assigned a struct's toString()
// inside a try must NOT be deleted by the null-delete guard.
function getStringName( obj ){
    var type = "[t]: ";
    var toStringValue = "";
    try {
        toStringValue = arguments.obj.toString();
    } catch ( any e ) {
        // do nothing
    }
    return type & toStringValue; // must not throw "toStringValue is undefined"
}
assertTrue("getStringName(struct) does not lose its local", len(getStringName(s)) gt 4);
assertTrue("getStringName(array) does not lose its local", len(getStringName(a)) gt 4);

// Content-deterministic struct.toString() (v0.362.0): Lucee backs a `{}` struct
// with a Java HashMap, so its toString() is content-deterministic — two structs
// with identical keys/values stringify identically regardless of build order.
// RustCFML's IndexMap is insertion-ordered, which broke any code hashing a
// stringified struct as an identity key (TestBox/MockBox normalizeArguments →
// $args matching → Preside AdHocTaskManagerService taskId cluster). We now emit
// struct keys in sorted order from .toString() so the hash matches both ways.
// Assert only the cross-engine-stable PROPERTY (order-independence), not the
// exact format: Lucee's toString() is `{ALPHA=1, ...}` HashMap-style, ours is
// `{alpha: 1, ...}` sorted — both are content-deterministic, which is all that
// matters for the hash identity. This file runs on Lucee too.
o1 = {}; o1.gamma = 3; o1.alpha = 1; o1.beta = 2;
o2 = {}; o2.beta = 2; o2.alpha = 1; o2.gamma = 3;
assert("struct.toString() is insertion-order-independent", o1.toString(), o2.toString());
// nested structs are deterministic recursively too
n1 = { outer = { z=1, a=2 } };
n2 = { outer = { a=2, z=1 } };
assert("nested struct.toString() is insertion-order-independent", n1.toString(), n2.toString());

suiteEnd();
</cfscript>
