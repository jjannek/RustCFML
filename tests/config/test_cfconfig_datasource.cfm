<cfscript>
suiteBegin("cfconfig — datasource registry");

// queryExecute should resolve "testds" through the cfconfig registry to the
// sqlite :memory: URL registered at startup.
try {
    result = queryExecute("SELECT 1 AS n", [], { datasource: "testds" });
    assert("queryExecute via named datasource", result.n[1], 1);
} catch (any e) {
    assert("queryExecute via named datasource FAILED: " & e.message, true, false);
}

// server.cfconfig surfaces the same datasource entry that was registered.
assert("datasource visible in server.cfconfig",
       server.cfconfig.datasources.testds.driver, "sqlite");

suiteEnd();
</cfscript>
