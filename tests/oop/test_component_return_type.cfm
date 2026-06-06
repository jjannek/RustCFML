<cfscript>
suiteBegin("OOP: component return type");

// A function may declare a `component` return type (e.g. `public component
// function init()`). The return-type annotation describes the value the
// function returns; it must not affect whether the CFC itself can be resolved
// or instantiated. Lucee, Adobe CF, and BoxLang all instantiate such a CFC
// normally. (Wheels' Seeder.cfc declares `public component function init()`.)
o = createObject("component", "oop.ComponentReturnType");
assertTrue("CFC with a `component` return-type function is an object", isObject(o));
assert("its methods are callable", o.ping(), "pong");

suiteEnd();
</cfscript>
