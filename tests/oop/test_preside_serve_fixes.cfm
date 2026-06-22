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

// 8) A component method whose name collides with a BIF (Preside cfflow's
//    `evaluate( wfInstance, args )`) must NOT shadow the BIF for bare calls.
evalShadow = new PresideFixEvalShadow();
assert("component method named evaluate dispatches via object", evalShadow.evaluate(wfInstance="x", args={}), true);
assert("bare Evaluate() still hits the BIF after a shadowing method loads", Evaluate("6 * 7"), 42);
assert("bare Evaluate() inside the shadowing component hits the BIF", evalShadow.callBif("3 + 4"), 7);

// 9) Computed-name method call `obj[ name ]( args )` must dispatch against the
//    receiver's component scope (Preside DelayedInjector.onMissingMethod
//    forwards `instance[ missingMethodName ]( argumentCollection=... )`).
dynTarget = new PresideFixDynTarget();
assert("direct method reads component state", dynTarget.readState(), "DYN-STATE");
dynProxy = new PresideFixDynProxy( dynTarget );
assert("computed-name forward keeps target scope", dynProxy.readState(), "DYN-STATE");

// 10) for-in over a component yields PUBLIC data + PUBLIC methods, never
//     private methods or engine internals (Lucee `this`-scope iteration —
//     WireBox virtual inheritance copies a base class's public methods this
//     way in `toVirtualInheritance`).
forinState = new PresideFixForInState();
forinKeys = [];
for ( k in forinState ) { arrayAppend( forinKeys, k ); }
forinKeys.sort( "textnocase" );
assert("for-in over CFC yields public data + public methods", arrayToList( forinKeys ), "configure,dataKey,greet");
assert("for-in over CFC hides private methods", arrayFindNoCase( forinKeys, "secret" ), 0);
assert("for-in over CFC hides engine internals", arrayFindNoCase( forinKeys, "__variables" ), 0);

// 11) Script `include template=<expr>` attribute form evaluates the whole
//     expression (Preside Router.cfc: `include template=ext.dir & "/x.cfm"`).
//     Without the fix `template=expr` parses as a variable assignment and the
//     include path comes out empty.
assert("include template=expr attribute form resolves the path", _testIncludeAttrForm(), "INC-OK");

// 12) A java.util.LinkedHashMap shim is a transparent map — its `__java_*`
//     markers never surface in struct key enumeration (ColdBox ModuleService
//     iterates `structKeyArray( moduleRegistry )` over exactly such a map).
lhm = createObject( "java", "java.util.LinkedHashMap" ).init();
lhm[ "alpha" ] = { x = 1 };
lhm[ "beta" ]  = { x = 2 };
assert("java map structKeyArray hides __ markers", arrayToList( structKeyArray( lhm ).sort( "textnocase" ) ), "alpha,beta");
assert("java map structCount excludes __ markers", structCount( lhm ), 2);
lhmForIn = [];
for ( mk in lhm ) { arrayAppend( lhmForIn, mk ); }
assert("java map for-in hides __ markers", arrayToList( lhmForIn.sort( "textnocase" ) ), "alpha,beta");

// 13) Chained member call on a `variables.X` struct receiver, where the inner
//     method looks up an element (`.find( key )`) and the outer method runs on
//     that element. The outer (non-mutating) call must NOT write its `this`
//     snapshot back onto the inner receiver's path — doing so replaced
//     `variables.interceptionStates` with the looked-up InterceptorState on the
//     2nd call (ColdBox InterceptorService.processState: state has no `find`).
chainSvc = new PresideFixChainService();
assert("chained .find().process() — 1st call", chainSvc.processState( "s1" ), "processed:EVT");
assert("chained .find().process() — 2nd call (receiver not clobbered)", chainSvc.processState( "s2" ), "processed:EVT");
assert("chained .find().process() — receiver still a struct", chainSvc.statesIsStruct(), true);

// 14) A component source file with classic-Mac (CR-only) line endings parses:
//     `//` line comments must terminate at a bare CR, not run to EOF and
//     swallow the closing braces (ColdBox EventHandler.cfc ships CR-only).
crComp = new PresideFixCrEndings();
assert("CR-only line endings parse + // comments terminate at CR", crComp.greet(), "cr-ok");

// 15) A leading UTF-8 BOM is stripped, not emitted as literal page output
//     (Preside/ColdBox files ship with a BOM).
savecontent variable="bomOut" { include "preside_fix_bom_include.cfm"; }
assert("leading UTF-8 BOM is stripped from page output", bomOut, "BODY");

suiteEnd();

private string function _testIncludeAttrForm() {
	var dir = "subinc";
	request._incProbe = "EMPTY";
	include template=dir & "/included.cfm";
	return request._incProbe;
}
</cfscript>
