<cfscript>
suiteBegin("Function annotation metadata (getMetadata on method ref)");

obj = createObject("component", "oop.FunctionAnnotationFixture");

// GitHub #208: getMetadata() on a method REFERENCE must surface the function's
// doc-comment annotations (@expectedException, @skip, @labels, @order) as flat
// top-level keys — the same values getComponentMetadata().functions[n] exposes.
// TestBox's xUnit runner reads these off `getMetadata( target[ methodName ] )`.
md = getMetadata( obj.annotated );

assertTrue("metadata is struct", isStruct(md));
assert("name present", md.name, "annotated");
assertTrue("expectedException present", structKeyExists(md, "expectedException"));
assert("expectedException value", md.expectedException, "InvalidException");
assert("skip value", md.skip, "false");
assert("labels value", md.labels, "foo,bar");
assert("order value", md.order, "3");

// Parameters still present alongside annotations.
assert("two parameters", arrayLen(md.parameters), 2);
assert("first param name", md.parameters[1].name, "a");
assertTrue("first param required", md.parameters[1].required);

// A method with no annotations: keys absent, but the call still works.
mdPlain = getMetadata( obj.plain );
assert("plain name", mdPlain.name, "plain");
assertFalse("plain has no expectedException", structKeyExists(mdPlain, "expectedException"));

// Consistency with getComponentMetadata(): the same annotation must appear via
// both paths (the inconsistency reported in #208).
cmd = getComponentMetadata( obj );
fnMeta = "";
for (fn in cmd.functions) {
	if (fn.name == "annotated") { fnMeta = fn; break; }
}
assertTrue("found annotated in component metadata", isStruct(fnMeta));
assert("component metadata agrees on expectedException", fnMeta.expectedException, md.expectedException);

suiteEnd();
</cfscript>
