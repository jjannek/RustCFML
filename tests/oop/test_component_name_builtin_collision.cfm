<cfscript>
suiteBegin("OOP: component NAME collides with a builtin function");

// ============================================================
// Background
// ============================================================
// A component file may be named the same as a built-in function, e.g.
// Mid.cfc / Left.cfc / Len.cfc collide with the BIFs mid()/left()/len().
// `new Mid()` and createObject("component","Mid") must instantiate the
// COMPONENT, not return the builtin function value.
//
// Regression: anonymous `component {}` files are registered in `globals`
// under the key "Anonymous"; the name-based lookup in
// resolve_component_template did a case-insensitive scan of globals BEFORE
// the "Anonymous" fallback, and that scan matched the builtin Function
// (registered earlier than the just-built component) — so `new Mid()`
// returned the `mid` BIF instead of the Mid instance. Fixed by restricting
// the name lookups to Struct values (a component is always a Struct).
// ============================================================

// --- the builtins themselves must still work as functions ---
assert("builtin mid() still callable",  mid("hello world", 1, 5), "hello");
assert("builtin left() still callable", left("hello", 2), "he");
assert("builtin len() still callable",  len("hello"), 5);

// --- `new X()` resolves the component, not the builtin ---
m = new Mid();
assertTrue("new Mid() is an object", isObject(m));
assert("new Mid().who()", m.who(), "Mid component");

l = new Len();
assertTrue("new Len() is an object", isObject(l));
assert("new Len().who()", l.who(), "Len component");

// --- createObject path resolves the component too ---
co = createObject("component", "Mid");
assertTrue("createObject('component','Mid') is an object", isObject(co));
assert("createObject Mid.who()", co.who(), "Mid component");

// --- init() ran on the instance ---
assert("init() ran on new Mid()", m.marker, "Mid-instance");

suiteEnd();
</cfscript>
