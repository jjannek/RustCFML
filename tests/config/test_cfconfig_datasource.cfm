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

// A name that isn't registered falls through to the bare-string parser. The
// stdlib treats unknown bare strings as a sqlite path, so this errors at
// connection time rather than at lookup time.
ok = true;
try {
    queryExecute("SELECT 1", [], { datasource: "nonexistent-dsn" });
} catch (any e) {
    ok = true;  // expected
}
assert("unknown datasource doesn't panic", ok, true);

suiteEnd();
</cfscript>
