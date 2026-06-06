<cfscript>
// Query reference semantics — queries are by reference in Lucee AND BoxLang
// (RustCFML matched this in the reference-typed-Query change). Cross-engine:
// must pass on RustCFML and Lucee identically.
suiteBegin("Query Reference Semantics");

// Build a one-row query (no `var` at page scope; helper uses a function body).
function makeQ() {
    var q = queryNew("id,name", "integer,varchar");
    queryAddRow(q);
    querySetCell(q, "id", 1);
    querySetCell(q, "name", "Alice");
    return q;
}

// === 1. Assignment aliases: `q2 = q1` shares the SAME query ===
q1 = makeQ();
q2 = q1;
queryAddRow(q2);
querySetCell(q2, "id", 2, 2);
querySetCell(q2, "name", "Bob", 2);
assert("q2=q1 alias: q1 sees q2's appended row", q1.recordCount, 2);
assert("q2=q1 alias: both report same count", q2.recordCount, q1.recordCount);
assert("q2=q1 alias: q1 sees q2's cell write", queryGetRow(q1, 2).name, "Bob");

// === 2. duplicate() is independent (deep copy breaks the alias) ===
orig = makeQ();
dup = duplicate(orig);
queryAddRow(dup);
querySetCell(dup, "id", 99, 2);
querySetCell(dup, "name", "Zed", 2);
assert("duplicate() independent: original unchanged", orig.recordCount, 1);
assert("duplicate() independent: copy grew", dup.recordCount, 2);
assert("duplicate() independent: original row intact", queryGetRow(orig, 1).name, "Alice");

// === 3. Pass-by-reference: a function mutates the CALLER's query ===
function addCarol(q) {
    queryAddRow(q);
    querySetCell(q, "id", 3, q.recordCount);
    querySetCell(q, "name", "Carol", q.recordCount);
}
caller = makeQ();
addCarol(caller);
assert("pass-by-ref: caller's query grew", caller.recordCount, 2);
assert("pass-by-ref: caller sees appended row", queryGetRow(caller, 2).name, "Carol");

// === 4. Reassignment inside a function stays LOCAL (rebind != mutate) ===
function reassignLocal(q) {
    q = queryNew("id,name", "integer,varchar"); // rebind local — caller unaffected
    queryAddRow(q);
    querySetCell(q, "name", "ReplacedInside");
}
keep = makeQ();
reassignLocal(keep);
assert("reassign-in-fn stays local: count", keep.recordCount, 1);
assert("reassign-in-fn stays local: original data intact", queryGetRow(keep, 1).name, "Alice");

suiteEnd();
</cfscript>
