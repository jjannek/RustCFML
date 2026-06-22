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

// 16) A nested dotted assignment must navigate an existing intermediate key
//     case-INSENSITIVELY, not fork a second physical key under a different
//     casing. Preside's system Config built `settings.assetManager = {…}`
//     (capital M, many keys); the site Config then wrote
//     `settings.assetmanager.storage.x = v` (lowercase). RustCFML forked a
//     separate `assetmanager` key holding only the lowercase-written subkeys,
//     and ColdBox's `structAppend(configStruct, settings, true)` later merged
//     both — last-writer (the 3-key fork) won, dropping `queue` etc. so
//     `getSetting("assetManager.queue.concurrency")` threw "does not exist".
forkS = {};
forkS.assetManager = { maxFileSize = "5", queue = { concurrency = 1 } };
forkS.assetmanager.storage.public = "/x";          // lowercase, 3-level deep write
forkS.assetmanager.derivativeLimits.maxHeight = 99; // lowercase, another branch
forkKeys = "";
for ( fk in forkS ) { forkKeys &= "[" & fk & "]"; }
assert("nested dotted write navigates case-insensitively — no key fork", forkKeys, "[assetManager]");
assert("case-fork: original keys preserved", structKeyExists( forkS.assetManager, "queue" ), true);
assert("case-fork: lowercase-written subkey landed in same struct", forkS.assetManager.storage.public, "/x");
assert("case-fork: second lowercase branch landed too", forkS.assetManager.derivativeLimits.maxHeight, 99);
assert("case-fork: queue.concurrency still reachable", forkS.assetManager.queue.concurrency, 1);

// 17) getMetadata()/getComponentMetadata() expose `path` = the absolute .cfc
//     file path (Lucee/ACF parity). Preside's PresideObjectReader reads
//     `meta.path` to re-parse a CFC's source (calling `.reReplace()` on it);
//     a missing key NPE'd ("cannot call method [reReplace] on a null value").
pathMeta = getMetadata( new PresideFixChainService() );
assert("getMetadata exposes path key", structKeyExists( pathMeta, "path" ), true);
assert("getMetadata path ends with the .cfc filename", listLast( replace( pathMeta.path, "\", "/", "all" ), "/" ), "PresideFixChainService.cfc");

// 18) directoryList( path=, recurse=, filter= ) — NAMED-argument form. Intercepted
//     BIFs are zero-param stubs, so named args fell through in call-site order and
//     `filter` landed in the `listInfo` slot → no filtering + subdirectories leaked
//     (Preside _getAllObjectPaths built an empty object filename from a leaked dir).
dlRoot = getTempDirectory() & "/rcfml_dl_" & createUUID();
directoryCreate( dlRoot & "/sub", true );
fileWrite( dlRoot & "/a.cfc", "" );
fileWrite( dlRoot & "/b.txt", "" );
fileWrite( dlRoot & "/sub/c.cfc", "" );
dlNamed = directoryList( path=dlRoot, recurse=true, filter="*.cfc" );
assert("directoryList named-arg filter+recurse returns only matching files", arrayLen( dlNamed ), 2);
dlHasDir = false;
for ( dlEntry in dlNamed ) { if ( !findNoCase( ".cfc", dlEntry ) ) { dlHasDir = true; } }
assert("directoryList named-arg excludes subdirectories", dlHasDir, false);
directoryDelete( dlRoot, true );

// 19) A bare identifier used as a statement (`j;`) is dead code: Lucee/ACF do NOT
//     throw even when the variable is undefined (Preside PresideObjectReader
//     ._setUseDrafts ships a stray `{j` typo that boots fine on Lucee).
bareIdentResult = _bareIdentStatement();
assert("bare undefined-identifier statement is a no-op (no throw)", bareIdentResult, "reached");

// 20) `array.delete( value )` member function deletes the element equal to `value`
//     and returns the modified array (Lucee parity). It was unmapped → returned
//     Null, and because `delete` is a mutating method the member-call write-back
//     stored that Null back into the variable, nulling the array — so a delete
//     loop (Preside _deletePropertiesMarkedForDeletion) hit a null receiver on
//     its 2nd iteration.
delArr = [ "x", "y", "z" ];
delArr.delete( "y" );
assert("array.delete(value) removes the element", arrayToList( delArr ), "x,z");
assert("array.delete(value) leaves the array non-null/usable", isArray( delArr ), true);
// Delete-in-loop must not null the array after the first removal.
loopArr = [ "a", "b", "c", "d" ];
toRemove = [ "b", "d" ];
for ( rm in toRemove ) { loopArr.delete( rm ); }
assert("array.delete in a loop survives (no null receiver)", arrayToList( loopArr ), "a,c");

// 21) UNQUOTED property attribute values (`required=false`, `ondelete=cascade`,
//     `maxlength=100`) must not terminate attribute parsing — every later
//     attribute (esp. a trailing `feature="…"`) must survive. This was the
//     Preside serve-mode boot blocker: `website_applied_permission.benefit`
//     declares `… required=false … feature="websiteBenefits"`, the unquoted
//     `required=false` dropped the trailing `feature`, so the feature-disabled
//     relationship was never removed → RelationshipGuidance threw "Object,
//     [website_benefit], could not be found" (Lucee reads `feature` and boots).
uqMeta = getComponentMetadata("oop.PresideFixUnquotedAttrs");
uqProps = {};
for ( p in uqMeta.properties ) { uqProps[ p.name ] = p; }
assert("unquoted attr does not drop trailing feature (benefit)", uqProps.benefit.feature ?: "MISSING", "websiteBenefits");
assert("unquoted attr preserves ondelete after required=false", uqProps.benefit.ondelete ?: "MISSING", "cascade");
assert("unquoted attr preserves uniqueindexes", uqProps.benefit.uniqueindexes ?: "MISSING", "context_permission|4");
assert("unquoted numeric attr does not drop trailing feature (qty)", uqProps.qty.feature ?: "MISSING", "someFeature");
assert("unquoted numeric attr value captured (maxlength)", uqProps.qty.maxlength ?: "MISSING", "100");

// ---------------------------------------------------------------------------
// Session 2026-06-22 boot fixes.

// A) Chained assignment where the MIDDLE target is a bracket/array access
//    (Preside SqlSchemaSynchronizer: `column = sql.columns[colName] = StructNew()`).
//    The leftmost target was left undefined because the SetIndex path didn't
//    leave the value for the outer store.
chSql = { columns = {} };
chCol = chSql.columns[ "foo" ] = StructNew();
assert("chained bracket-middle assign — left target is the struct", isStruct(chCol), true);
assert("chained bracket-middle assign — bracket target is the struct", isStruct(chSql.columns.foo), true);
chCol.definitionSql = "ddl";
assert("chained bracket-middle assign — left/bracket share reference", chSql.columns.foo.definitionSql ?: "MISSING", "ddl");

// B) String member-method list functions (NoCase + mutators) were missing from
//    the member dispatch map, so `list.listFindNoCase(x)` returned empty while
//    `listFindNoCase(list, x)` worked. Preside VersioningService guards a
//    dbFieldList append with `!objMeta.dbFieldList.listFindNoCase(field)`; the
//    broken member form duplicated _version columns in CREATE TABLE.
lst = "id,name,_version_is_draft,_version_has_drafts,datecreated";
assert("member listFindNoCase finds present value", lst.listFindNoCase("_version_is_draft"), 3);
assert("member listFindNoCase absent value is 0", lst.listFindNoCase("nope"), 0);
assert("member listFind agrees", lst.listFind("name"), 2);
assert("member listValueCountNoCase", lst.listValueCountNoCase("name"), 1);

// C) Positional argumentCollection forward: a paramless proxy forwarding
//    `argumentCollection=arguments` (arguments={1:val}) to a required named
//    param. (Preside _getAdapter() -> getAdapter(argumentCollection=arguments).)
assert("positional argumentCollection binds required named param", _argCollProxy("preside"), "got[preside]");

// D) pending_extra_named_args must not leak into a following positional call.
//    A named call to a paramless fn creates an overflow-named extra; the next
//    paramless POSITIONAL call must still see its arg keyed "1", not the stale
//    name. (Preside: runSql(dsn=…, sql=_getAdapter(dsn)…) renamed the inner
//    positional arg to "sql".)
_argNamedOverflow( foo = "x" );
assert("no named-extra leak into next positional call", _argPositionalKeys("v"), "1");

// E) Unscoped compound auto-viv inside a classic-localmode CFC method must
//    land in the shared component (variables) scope — so a super.method()
//    dispatched sibling can read it. (ColdBox cbi18n: parent `init` seeds
//    `instance.aLocale = createObject(...)`, then a child-overridden
//    `buildLocale` reaches back via `super` and must find it — it was being
//    forked into a phantom frame-local invisible to the super-dispatched frame,
//    so `instance.aLocale.getDefault()` threw a null-method error at boot.)
superScope = new PresideFixSuperScopeChild();
assert("super-dispatched sibling sees parent-init's auto-vivd instance", superScope.getSeed(), "SEEDED");

suiteEnd();

// Paramless proxy forwarding a positional arg via argumentCollection to a
// function declaring the param by name.
private string function _argCollReal( required string dsn ) { return "got[" & dsn & "]"; }
private string function _argCollProxy() { return _argCollReal( argumentCollection = arguments ); }
// Paramless fn called with a NAMED arg (overflow → arguments scope keyed by name).
private string function _argNamedOverflow() { return structKeyList( arguments ); }
// Paramless fn called positionally — its arguments scope must be keyed "1".
private string function _argPositionalKeys() { return structKeyList( arguments ); }

private string function _bareIdentStatement() {
	if ( true ) {j
		return "reached";
	}
	return "no";
}

private string function _testIncludeAttrForm() {
	var dir = "subinc";
	request._incProbe = "EMPTY";
	include template=dir & "/included.cfm";
	return request._incProbe;
}
</cfscript>
