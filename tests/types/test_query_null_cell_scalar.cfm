<cfscript>
suiteBegin("Types: a NULL query cell proxies as an empty string in scalar contexts");

// A query aggregate over zero matching rows yields one row with a NULL cell
// (e.g. SELECT MAX(x) ... WHERE id=0). Lucee/ACF read a NULL query cell as ""
// with full-null support off, so the bare-column proxy must behave as an empty
// string in scalar contexts. Without this, Wheels' model.maximum/minimum/sum
// over a non-matching where returned a non-simple, not-equal-to-"" value.
q = queryNew("viewsmaximum", "integer");
queryAddRow(q);                 // one row, NULL cell
col = q["viewsmaximum"];

assert("null-cell column EQ empty string", col EQ "", true);
assertTrue("isSimpleValue(null-cell column)", isSimpleValue(col));
assert("Len of null-cell column is 0", Len(col), 0);

// A populated column still proxies to its first row, unchanged.
q2 = queryNew("n", "integer");
queryAddRow(q2);
querySetCell(q2, "n", 7, 1);
col2 = q2["n"];
assert("populated column proxies first row", col2 EQ 7, true);
assertTrue("isSimpleValue(populated column)", isSimpleValue(col2));

// An empty query (zero rows) column proxy also reads as empty string.
q3 = queryNew("c", "varchar");
col3 = q3["c"];
assert("empty-query column EQ empty string", col3 EQ "", true);

suiteEnd();
</cfscript>
