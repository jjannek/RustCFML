<cfscript>
// GH #224: a bare queryExecute (no datasource arg) must resolve to
// this.datasource — the SAME datasource the transaction path uses — instead of
// silently falling through to the in-memory SQLite default.

// 1) Bare writes OUTSIDE a transaction. Pre-fix these hit :memory:.
queryExecute("CREATE TABLE t224 (id integer)");
queryExecute("INSERT INTO t224 (id) VALUES (42)");

// 2) Read back with an EXPLICIT datasource pointing at appds. The row is only
//    here if the bare writes above resolved to appds too.
r = queryExecute("SELECT id FROM t224", [], { datasource: "appds" });
writeOutput("BARE_WRITE_VISIBLE_TO_EXPLICIT:" & (r.recordCount == 1 && r.id[1] == 42) & chr(10));

// 3) Write via a transaction (this.datasource), read via a bare query. Both
//    must resolve to the same datasource — the inside/outside consistency the
//    issue is about.
transaction {
    queryExecute("INSERT INTO t224 (id) VALUES (99)");
}
r2 = queryExecute("SELECT id FROM t224 ORDER BY id");
writeOutput("BARE_READ_AFTER_TXN_SEES_BOTH:" & (r2.recordCount == 2) & chr(10));

// 4) Repeated top-level transactions must not leak savepoint depth (the
//    cftxn_spN cascade). A dozen sequential commits should all succeed.
ok = true;
for (i = 1; i <= 12; i++) {
    try {
        transaction {
            queryExecute("INSERT INTO t224 (id) VALUES (:v)", { v: 1000 + i });
        }
    } catch (any e) {
        ok = false;
    }
}
writeOutput("SEQUENTIAL_TXNS_OK:" & ok & chr(10));
</cfscript>
