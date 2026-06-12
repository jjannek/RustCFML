<cfscript>
// Issue #90 Gap A: delivery of query mutation/SELECT metadata.
//  - queryExecute's `result` option populates the named variable (dotted
//    names like "local.wheels.result" included) — Wheels depends on this.
//  - cfquery's `result=` attribute does the same in tag AND script-block
//    form, including when passed via attributeCollection (the only shape
//    Wheels uses).
//  - cfquery's `name=` is only set when a resultset came back: an INSERT
//    leaves it untouched (Lucee semantics — previously RustCFML assigned
//    the metadata struct to `name` and dropped `result`).
//  - The metadata struct carries {recordCount, cached, sql, executionTime}
//    plus `generatedKey` for INSERTs only.
// Cross-engine: QoQ sections run on Lucee too; datasource sections are
// skip-guarded on engines without the bundled sqlite driver.
suiteBegin("cfquery/queryExecute result delivery");

// ------------------------------------------------------------
// QoQ (no datasource needed — cross-engine)
// ------------------------------------------------------------
src = queryNew("id,name", "integer,varchar", [[1, "alpha"], [2, "beta"]]);

qoq = queryExecute("SELECT name FROM src ORDER BY id", {}, {dbtype: "query", result: "qoqRes"});
assertTrue("QoQ result option populates variable", isDefined("qoqRes"));
assert("QoQ result.recordCount", qoqRes.recordCount, 2);
assertTrue("QoQ result.sql present", len(qoqRes.sql) GT 0);

// result option delivered into a function's local scope (Lucee parity)
function qoqResultLocal(required query src) {
    queryExecute("SELECT name FROM src", {}, {dbtype: "query", result: "local.lr"});
    return structKeyExists(local, "lr");
}
assertTrue("QoQ result option lands in function local scope", qoqResultLocal(src));

// dotted result path auto-vivifies intermediate structs
function qoqResultDotted(required query src) {
    queryExecute("SELECT name FROM src", {}, {dbtype: "query", result: "local.wheels.result"});
    return isDefined("local.wheels.result") AND local.wheels.result.recordCount EQ 2;
}
assertTrue("QoQ dotted result path (local.wheels.result)", qoqResultDotted(src));

// direct queryExecute must NOT honor a `name` option (Lucee parity)
queryExecute("SELECT name FROM src", {}, {dbtype: "query", name: "qoqNamed"});
assertFalse("direct queryExecute ignores name option", isDefined("qoqNamed"));
</cfscript>

<!--- tag form over QoQ: name receives the query, result the metadata --->
<cfquery name="qoqTagQ" result="qoqTagRes" dbtype="query">
    SELECT name FROM src WHERE id = 1
</cfquery>
<cfscript>
assertTrue("tag QoQ name is a query", isQuery(qoqTagQ));
assert("tag QoQ rows", qoqTagQ.recordCount, 1);
assertTrue("tag QoQ result delivered", isDefined("qoqTagRes"));
assert("tag QoQ result.recordCount", qoqTagRes.recordCount, 1);
</cfscript>

<cfscript>
// ------------------------------------------------------------
// Datasource-backed checks (skipped when sqlite isn't available)
// ------------------------------------------------------------
qrdDs = "sqlite://" & getTempDirectory() & "/rustcfml_qres_" & createUUID() & ".sqlite";
qrdSkip = false;
try {
    queryExecute("CREATE TABLE t (id INTEGER PRIMARY KEY AUTOINCREMENT, n TEXT)", [], {datasource: qrdDs});
} catch (any e) {
    qrdSkip = true;
    assertTrue("cfquery result delivery datasource checks skipped (no sqlite): " & e.message, true);
}

if (NOT qrdSkip) {
    // --- queryExecute result option: INSERT metadata shape ---
    queryExecute("INSERT INTO t (n) VALUES ('a')", [], {datasource: qrdDs, result: "insRes"});
    assertTrue("insert result delivered", isDefined("insRes"));
    assert("insert result.recordCount", insRes.recordCount, 1);
    assert("insert result.generatedKey", insRes.generatedKey, 1);
    assertTrue("insert result.sql is the statement", insRes.sql CONTAINS "INSERT INTO t");
    assertFalse("insert result.cached", insRes.cached);

    // --- incrementing generated keys, no staleness on UPDATE/DELETE ---
    queryExecute("INSERT INTO t (n) VALUES ('b')", [], {datasource: qrdDs, result: "insRes2"});
    assert("second insert generatedKey increments", insRes2.generatedKey, 2);
    queryExecute("UPDATE t SET n = 'z'", [], {datasource: qrdDs, result: "updRes"});
    assertFalse("update result has NO generatedKey", structKeyExists(updRes, "generatedKey"));
    assert("update result.recordCount", updRes.recordCount, 2);
    queryExecute("DELETE FROM t WHERE n = 'no-match'", [], {datasource: qrdDs, result: "delRes"});
    assertFalse("delete result has NO generatedKey", structKeyExists(delRes, "generatedKey"));
    assert("delete result.recordCount", delRes.recordCount, 0);

    // --- SELECT result shape ---
    queryExecute("SELECT * FROM t", [], {datasource: qrdDs, result: "selRes"});
    assert("select result.recordCount", selRes.recordCount, 2);
    assertTrue("select result.columnList present", listLen(selRes.columnList) EQ 2);

    // --- the exact Wheels shape: script-block cfquery, all attributes via
    //     attributeCollection, dotted name + result into local scope ---
    wheelsSelect = function(required string ds) {
        local.attrs = {datasource: arguments.ds, name: "local.query", result: "local.wheels.result"};
        cfquery(attributeCollection = local.attrs) {
            writeOutput("SELECT n FROM t ORDER BY id");
        }
        return {
            hasName: structKeyExists(local, "query"),
            isQ: isQuery(local.query),
            resSql: local.wheels.result.sql,
            rows: local.wheels.result.recordCount
        };
    };
    ws = wheelsSelect(qrdDs);
    assertTrue("wheels-shape: name delivered to local scope", ws.hasName);
    assertTrue("wheels-shape: name is a query", ws.isQ);
    assert("wheels-shape: result.recordCount", ws.rows, 2);
    assertTrue("wheels-shape: result.sql", ws.resSql CONTAINS "SELECT n FROM t");

    wheelsInsert = function(required string ds) {
        local.attrs = {datasource: arguments.ds, name: "local.query", result: "local.wheels.result"};
        cfquery(attributeCollection = local.attrs) {
            writeOutput("INSERT INTO t (n) VALUES ('w')");
        }
        return {
            nameDefined: structKeyExists(local, "query"),
            genKey: local.wheels.result.generatedKey
        };
    };
    wi = wheelsInsert(qrdDs);
    assertFalse("wheels-shape INSERT: name stays undefined", wi.nameDefined);
    assert("wheels-shape INSERT: result.generatedKey", wi.genKey, 3);

    // --- explicit attributes win over attributeCollection entries ---
    explicitWins = function(required string ds) {
        local.attrs = {datasource: arguments.ds, result: "local.collRes"};
        cfquery(attributeCollection = local.attrs, result = "local.explicitRes") {
            writeOutput("SELECT n FROM t");
        }
        return structKeyExists(local, "explicitRes") AND NOT structKeyExists(local, "collRes");
    };
    assertTrue("explicit result attr wins over attributeCollection", explicitWins(qrdDs));
}
</cfscript>

<cfif NOT qrdSkip>
    <!--- tag form: INSERT leaves name undefined, result receives metadata --->
    <cfquery name="tagIns" result="tagInsRes" datasource="#qrdDs#">
        INSERT INTO t (n) VALUES ('tagrow')
    </cfquery>
    <cfscript>
    assertFalse("tag INSERT: name stays undefined", isDefined("tagIns"));
    assertTrue("tag INSERT: result delivered", isDefined("tagInsRes"));
    assert("tag INSERT: result.generatedKey", tagInsRes.generatedKey, 4);
    </cfscript>

    <cfquery name="tagSel" result="tagSelRes" datasource="#qrdDs#">
        SELECT * FROM t ORDER BY id
    </cfquery>
    <cfscript>
    assertTrue("tag SELECT: name is a query", isQuery(tagSel));
    assert("tag SELECT: result.recordCount", tagSelRes.recordCount, 4);
    </cfscript>
</cfif>

<cfscript>
suiteEnd();
</cfscript>
