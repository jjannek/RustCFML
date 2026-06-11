<cfscript>
suiteBegin("component header: bare boolean attribute (singleton, etc.)");

// Regression: `component singleton {` with NO other attributes was parsed by
// RustCFML as `component name="singleton"` (the identifier was consumed as
// the component name when not followed by `=`/`extends`/`implements`). The
// body's static `this.X = Y` and function declarations were dropped silently.
// Fix: only consume an identifier as the component name when followed by
// `extends` or `implements`; otherwise treat it as a bare-bool attribute.

eb = new ProbeBareSingleton();
assertTrue("static `this.X = Y` after `component singleton {` executes", eb.MARKER EQ "alive");

md = getMetadata(eb);
fnNames = md.functions.map(function(f){ return arguments.f.name; });
assertTrue("function declared in bare-singleton body is registered", arrayFindNoCase(fnNames, "ping") gt 0);
assert("function in bare-singleton body invokes", eb.ping(), "pong");

// Combined: bare bool BEFORE an `attr=val` pair (already worked v0.54.0 onward,
// but assert it still does — `component singleton accessors="true" {`).
ex = new ProbeBareSingletonExtended();
exFns = getMetadata(ex).functions.map(function(f){ return arguments.f.name; });
assertTrue("bare-bool + accessors=true still works", arrayFindNoCase(exFns, "bark") gt 0);
assert("function invokes", ex.bark(), "woof");

suiteEnd();
</cfscript>
