<cfscript>
suiteBegin("Tags: invoke as a CFScript statement");

// ============================================================
// Background
// ============================================================
// CFML supports `invoke` as a CFScript STATEMENT — attributes on their own,
// terminated by `;`, optionally with an `invokeargument` block body:
//
//     invoke component="Svc" method="m" arg=1 returnvariable="r";
//     invoke component=obj method="m" { invokeargument name="a" value=1; }
//
// This is distinct from the function-CALL form `invoke(comp, "m", args)`, which
// already worked. RustCFML previously did not recognise the statement form at
// all (it parsed `invoke` as a bare identifier). It now compiles to the same
// __cfinvoke(...) as the <cfinvoke> tag.
//
// ENGINE NOTE: the cf-LESS `invoke` is the script statement form Lucee accepts,
// so these cross-engine tests use it. Adobe ColdFusion also accepts a
// cf-PREFIXED `cfinvoke` script statement; RustCFML accepts that spelling too
// (as an ACF-compatible superset), but Lucee rejects it, so it is NOT exercised
// here. Component paths use the `/tags` mapping from tests/Application.cfc, which
// resolves on both engines.
// ============================================================

target = createObject("component", "tags.CfInvokeStmtTarget");

// (1) component as an already-instantiated OBJECT instance + named attr args
invoke component=target method="add" a=2 b=3 returnvariable="sum";
assert("object-instance component, attribute args, returnVariable", sum, 5);

// (2) no returnVariable -> invoked for side effect, must still parse & run
invoke component=target method="answer";
assert("statement-form invoke without returnVariable parses & runs", true, true);

// (3) component by NAME (resolved via the /tags mapping)
invoke component="tags.CfInvokeStmtTarget" method="answer" returnvariable="ans";
assert("by-name component resolves and dispatches", ans, 42);

// (4) argumentcollection
collected = { name = "Bob" };
invoke component=target method="greet" argumentcollection=collected returnvariable="g";
assert("argumentcollection passes a struct of args", g, "hi Bob");

// (5) invokeargument block body
invoke component=target method="add" returnvariable="blockSum" {
	invokeargument name="a" value=10;
	invokeargument name="b" value=20;
}
assert("invokeargument block supplies the args", blockSum, 30);

// (6) returnVariable into the local scope from inside a function
function viaLocal() {
	invoke component=variables.target method="add" a=4 b=4 returnvariable="local.res";
	return local.res;
}
assert("returnVariable can target the local scope", viaLocal(), 8);

suiteEnd();
</cfscript>
