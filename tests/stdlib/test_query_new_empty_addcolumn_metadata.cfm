<cfscript>
suiteBegin("QueryNew empty / QueryAddColumn extend / GetMetadata(query)");

// QueryNew("") yields ZERO columns (Lucee), not one empty-named column.
q = queryNew("");
assert("QueryNew('') columnlist empty", q.columnList, "");
assert("QueryNew('') recordcount 0", q.recordCount, 0);

// QueryAddColumn with MORE values than current rows EXTENDS the query (Lucee).
queryAddColumn(q, "id", ["first", "second", "third"]);
assert("after add: columnlist", q.columnList, "ID");
assert("after add: recordcount extended to 3", q.recordCount, 3);
assert("after add: cell 1", q["id"][1], "first");
assert("after add: cell 3", q["id"][3], "third");

// Adding a SHORTER column Null-pads to existing row count.
queryAddColumn(q, "extra", ["x"]);
assert("short col recordcount unchanged", q.recordCount, 3);
assertTrue("short col row 2 is null", isNull(q["extra"][2]));

// GetMetadata(query) -> ordinal array of {name, typeName, isCaseSensitive}.
info = getMetadata(q);
assertTrue("getMetadata is array", isArray(info));
assert("getMetadata len", arrayLen(info), 2);
assert("getMetadata col1 name", info[1].name, "id");
assert("getMetadata col2 name", info[2].name, "extra");
assertTrue("getMetadata has typeName", structKeyExists(info[1], "typeName"));
assertFalse("getMetadata isCaseSensitive false", info[1].isCaseSensitive);

suiteEnd();
</cfscript>
