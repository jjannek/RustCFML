<cfscript>
suiteBegin("Struct Functions");

// --- structNew ---
s = structNew();
assertTrue("structNew creates struct", isStruct(s));
assert("structNew is empty", structCount(s), 0);

// --- structCount ---
assert("structCount", structCount({a: 1, b: 2}), 2);

// --- structKeyExists ---
assertTrue("structKeyExists found", structKeyExists({a: 1}, "a"));
assertFalse("structKeyExists not found", structKeyExists({a: 1}, "z"));

// --- structKeyList ---
keyList = structKeyList({a: 1, b: 2});
assertTrue("structKeyList contains A", listFindNoCase(keyList, "A") > 0);
assertTrue("structKeyList contains B", listFindNoCase(keyList, "B") > 0);

// --- structKeyArray ---
keys = structKeyArray({a: 1, b: 2});
assert("structKeyArray length", arrayLen(keys), 2);

// --- structDelete ---
del = {a: 1, b: 2};
structDelete(del, "a");
assertFalse("structDelete removes key", structKeyExists(del, "a"));
assert("structDelete count", structCount(del), 1);

// --- structInsert ---
ins = {};
structInsert(ins, "x", 99);
assert("structInsert", ins.x, 99);

// --- structUpdate ---
upd = {a: 1};
structUpdate(upd, "a", 42);
assert("structUpdate", upd.a, 42);

// --- structFind ---
assert("structFind", structFind({a: 1, b: 2}, "a"), 1);

// --- structClear ---
clr = {a: 1, b: 2};
structClear(clr);
assert("structClear empties", structCount(clr), 0);

// --- structCopy ---
original = {a: 1, b: 2};
copied = structCopy(original);
assert("structCopy count", structCount(copied), 2);
assert("structCopy value", copied.a, 1);

// --- structAppend ---
base = {a: 1};
extra = {b: 2, c: 3};
structAppend(base, extra);
assert("structAppend count", structCount(base), 3);
assert("structAppend merged value", base.b, 2);

// --- structIsEmpty ---
assertTrue("structIsEmpty on empty", structIsEmpty({}));
assertFalse("structIsEmpty on non-empty", structIsEmpty({a: 1}));

// --- structSort ---
sortStruct = {b: 2, a: 1, c: 3};
sorted = structSort(sortStruct, "text");
assert("structSort first key", sorted[1], "A");

// --- structValueArray ---
vals = structValueArray({a: 1});
assert("structValueArray length", arrayLen(vals), 1);
assert("structValueArray value", vals[1], 1);

// --- isEmpty on structs ---
assertTrue("isEmpty empty struct", isEmpty({}));
assertFalse("isEmpty non-empty struct", isEmpty({a: 1}));

// --- struct introspection BIFs treat a query's columns as its keys (issue
// #146; Lucee parity). cfdbinfo type="version" returns a query and the Wheels
// migrator reads it with structKeyExists/structKeyList. ---
qIntro = queryNew("aa,bb,cc", "varchar,integer,varchar", [["x", 1, "p"], ["y", 2, "q"]]);
assertTrue("structKeyExists on query column", structKeyExists(qIntro, "bb"));
assertTrue("structKeyExists on query column case-insensitive", structKeyExists(qIntro, "BB"));
assertFalse("structKeyExists false for absent query column", structKeyExists(qIntro, "zz"));
assertTrue("structKeyList lists query columns", listFindNoCase(structKeyList(qIntro), "aa") > 0);
assert("structCount equals query column count", structCount(qIntro), 3);
assert("structKeyArray length equals query column count", arrayLen(structKeyArray(qIntro)), 3);
assertFalse("structIsEmpty false for query with columns", structIsEmpty(qIntro));
// columns are keys regardless of row count — a zero-row query is not empty
qZero = queryNew("mm,nn", "varchar,varchar");
assert("structCount counts columns of zero-row query", structCount(qZero), 2);
assertFalse("structIsEmpty false for zero-row query with columns", structIsEmpty(qZero));

suiteEnd();
</cfscript>
