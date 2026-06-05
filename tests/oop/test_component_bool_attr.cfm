<cfscript>
suiteBegin("Bare boolean component header attribute");

// Regression: a bare boolean attribute in the component header (e.g.
// `component accessors="true" singleton {`, equivalent to singleton="true")
// must parse. Previously the bare `singleton` was left before the `{`, the
// body parse failed, and the component silently built as null. WireBox models
// commonly declare `component singleton {`.

o = new BoolAttrProbe();
assertTrue("component with bare boolean attr builds", isObject(o));
assert("method on it works", o.whoAmI(), "BoolAttrProbe");

// the bare attribute is exposed as a true-valued component annotation
md = getMetadata( o );
assertTrue("bare boolean attr present in metadata", structKeyExists(md, "singleton"));

suiteEnd();
</cfscript>
