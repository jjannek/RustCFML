<cfscript>
suiteBegin("struct.get() vs same-named component method (GH ##223)");

// A component with its own get() method reads from a plain struct via
// variables.pool.get(key). The struct member (java.util.Map get passthrough)
// must win over the component's method — otherwise it recurses to depth 256.
o = new StructGetStore();
o.setPool( { myKey: "123" } );
assert("component's get() reaches struct.get() without recursion", o.get("myKey"), "123");
assert("component's get() returns MISS for absent key", o.get("nope"), "MISS");

suiteEnd();
</cfscript>
