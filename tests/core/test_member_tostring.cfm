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

suiteEnd();
</cfscript>
