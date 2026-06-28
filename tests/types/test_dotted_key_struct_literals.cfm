<cfscript>
suiteBegin("Dotted-key struct literals");

// Lucee/ACF: a struct literal with dotted-path keys builds a NESTED struct.
// `{ a.b = X }` is equivalent to `{ a = { b = X } }`. RustCFML previously
// evaluated the dotted key as an expression and silently dropped the entry
// (producing an empty struct) — this broke Preside's RelationshipGuidance
// fixtures (`{ obj_a.meta = {...}, obj_b.meta = {...} }`).

// 1. Single dotted key nests one level
one = { a.b = 1 };
assertTrue("single dotted key creates parent", structKeyExists(one, "a"));
assert("single dotted key value", one.a.b, 1);

// 2. The Preside fixture shape: distinct roots, each with a .meta child
objects = {
      obj_a.meta = { tableName = "pobj_obj_a" }
    , obj_b.meta = { tableName = "pobj_obj_b", properties = { obj_a = { relationship = "many-to-one", relatedTo = "obj_a" } } }
};
assert("two dotted roots present", structKeyList(objects), "obj_a,obj_b");
assert("nested meta survives", objects.obj_b.meta.tableName, "pobj_obj_b");
assert("deeply nested literal survives", objects.obj_b.meta.properties.obj_a.relatedTo, "obj_a");

// 3. Siblings sharing a prefix deep-merge into one branch
merged = { a.b = 1, a.c = 2, d = 3 };
assert("merged sibling b", merged.a.b, 1);
assert("merged sibling c", merged.a.c, 2);
assert("flat sibling d", merged.d, 3);
assert("merged root key count", structCount(merged), 2);

// 4. Three levels deep
deep = { x.y.z = "deep" };
assert("three-level nesting", deep.x.y.z, "deep");

// 5. Quoted keys stay LITERAL — they are not split on dots
quoted = { "a.b" = 1, c.d = 2 };
assertTrue("quoted dotted key is a literal key", structKeyExists(quoted, "a.b"));
assert("unquoted dotted key still nests alongside", quoted.c.d, 2);

// 6. Non-consecutive same-prefix keys still merge, root order preserved
interleaved = { a.b = 1, c = 2, a.d = 3 };
assert("interleaved b", interleaved.a.b, 1);
assert("interleaved d", interleaved.a.d, 3);
assert("interleaved c", interleaved.c, 2);
assert("interleaved root order", structKeyList(interleaved), "a,c");

// 7. Plain (non-dotted) struct literals are unaffected
plain = { foo = 1, bar = 2 };
assert("plain struct foo", plain.foo, 1);
assert("plain struct bar", plain.bar, 2);

// 8. Computed/bracketed keys still work (flat path), mixed with a dotted key
k = "dyn";
computed = { "#k#" = 99, plain.inner = 1 };
assert("computed key value", computed.dyn, 99);
assert("dotted key alongside computed", computed.plain.inner, 1);

// 9. Works inside a function body too (pure-stack build, no scope leakage)
result = (function() {
    var objs = { p.meta = { name = "x" }, q.meta = { name = "y" } };
    return objs.q.meta.name;
})();
assert("dotted struct literal inside function", result, "y");

suiteEnd();
</cfscript>
