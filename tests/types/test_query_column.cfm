<cfscript>
// Tests Lucee@7-parity QueryColumn semantics. `q.colname` is NOT an array —
// it's a string proxy (stringifies to first row) with bracket indexing for
// row access. Passing it to array BIFs (arrayLen/arrayMap/arrayContains)
// throws "Can't cast String [...] to a value of type [Array]".
suiteBegin("Type: QueryColumn proxy");

q = queryNew("id,name");
queryAddRow(q);
querySetCell(q, "id", 1);
querySetCell(q, "name", "Alice");
queryAddRow(q);
querySetCell(q, "id", 2, 2);
querySetCell(q, "name", "Bob", 2);

// --- isArray / isStruct: both false (it's a string-ish proxy) ---
assertFalse("isArray(q.col) is false", isArray(q.name));
assertFalse("isStruct(q.col) is false", isStruct(q.name));

// --- Stringification: q.col reads as first row ---
assert("concat stringifies first row", q.name & "!", "Alice!");
assert("concat with prefix", "hi-" & q.name, "hi-Alice");

// --- len() returns string length of first row (NOT row count) ---
assert("len(q.col) is first-row strlen", len(q.name), 5);

// --- Bracket indexing returns row N (1-based) ---
assert("index [1] first row", q.name[1], "Alice");
assert("index [2] second row", q.name[2], "Bob");

// --- arrayLen rejects: QueryColumn is not an Array ---
assertThrows("arrayLen(q.col) throws", function() { arrayLen(q.name); });

// --- for-in iterates ONCE, yielding the first row (Lucee treats as string) ---
collected = "";
for (v in q.name) { collected = collected & v & ","; }
assert("for-in yields first row only", collected, "Alice,");

// --- valueList / quotedValueList DO iterate rows (canonical query use) ---
assert("valueList iterates rows", valueList(q.name), "Alice,Bob");
assert("quotedValueList iterates rows", quotedValueList(q.name), "'Alice','Bob'");

// --- QueryColumn scalar proxy should coerce to the first row value ---
qScalar = queryNew("times,iterations,name", "integer,integer,varchar", [
    { times = 3, iterations = 120000, name = "index.cfc" }
]);
assert("query column coerces to number for builtin argument", repeatString("x", qScalar.times), "xxx");
assert("query column scalar compares against first row", qScalar.name EQ "INDEX.CFC", true);
assert("query column scalar reversed compare uses first row", "index.cfc" NEQ qScalar.name, false);

// --- elvis: q.col stringifies to first row ---
val = q.name ?: "fallback";
assert("elvis yields proxy (stringified)", val & "", "Alice");

suiteEnd();
</cfscript>
