<cfscript>
suiteBegin("Indexed query-cell assignment");

// `q[col][row] = v` and `q.col[row] = v` must mutate the query in place and be
// visible through every alias. RustCFML's SetIndex/SetProperty handlers had no
// Query/QueryColumn arms, so these assignments were silent no-ops (only
// querySetCell worked) — breaking Wheels afterFind callbacks that write back
// via `arguments.collection[key][row] = ...`.
q = queryNew("id,views", "integer,integer");
queryAddRow(q);
querySetCell(q, "id", 1, 1);
querySetCell(q, "views", 2, 1);

// Bracket form.
q["views"][1] = 102;
assert("bracket cell assign persists", q.views[1], 102);

// Dot form.
q.views[1] = 202;
assert("dot cell assign persists", q["views"][1], 202);

// A new column added then written cell-by-cell (the Wheels new-property case).
queryAddColumn(q, "something", []);
q["something"][1] = "hello world";
assert("new column cell assign persists", q.something[1], "hello world");

// Alias visibility: a second handle onto the same query sees the mutation.
alias = q;
q["views"][1] = 555;
assert("alias sees mutation", alias.views[1], 555);

suiteEnd();
</cfscript>
