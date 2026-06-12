<cfscript>
// Issue #90 Gap B: cfdbinfo — datasource metadata. Result shapes follow
// Lucee's DBInfo.java (verified against Lucee 7 + H2 empirically and the
// Wheels ORM's consumption: driver_name/database_productname dispatch,
// pragma-shaped columns, FKCOLUMN_NAME, COLUMN_DEFAULT_VALUE fallbacks).
//
// The sqlite-backed section runs on RustCFML and skips on engines without
// the bundled sqlite driver. MySQL / PostgreSQL / SQL Server sections are
// gated on env vars so default runs stay self-contained:
//   RUSTCFML_TEST_MYSQL_DS  e.g. mysql://root:pw@127.0.0.1:3306/testdb
//   RUSTCFML_TEST_PG_DS     e.g. postgresql://postgres:pw@127.0.0.1:5432/testdb
//   RUSTCFML_TEST_MSSQL_DS  e.g. mssql://sa:pw@127.0.0.1:1433/testdb
suiteBegin("cfdbinfo");

dbiDs = "sqlite://" & getTempDirectory() & "/rustcfml_dbi_" & createUUID() & ".sqlite";
dbiSkip = false;
try {
    queryExecute("CREATE TABLE roles (role_id INTEGER PRIMARY KEY AUTOINCREMENT, role_name VARCHAR(50) NOT NULL DEFAULT 'guest')", [], {datasource: dbiDs});
    queryExecute("CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT, username VARCHAR(40), role_id INTEGER REFERENCES roles(role_id))", [], {datasource: dbiDs});
    queryExecute("CREATE INDEX idx_users_name ON users(username)", [], {datasource: dbiDs});
} catch (any e) {
    dbiSkip = true;
    assertTrue("cfdbinfo sqlite checks skipped (no sqlite): " & e.message, true);
}

if (NOT dbiSkip) {
    // --- version: the adapter-dispatch contract Wheels depends on ---
    cfdbinfo(type="version", name="vinfo", datasource=dbiDs);
    assertTrue("version: is a query", isQuery(vinfo));
    assert("version: recordCount", vinfo.recordCount, 1);
    assertTrue("version: driver_name names SQLite", vinfo.driver_name CONTAINS "SQLite");
    assert("version: database_productname", vinfo.database_productname[1], "SQLite");
    assertTrue("version: database_version populated", len(vinfo.database_version[1]) GT 0);
    assertTrue("version: has jdbc columns", listFindNoCase(vinfo.columnList, "jdbc_major_version") GT 0);

    // --- columns: names, types, PK/FK enrichment, defaults ---
    cfdbinfo(type="columns", name="cols", table="users", datasource=dbiDs);
    assert("columns: names", valueList(cols.COLUMN_NAME), "id,username,role_id");
    assert("columns: type_name", cols.TYPE_NAME[2], "VARCHAR");
    assert("columns: column_size from decl", cols.COLUMN_SIZE[2], 40);
    assert("columns: is_primarykey", valueList(cols.IS_PRIMARYKEY), "YES,NO,NO");
    assert("columns: is_foreignkey", valueList(cols.IS_FOREIGNKEY), "NO,NO,YES");
    assert("columns: referenced pk table", cols.REFERENCED_PRIMARYKEY_TABLE[3], "roles");
    assert("columns: referenced pk column", cols.REFERENCED_PRIMARYKEY[3], "role_id");
    assertTrue("columns: columnList has REFERENCED_PRIMARYKEY",
        listFindNoCase(cols.columnList, "REFERENCED_PRIMARYKEY") GT 0);
    assert("columns: ordinal positions", valueList(cols.ORDINAL_POSITION), "1,2,3");

    cfdbinfo(type="columns", name="rcols", table="roles", datasource=dbiDs);
    assertTrue("columns: default value", rcols.COLUMN_DEFAULT_VALUE[2] CONTAINS "guest");
    assert("columns: is_nullable NO on NOT NULL", rcols.IS_NULLABLE[2], "NO");

    // pattern filters column names
    cfdbinfo(type="columns", name="pcols", table="users", pattern="user%", datasource=dbiDs);
    assert("columns: pattern filter", valueList(pcols.COLUMN_NAME), "username");

    // columns_minimal: no enrichment columns
    cfdbinfo(type="columns_minimal", name="mcols", table="users", datasource=dbiDs);
    assertTrue("columns_minimal: no IS_PRIMARYKEY column",
        listFindNoCase(mcols.columnList, "IS_PRIMARYKEY") EQ 0);
    assert("columns_minimal: still has rows", mcols.recordCount, 3);

    // --- tables ---
    cfdbinfo(type="tables", name="tbls", datasource=dbiDs);
    assert("tables: names", valueList(tbls.TABLE_NAME), "roles,users");
    assert("tables: type", tbls.TABLE_TYPE[1], "TABLE");
    cfdbinfo(type="tables", name="ftbls", filter="VIEW", datasource=dbiDs);
    assert("tables: filter=VIEW excludes tables", ftbls.recordCount, 0);
    cfdbinfo(type="tables", name="ptbls", pattern="ro%", datasource=dbiDs);
    assert("tables: pattern", valueList(ptbls.TABLE_NAME), "roles");

    // --- index: named index + synthetic PRIMARY row ---
    cfdbinfo(type="index", name="idx", table="users", datasource=dbiDs);
    assertTrue("index: named index present", listFindNoCase(valueList(idx.INDEX_NAME), "idx_users_name") GT 0);
    assertTrue("index: PRIMARY row present", listFindNoCase(valueList(idx.INDEX_NAME), "PRIMARY") GT 0);
    assertTrue("index: column_name present", listFindNoCase(valueList(idx.COLUMN_NAME), "username") GT 0);

    // --- foreignkeys: exported keys (FKs referencing the table) ---
    cfdbinfo(type="foreignkeys", name="fks", table="roles", datasource=dbiDs);
    assert("foreignkeys: FKCOLUMN_NAME", valueList(fks.FKCOLUMN_NAME), "role_id");
    assert("foreignkeys: FKTABLE_NAME", valueList(fks.FKTABLE_NAME), "users");
    assert("foreignkeys: PKTABLE_NAME", fks.PKTABLE_NAME[1], "roles");

    // --- dbnames ---
    cfdbinfo(type="dbnames", name="dbn", datasource=dbiDs);
    assertTrue("dbnames: main catalog", listFindNoCase(valueList(dbn.database_name), "main") GT 0);
    assert("dbnames: type column", dbn.type[1], "CATALOG");

    // --- procedures: SQLite has none — empty query, correct shape ---
    cfdbinfo(type="procedures", name="procs", datasource=dbiDs);
    assert("procedures: empty on sqlite", procs.recordCount, 0);
    assertTrue("procedures: has PROCEDURE_NAME column",
        listFindNoCase(procs.columnList, "PROCEDURE_NAME") GT 0);

    // --- terms: a struct, not a query ---
    cfdbinfo(type="terms", name="trm", datasource=dbiDs);
    assertTrue("terms: is a struct", isStruct(trm));
    assertTrue("terms: has catalog key", structKeyExists(trm, "catalog"));

    // --- missing table THROWS, catchably (Wheels' TableNotFound path) ---
    dbiCaught = false;
    dbiMsg = "";
    try {
        cfdbinfo(type="columns", name="nope", table="no_such_table", datasource=dbiDs);
    } catch (any e) {
        dbiCaught = true;
        dbiMsg = e.message;
    }
    assertTrue("missing table throws catchably", dbiCaught);
    assertTrue("missing table message names the table", dbiMsg CONTAINS "no_such_table");

    // --- empty result on a VALID table is NOT an error ---
    cfdbinfo(type="foreignkeys", name="nofks", table="users", datasource=dbiDs);
    assert("no exported keys on users is empty, not an error", nofks.recordCount, 0);

    // --- the exact Wheels $dbinfo shape: attributeCollection + dotted name
    //     into the calling function's local scope ---
    function wheelsDbinfo(required string ds) {
        local.args = {type: "version", datasource: arguments.ds, name: "local.rv"};
        cfdbinfo(attributeCollection = local.args);
        return structKeyExists(local, "rv") ? local.rv.driver_name : "MISSING";
    }
    assertTrue("wheels-shape: attributeCollection + local.rv",
        wheelsDbinfo(dbiDs) CONTAINS "SQLite");

    // --- tag form ---
    dbiTagOk = false;
}
</cfscript>
<cfif NOT dbiSkip>
    <cfdbinfo type="tables" name="tagTbls" datasource="#dbiDs#">
    <cfscript>
    assert("tag form: tables", valueList(tagTbls.TABLE_NAME), "roles,users");

    // --- validation errors are catchable ---
    dbiCaught = false;
    try { cfdbinfo(type="columns", name="x", datasource=dbiDs); } catch (any e) { dbiCaught = true; }
    assertTrue("missing table attribute throws catchably", dbiCaught);
    dbiCaught = false;
    try { cfdbinfo(type="frobnicate", name="x", datasource=dbiDs); } catch (any e) { dbiCaught = true; }
    assertTrue("invalid type throws catchably", dbiCaught);
    </cfscript>
</cfif>

<cfscript>
// ------------------------------------------------------------
// Live-server drivers (opt-in via env vars; see header)
// ------------------------------------------------------------
function dbinfoEnvDs(required string varName) {
    try {
        local.v = getEnvironmentVariable(arguments.varName, "");
        return local.v;
    } catch (any e) {
        return "";
    }
}

function dbinfoServerChecks(required string label, required string ds, required string expectDriver) {
    // Shared assertions for MySQL / PostgreSQL / SQL Server: version
    // dispatch, columns enrichment, foreignkeys, throw-on-missing-table.
    try { queryExecute("DROP TABLE dbi_users", [], {datasource: arguments.ds}); } catch (any e) {}
    try { queryExecute("DROP TABLE dbi_roles", [], {datasource: arguments.ds}); } catch (any e) {}
    local.idcol = arguments.label EQ "mssql" ? "INT IDENTITY" : (arguments.label EQ "pg" ? "SERIAL" : "INT AUTO_INCREMENT");
    queryExecute("CREATE TABLE dbi_roles (role_id #local.idcol# PRIMARY KEY, role_name VARCHAR(50))", [], {datasource: arguments.ds});
    queryExecute("CREATE TABLE dbi_users (id #local.idcol# PRIMARY KEY, username VARCHAR(40), role_id INT, CONSTRAINT fk_dbi_role FOREIGN KEY (role_id) REFERENCES dbi_roles(role_id))", [], {datasource: arguments.ds});

    cfdbinfo(type="version", name="local.vinfo", datasource=arguments.ds);
    assertTrue("#arguments.label#: driver_name dispatch", local.vinfo.driver_name CONTAINS arguments.expectDriver);

    cfdbinfo(type="columns", name="local.cols", table="dbi_users", datasource=arguments.ds);
    assert("#arguments.label#: column names", lCase(valueList(local.cols.COLUMN_NAME)), "id,username,role_id");
    assert("#arguments.label#: pk flags", valueList(local.cols.IS_PRIMARYKEY), "YES,NO,NO");
    assert("#arguments.label#: fk flags", valueList(local.cols.IS_FOREIGNKEY), "NO,NO,YES");
    assert("#arguments.label#: referenced table", lCase(local.cols.REFERENCED_PRIMARYKEY_TABLE[3]), "dbi_roles");

    cfdbinfo(type="foreignkeys", name="local.fks", table="dbi_roles", datasource=arguments.ds);
    assert("#arguments.label#: exported FKCOLUMN_NAME", lCase(valueList(local.fks.FKCOLUMN_NAME)), "role_id");

    cfdbinfo(type="tables", name="local.tbls", datasource=arguments.ds);
    assertTrue("#arguments.label#: tables lists dbi_users",
        listFindNoCase(valueList(local.tbls.TABLE_NAME), "dbi_users") GT 0);

    local.caught = false;
    try { cfdbinfo(type="columns", name="local.x", table="dbi_no_such", datasource=arguments.ds); }
    catch (any e) { local.caught = true; }
    assertTrue("#arguments.label#: missing table throws catchably", local.caught);

    queryExecute("DROP TABLE dbi_users", [], {datasource: arguments.ds});
    queryExecute("DROP TABLE dbi_roles", [], {datasource: arguments.ds});
}

mysqlDs = dbinfoEnvDs("RUSTCFML_TEST_MYSQL_DS");
if (len(mysqlDs)) {
    dbinfoServerChecks("mysql", mysqlDs, "MySQL");
} else {
    assertTrue("cfdbinfo MySQL checks skipped (RUSTCFML_TEST_MYSQL_DS not set)", true);
}

pgDs = dbinfoEnvDs("RUSTCFML_TEST_PG_DS");
if (len(pgDs)) {
    dbinfoServerChecks("pg", pgDs, "PostgreSQL");
} else {
    assertTrue("cfdbinfo PostgreSQL checks skipped (RUSTCFML_TEST_PG_DS not set)", true);
}

mssqlDs = dbinfoEnvDs("RUSTCFML_TEST_MSSQL_DS");
if (len(mssqlDs)) {
    dbinfoServerChecks("mssql", mssqlDs, "SQL Server");
} else {
    assertTrue("cfdbinfo SQL Server checks skipped (RUSTCFML_TEST_MSSQL_DS not set)", true);
}

suiteEnd();
</cfscript>
