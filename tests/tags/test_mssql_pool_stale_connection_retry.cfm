<cfscript>
// SQL Server pooled connections must recover when the server closes idle
// sessions behind the pool (Azure SQL idle eviction / failover / serverless
// scale-to-zero parity).
//
// Like the PostgreSQL path, RustCFML deliberately does not ping on every
// checkout (Lucee parity / remote-DB perf), so a pooled connection whose backend
// was closed while idle is handed out and the first real query fails at the
// connection level. The runtime must mark it broken, discard it, and retry the
// (side-effect-free) statement on a fresh connection.
//
// A single retry is NOT enough: when the server drops several idle sessions at
// once, the pool holds multiple dead connections and r2d2 establishes their
// replacements asynchronously, so an immediate retry can draw a SECOND stale
// connection. This test therefore grows the pool with concurrent checkouts,
// kills every resulting session at once, and asserts that follow-up queries all
// recover — which only holds with the bounded drain-and-retry loop.
//
// Live test, gated on RUSTCFML_TEST_MSSQL_DS. To run locally:
//   docker run --rm -d -p 1433:1433 -e ACCEPT_EULA=Y -e MSSQL_SA_PASSWORD=Str0ng!Passw0rd \
//     --platform linux/amd64 mcr.microsoft.com/mssql/server:2022-latest
//   RUSTCFML_TEST_MSSQL_DS=mssql://sa:Str0ng!Passw0rd@127.0.0.1:1433/master \
//     cargo run -- tests/runner.cfm

suiteBegin("SQL Server stale pooled connection retry (skipped without RUSTCFML_TEST_MSSQL_DS)");

mssqldsn = "";
try {
    mssqldsn = server.system.environment.RUSTCFML_TEST_MSSQL_DS ?: "";
} catch (any e) {
    mssqldsn = "";
}

mssqlskip = false;

if (mssqldsn == "") {
    mssqlskip = true;
    writeOutput("  (skipped - set RUSTCFML_TEST_MSSQL_DS to enable)" & chr(10));
} else {
    try {
        queryExecute("SELECT 1 AS one", [], { datasource: mssqldsn });
    } catch (any e) {
        mssqlskip = true;
        writeOutput("  (skipped - SQL Server not reachable: " & e.message & ")" & chr(10));
    }
}

if (NOT mssqlskip) {
    try {
        // A distinct datasource string forces a separate pool, so killing from it
        // does not terminate the connection issuing the KILLs.
        killerDsn = mssqldsn & (find("?", mssqldsn) ? "&" : "?") & "killerpool=1";

        // Grow the target pool to several connections via concurrent checkouts;
        // after the join they sit idle in the pool.
        for (t = 1; t <= 6; t++) {
            thread name="mssqlStaleRetry#t#" action="run" datasource="#mssqldsn#" {
                queryExecute("WAITFOR DELAY '00:00:01'; SELECT @@SPID AS pid", [], { datasource: attributes.datasource });
            }
        }
        thread action="join";

        // Serverless suspend: close every other user session at once. System
        // sessions reject KILL ("Only user processes can be killed") — ignore those.
        victims = queryExecute(
            "SELECT session_id AS sid FROM sys.dm_exec_sessions WHERE session_id > 50 AND session_id <> @@SPID",
            [], { datasource: killerDsn });
        for (i = 1; i <= victims.recordCount; i++) {
            try { queryExecute("KILL " & victims.sid[i], [], { datasource: killerDsn }); } catch (any e) {}
        }

        // Every pooled connection in the target pool is now dead. The runtime must
        // drain the dead connections and transparently recover each query.
        recovered = 0;
        for (i = 1; i <= 5; i++) {
            r = queryExecute("SELECT 100 + " & i & " AS n", [], { datasource: mssqldsn });
            if (r.n[1] == 100 + i) recovered++;
        }
        assert("all queries recover after every idle session is killed", recovered, 5);
    } catch (any e) {
        assertTrue("stale pooled connection retry failed: " & e.message, false);
    }
}

suiteEnd();
</cfscript>
