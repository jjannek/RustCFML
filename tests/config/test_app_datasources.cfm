<cfscript>
suiteBegin("cfconfig — per-application datasources (this.datasources)");

// Per-application datasources defined in tests/Application.cfc via
// `this.datasources` must be resolved by queryExecute/cfquery for THIS
// application, ahead of the process-global cfconfig registry. This is the
// Lucee/BoxLang behaviour; previously RustCFML ignored this.datasources.
//
// RustCFML-only: these exercise RustCFML's in-memory sqlite datasources
// (rc_app_mem / rc_app_mem_str / rc_app_bad) declared in tests/Application.cfc,
// which don't exist on the Lucee test server. Skip the whole suite there.
if (isRustCFML()) {

// 1. Valid in-memory sqlite datasource (struct form) — a basic query works.
ok = false;
try {
    r = queryExecute("SELECT 1 AS n", [], { datasource: "rc_app_mem" });
    ok = (r.n[1] == 1);
} catch (any e) {
    ok = false;
}
assert("this.datasources struct form resolves (rc_app_mem)", ok, true);

// 2. Same via the bare connection-string form.
okStr = false;
try {
    r2 = queryExecute("SELECT 2 AS n", [], { datasource: "rc_app_mem_str" });
    okStr = (r2.n[1] == 2);
} catch (any e) {
    okStr = false;
}
assert("this.datasources string form resolves (rc_app_mem_str)", okStr, true);

// 3. DISCRIMINATOR: rc_app_bad is defined with a non-sqlite driver pointing at
//    an unreachable server. If this.datasources is honoured, the name resolves
//    to that (postgres) URL and the query MUST fail (connection refused, or
//    "driver not available" when the feature isn't compiled). If it were
//    ignored, the bare name would fall through to the sqlite catch-all and
//    "SELECT 1" would silently succeed — so a throw here proves real
//    per-application resolution, not an accidental sqlite pass.
assertThrows(
    "this.datasources is actually resolved (bad driver throws, no sqlite fallthrough)",
    function() {
        queryExecute("SELECT 1 AS n", [], { datasource: "rc_app_bad" });
    }
);

} // end if (isRustCFML())

suiteEnd();
</cfscript>
