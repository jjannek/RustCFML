<cfscript>
suiteBegin("Preside serve-mode boot fixes");

// 1) Chained assignment leaves its value, so all targets are set.
//    (Preside Config.cfc: settings.x = application.x = expr.)
x = y = 5;
assert("chained scalar assign — left target", x, 5);
assert("chained scalar assign — right target", y, 5);

s = {}; t = {};
s.a = t.a = 9;
assert("chained member assign — left target", s.a, 9);
assert("chained member assign — right target", t.a, 9);

// Chained assign where the MIDDLE target is a reserved SCOPE (Preside
// Config.cfc: settings.appMappingPath = application.appMappingPath = expr).
cfg = {};
cfg.appMappingPath = request._chainProbe = "app.path";
assert("chained assign — scope middle target sets left", cfg.appMappingPath, "app.path");
assert("chained assign — scope middle target sets scope", request._chainProbe, "app.path");

// 2) `throw object=expr;` single bare-attribute script form preserves the
//    thrown struct's message/type (Preside Bootstrap.onError).
exc = { message = "boom", type = "MyType", detail = "d" };
caught = "";
caughtType = "";
try {
	throw object=exc;
} catch (any e) {
	caught = e.message;
	caughtType = e.type;
}
assert("throw object= preserves message", caught, "boom");
assert("throw object= preserves type", caughtType, "MyType");

// 3) A method named after a reserved scope word (`local`) is reachable.
lm = new PresideFixLocalMethod();
assert("method named 'local' is invocable directly", lm.local(), "local-ran");
assert("invoke() can call method named 'local'", invoke(lm, "local"), "local-ran");
assert("sibling normal method still works", lm.normal(), "normal-ran");

// 4) An unset property is NOT a key in the variables scope (Lucee parity),
//    so getMemento's variables.filter(closure) doesn't crash on a null value.
p = new PresideFixProps();
p.setFoo("hello");
assertFalse("unset property absent from variables scope", p.hasBarKey());
mem = p.getMemento();
assert("getMemento keeps the set property", mem.foo, "hello");
assertFalse("getMemento omits the unset property", structKeyExists(mem, "bar"));

// 5) argumentCollection with numeric (positional) keys binds the param LOCALS,
//    not just arguments-scope keys (ColdBox paramless child -> super.init).
ctrl = new PresideFixArgCollChild("/some/path", "cbController");
assert("argumentCollection numeric keys bind param locals", ctrl.getAppRootPath(), "/some/path/");

// 6) A bound method invoked via a non-component receiver (arguments.fn(x))
//    runs with its defining component's variables.
b = new PresideFixBound();
assert("bound method via arguments keeps definer scope", b.run(), "svc=bound-svc arg=x");

// 7) A mixin (another component's method injected and invoked via the host)
//    runs with the HOST's variables.
mt = new PresideFixMixTarget();
assert("mixin invoked via host uses host scope", mt.run(), "TARGET-CACHEBOX");

suiteEnd();
</cfscript>
