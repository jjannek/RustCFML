<cfscript>
suiteBegin("Exceptions: multi-catch selects ONE clause by type");

// Before this fix the codegen concatenated every catch clause body and ran
// them all unconditionally — the declared catch type was ignored, so both
// `catch (SpecificType)` and `catch (any)` fired for a single throw, and a
// type that matched NO clause was still "caught" by the first one. CFML
// semantics: exactly the FIRST clause whose declared type matches runs; if
// none match, the exception propagates to an enclosing handler.

// --- exactly one clause runs, and it's the matching typed one ---
ran = [];
try { throw(type = "TypeA", message = "m"); }
catch (TypeA e) { arrayAppend(ran, "typed"); }
catch (any e)   { arrayAppend(ran, "any"); }
assert("only the matching typed clause runs", ran.toList(), "typed");

// --- a non-matching typed clause is skipped; `any` catches the rest ---
ran = [];
try { throw(type = "TypeB", message = "m"); }
catch (TypeA e) { arrayAppend(ran, "A"); }
catch (any e)   { arrayAppend(ran, "any"); }
assert("non-matching typed clause is skipped, any catches", ran.toList(), "any");

// --- first match wins among several typed clauses ---
ran = [];
try { throw(type = "TypeC", message = "m"); }
catch (TypeX e) { arrayAppend(ran, "X"); }
catch (TypeC e) { arrayAppend(ran, "C"); }
catch (any e)   { arrayAppend(ran, "any"); }
assert("first matching clause wins, the rest are skipped", ran.toList(), "C");

// --- no clause matches => propagates to the enclosing handler ---
where = "";
try {
    try { throw(type = "Inner", message = "deep"); }
    catch (DoesNotMatch e) { where = "inner"; }
} catch (Inner e) { where = "outer:" & e.message; }
assert("unmatched type propagates to the outer try", where, "outer:deep");

// --- dotted-type hierarchy: catching a parent catches its subtypes ---
hit = "";
try { throw(type = "App.Config.Invalid", message = "m"); }
catch (App.Config e) { hit = "parent"; }
catch (any e)        { hit = "any"; }
assert("catch parent type matches a dotted subtype", hit, "parent");

// --- prefix that is not a dotted boundary must NOT match ---
hit = "";
try { throw(type = "Application", message = "m"); }
catch (App e)  { hit = "App"; }
catch (any e)  { hit = "any"; }
assert("a bare prefix does not falsely match (App vs Application)", hit, "any");

// --- finally runs on the matched path ---
order = [];
try { throw(type = "Foo", message = "m"); }
catch (Foo e) { arrayAppend(order, "catch"); }
finally       { arrayAppend(order, "finally"); }
assert("finally runs after a matched catch", order.toList(), "catch,finally");

// --- finally runs on the NO-match path before the exception propagates ---
order = [];
try {
    try { throw(type = "Bar", message = "m"); }
    catch (Nope e) { arrayAppend(order, "inner-catch"); }
    finally        { arrayAppend(order, "inner-finally"); }
} catch (any e) { arrayAppend(order, "outer-catch"); }
assert("finally runs before propagating an unmatched throw", order.toList(), "inner-finally,outer-catch");

// --- finally runs on the normal (no-exception) path ---
order = [];
try { x = 1; }
catch (any e) { arrayAppend(order, "catch"); }
finally       { arrayAppend(order, "finally"); }
assert("finally runs on the normal completion path", order.toList(), "finally");

// --- assertThrows still sees an unmatched throw as thrown ---
assertThrows("an unmatched typed throw is not swallowed", function() {
    try { throw(type = "Solo", message = "boom"); }
    catch (Other e) { /* must not match */ }
});

suiteEnd();
</cfscript>
