<cfscript>
suiteBegin("Core: typed return type on a top-level cfscript function");

function plainTop() { return {a: 1}; }
assertTrue("control: untyped top-level fn returns a struct", isStruct(plainTop()));

struct  function makeStruct() { return {a: 1}; }
array   function makeArray()  { return [1, 2, 3]; }
string  function makeString() { return "hi"; }
assertTrue("struct-typed top-level fn returns a struct", isStruct(makeStruct()));
assertTrue("array-typed top-level fn returns an array", isArray(makeArray()));
assert("string-typed top-level fn returns its value", makeString(), "hi");

suiteEnd();
</cfscript>
