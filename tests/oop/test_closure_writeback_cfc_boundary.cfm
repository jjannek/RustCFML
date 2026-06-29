<cfscript>
suiteBegin("Closure unscoped-write writeback across a CFC-method boundary");

// Regression (v0.362.0): a closure that mutates an UNSCOPED enclosing variable
// must propagate that write back to its defining frame even when it is invoked
// behind one or more intermediate CFC-method frames — e.g. Preside's
// DynamicFindAndReplaceService passing a capture-groups processor down to
// `sut.dynamicFindAndReplace()` which then calls `arguments.processor()`.
//
// The closure's writeback reaches the frame's shared closure env (an Arc shared
// with the defining frame) via write_back_to_captured_scope, but the per-call
// `closure_parent_writeback` was consumed by the intermediate CFC-method frame
// and never bubbled to the defining frame, whose own reads come from `locals`.
// The fix reconciles the shared env back into `locals` after each call. Lucee
// (closures capture lexically by reference) propagates the write; we now match.
// Helper CFCs live in CwbSvc.cfc / CwbOuter.cfc (Lucee rejects named components
// declared inside a .cfm template).

// single CFC boundary
function singleBoundary() {
    var groups = "INITIAL";
    var processor = function( required string val ) { groups = arguments.val; return "ok"; };
    new CwbSvc().relay( processor );
    return groups;
}
assert("unscoped write propagates across one CFC-method frame", singleBoundary(), "CAPTURED");

// two CFC boundaries deep
function twoBoundaries() {
    var groups = "INITIAL";
    var p = function( v ) { groups = arguments.v; return "x"; };
    new CwbOuter().go( p );
    return groups;
}
assert("unscoped write propagates two CFC-method frames deep", twoBoundaries(), "CAPTURED");

// reference-typed enclosing var (array) mutated by a closure behind a CFC call
function refArrayThroughCfc() {
    var acc = [];
    new CwbSvc().relay( function( v ) { arrayAppend( acc, v ); return ""; } );
    return arrayLen( acc );
}
assert("array append from closure behind CFC boundary is visible", refArrayThroughCfc(), 1);

// no-regression: a direct closure write still works and locals don't leak
function directNoLeak() {
    var base = "keep";
    var f = function() { var localOnly = "x"; base = "changed"; return localOnly; };
    f();
    return base & "/" & ( isDefined("localOnly") ? "LEAK" : "ok" );
}
assert("direct closure write + no local leak", directNoLeak(), "changed/ok");

suiteEnd();
</cfscript>
