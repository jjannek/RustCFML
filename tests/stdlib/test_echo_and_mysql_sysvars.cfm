<cfscript>
suiteBegin("echo() + MySQL @@ system vars on SQLite");

// ---- echo() — Lucee/ACF alias of writeOutput (GH #200) ----
// echo writes to the page buffer; capture it with cfsavecontent.
savecontent variable="captured" { echo("hello "); echo("world"); }
assert("echo writes to page buffer", trim(captured), "hello world");

// ---- MySQL/MariaDB @@ system variables on the SQLite backend (GH #199) ----
// SQLite has no @@-prefixed session variables; RustCFML emulates the common
// capability-detection probes so a FROM-less SELECT doesn't crash with
// `unrecognized token: "@"`. Real MySQL datasources route to the mysql driver.
if (isRustCFML()) {
    // @@sql_mode is emulated as empty → ONLY_FULL_GROUP_BY detection reports
    // the permissive mode (exactly Preside's MySqlAdapter probe).
    r1 = queryExecute("select @@sql_mode as sqlmode");
    assert("@@sql_mode emulated empty", r1.sqlmode, "");
    assertFalse("ONLY_FULL_GROUP_BY absent", listFindNoCase(r1.sqlmode, "ONLY_FULL_GROUP_BY") > 0);

    // @@version returns the SQLite version (non-empty).
    r2 = queryExecute("select @@version as v");
    assertTrue("@@version non-empty", len(r2.v) > 0);

    // A plain constant select still works (the rewrite is a no-op without @@).
    r3 = queryExecute("select 1 as one");
    assert("constant select unaffected", r3.one, 1);
}

suiteEnd();
</cfscript>
