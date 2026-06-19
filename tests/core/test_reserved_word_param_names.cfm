<cfscript>
// Issue #179 — CFML reserved words (in/do/for/eq/is) are legal function
// parameter names on Lucee/ACF/BoxLang. They must parse in both script and
// tag form. ColdBox's coldbox.system.core.util.Util.cfc declares
// `<cfargument name="in" type="array">`, so a parse failure here blocks the
// whole Preside/CacheBox boot chain.
suiteBegin("Reserved-word parameter names (Issue ##179)");

// Script form, untyped
function p_in( in )   { return arguments.in; }
function p_do( do )   { return arguments.do; }
function p_for( for ) { return arguments.for; }
function p_eq( eq )   { return arguments.eq; }
function p_is( is )   { return arguments.is; }

assert("param named 'in'",  p_in(1),  1);
assert("param named 'do'",  p_do(2),  2);
assert("param named 'for'", p_for(3), 3);
assert("param named 'eq'",  p_eq(4),  4);
assert("param named 'is'",  p_is(5),  5);

// Script form, typed (type annotation + reserved-word name)
function typed_in( array in ) { return arguments.in[1]; }
assert("typed param named 'in'", typed_in([99]), 99);

// ColdBox Util.cfc shape — reduce over arguments.in
function arrToStr( array in ) {
    return arguments.in.reduce(function(acc, v) { return acc & v; }, "");
}
assert("reduce over arguments.in", arrToStr(["a","b","c"]), "abc");

suiteEnd();
</cfscript>

<!--- Tag form: <cfargument name="in"> --->
<cffunction name="tagReserved" returntype="any">
    <cfargument name="in" type="array" required="true"/>
    <cfargument name="for" type="numeric" required="true"/>
    <cfreturn arguments.in[ arguments.for ]/>
</cffunction>
<cfscript>
suiteBegin("Reserved-word parameter names — tag form (Issue ##179)");
assert("tag-form params 'in'/'for'", tagReserved(["x","y","z"], 2), "y");
suiteEnd();
</cfscript>
