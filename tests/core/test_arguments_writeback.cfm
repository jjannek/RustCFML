<cfscript>
suiteBegin("Arguments Scope Pass-by-Reference Writeback");

// Test 1: modify struct param via arguments scope (arguments.param.prop = val)
function addViaArguments(required any obj) {
    arguments.obj.foo = "bar";
}
s1 = { name: "test" };
addViaArguments(s1);
assertTrue("arguments.obj.prop writeback", structKeyExists(s1, "foo"));
assert("arguments.obj.prop value", s1.foo, "bar");

// Test 2: modify struct param directly (param.prop = val)
function addDirect(required any obj) {
    obj.color = "blue";
}
s2 = { name: "test" };
addDirect(s2);
assertTrue("direct param.prop writeback", structKeyExists(s2, "color"));
assert("direct param.prop value", s2.color, "blue");

// Test 3: both patterns in same function
function addBoth(required any obj) {
    obj.directProp = "direct";
    arguments.obj.argsProp = "args";
}
s3 = {};
addBoth(s3);
assertTrue("both: direct prop exists", structKeyExists(s3, "directProp"));
assertTrue("both: args prop exists", structKeyExists(s3, "argsProp"));

// Test 4: mixin pattern — inject function ref into struct via arguments
function injectMethod(required any target) {
    arguments.target.greet = function(name) { return "Hello " & arguments.name; };
}
svc = {};
injectMethod(svc);
assertTrue("mixin: function injected", structKeyExists(svc, "greet"));
assert("mixin: function callable", svc.greet("World"), "Hello World");

// Test 5: structInsert via arguments scope
function addViaStructInsert(required any obj) {
    structInsert(arguments.obj, "inserted", "yes");
}
s5 = {};
addViaStructInsert(s5);
assertTrue("structInsert via arguments", structKeyExists(s5, "inserted"));

// Test 6: nested struct modification
function modifyNested(required any obj) {
    arguments.obj.child = { nested: true };
}
s6 = {};
modifyNested(s6);
assertTrue("nested struct added", structKeyExists(s6, "child"));
assertTrue("nested value preserved", s6.child.nested);

suiteEnd();

// ---------------------------------------------------------------------------
suiteBegin("Closure unscoped-write propagates when invoked via a non-CFC receiver");

// A closure that writes to an unscoped enclosing variable must propagate that
// write back to the defining frame when invoked indirectly — `arguments.fn()`,
// `someStruct.fn()`, `variables.fn()` — exactly as a bare `fn()` call does.
// This was lost because the member-call dispatch cleared closure_parent_writeback
// unconditionally. (Engine cause behind Preside DynamicFindAndReplaceService's
// capture-groups-to-processor spec.)

// Via arguments.fn()
function runViaArgs( required any processor ) {
    return arguments.processor( [ "a", "b", "c" ] );
}
function viaArgsTest() {
    var captured = "";
    runViaArgs( processor = function( v ){ captured = v; return "ok"; } );
    return captured;
}
caViaArgs = viaArgsTest();
assertTrue("arguments.fn() write is array", isArray(caViaArgs));
assert("arguments.fn() write value", arrayToList(caViaArgs), "a,b,c");

// Via a plain struct member
function viaStructTest() {
    var captured = "";
    var holder = { p: function( v ){ captured = v; return "ok"; } };
    holder.p( [ "x", "y" ] );
    return captured;
}
caViaStruct = viaStructTest();
assert("struct.fn() write value", isArray(caViaStruct) ? arrayToList(caViaStruct) : caViaStruct, "x,y");

// Direct call still works (control)
function directTest() {
    var captured = "";
    var p = function( v ){ captured = v; return "ok"; };
    p( [ "1", "2" ] );
    return captured;
}
caDirect = directTest();
assert("direct fn() write value (control)", isArray(caDirect) ? arrayToList(caDirect) : caDirect, "1,2");

suiteEnd();
</cfscript>
