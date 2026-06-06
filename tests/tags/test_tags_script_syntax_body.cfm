<cfscript>
suiteBegin("Tags: Script-syntax with body block");

// ============================================================
// Background
// ============================================================
// Modern CFML lets you invoke body-block tags from cfscript via the
// `cfXXX(args) { body }` function-call form. This is the dominant idiom
// in BoxLang, Lucee 5+, and Adobe ColdFusion 2018+. Frameworks like
// CFWheels/Wheels use it heavily in components — e.g. Global.cfc has
// `cfmail(attributeCollection = local.args) { ... }`.
//
// Both forms must produce equivalent behavior:
//
//   <cfsavecontent variable="x">body</cfsavecontent>
//   cfsavecontent(variable="x") { writeOutput("body"); }
//
// This file exercises the script-call-with-body form for the body-block
// tags that have non-side-effect-y semantics (no SMTP, no real DB
// connection, no filesystem mutation). The pure-tag form is already
// covered by sibling test files (test_tags_savecontent.cfm,
// test_tags_cfmail.cfm).
// ============================================================

// ------------------------------------------------------------
// cfsavecontent(...) { body }
// ------------------------------------------------------------
cfsavecontent(variable = "captured") {
    writeOutput("hello from script-syntax savecontent");
}
assert("cfsavecontent script: captures plain text",
    trim(captured), "hello from script-syntax savecontent");

cfsavecontent(variable = "captured2") {
    for (i = 1; i <= 3; i++) {
        writeOutput(i);
    }
}
assert("cfsavecontent script: captures loop output",
    trim(captured2), "123");

// Verify it doesn't leak into the surrounding output buffer.
cfsavecontent(variable = "captured3") {
    writeOutput("inside");
}
assertTrue("cfsavecontent script: returns a string",
    isSimpleValue(captured3));

// ------------------------------------------------------------
// cflock(...) { body }
// ------------------------------------------------------------
lockResult = "";
cflock(name = "rustcfml-script-lock-test", type = "exclusive", timeout = 10) {
    lockResult = "inside-lock";
}
assert("cflock script: body executes",
    lockResult, "inside-lock");

// Nested lock with different name should work.
nestedResult = "";
cflock(name = "rustcfml-outer", timeout = 10) {
    cflock(name = "rustcfml-inner", timeout = 10) {
        nestedResult = "nested-ok";
    }
}
assert("cflock script: nested locks",
    nestedResult, "nested-ok");

// ------------------------------------------------------------
// transaction action="..." { body }   (cfscript keyword form)
// ------------------------------------------------------------
// Cf2018+/Lucee/BoxLang let you write `transaction action="begin" { }`
// in cfscript without the `cf` prefix — same shape as `lock { }` and
// `savecontent { }`. This is the form Wheels' Migrator.cfc and Seeder.cfc
// use, plus all migrator templates (create-table.cfc etc.).
//
// Without a configured datasource, `commit` will throw — that's fine, we
// only care that the body parsed and executed before the implicit commit.
txResult = "";
try {
    transaction {
        txResult = "tx-body-ran";
    }
} catch (any e) {
    // Implicit commit can throw without a datasource. Body should have
    // run before that point.
}
assert("transaction keyword: body executes before commit",
    txResult, "tx-body-ran");

// ------------------------------------------------------------
// cftransaction(...) { body }   (script function-call form)
// ------------------------------------------------------------
ctxResult = "";
try {
    cftransaction() {
        ctxResult = "cftx-body-ran";
    }
} catch (any e) {
    // Same — implicit commit may throw, body still runs.
}
assert("cftransaction script: body executes before commit",
    ctxResult, "cftx-body-ran");

// ------------------------------------------------------------
// cfmail(...) { body }   (parses but doesn't send — caught)
// ------------------------------------------------------------
// We can't actually send mail without an SMTP server, but the file must
// at minimum *parse* and the function must at minimum *enter the body*.
// We rely on cfmail throwing without a configured mail server (matches
// the behavior verified by tests/tags/test_tags_cfmail.cfm for the tag
// form).
mailBodyRan = false;
mailThrew   = false;
try {
    cfmail(to = "nowhere@example.com",
           from = "noreply@example.com",
           subject = "script-form parse check",
           type = "text") {
        mailBodyRan = true;
        writeOutput("hi");
    }
} catch (any e) {
    mailThrew = true;
}
// The body either runs (engine has a mail spooler that accepts it) or
// the call throws (no server configured). Either is acceptable — what
// matters is that the file parsed and the call dispatched.
assertTrue("cfmail script: parsed and dispatched (body ran OR threw)",
    mailBodyRan OR mailThrew);

// ------------------------------------------------------------
// cfquery(...) { body }   (issue #68)
// ------------------------------------------------------------
// The script-call form must parse its `{ ... }` as a statement block — not a
// struct literal — so the body can build the SQL dynamically. This is the
// pattern Wheels' database adapter uses. Core assertion: a function wrapping a
// script cfquery (with statements + control flow in the body) PARSES, proven
// by the function being defined, regardless of whether a DB is reachable.
buildScriptQuery = function(required string ds, boolean onlyAlpha = true) {
    cfquery(name = "local.q", datasource = "#arguments.ds#") {
        writeOutput("SELECT name FROM cfqtest WHERE 1 = 1");
        if (arguments.onlyAlpha) { writeOutput(" AND name = 'alpha'"); }
        writeOutput(" ORDER BY id");
    }
    return local.q;
};
assertTrue("cfquery script: body parses as a statement block (function defined)",
    isCustomFunction(buildScriptQuery));

// Behavioral check when SQLite is available (skipped otherwise, matching
// tests/tags/test_tags_cfquery_control_tags.cfm).
cfqDs = "sqlite://" & getTempDirectory() & "/rustcfml_scriptq_" & createUUID() & ".sqlite";
cfqSkip = false;
try {
    queryExecute("CREATE TABLE cfqtest (id INTEGER PRIMARY KEY, name TEXT)", [], { datasource: cfqDs });
    queryExecute("INSERT INTO cfqtest (id, name) VALUES (1, 'alpha'), (2, 'beta')", [], { datasource: cfqDs });
} catch (any e) {
    cfqSkip = true;
    assertTrue("cfquery script behavioral checks skipped (no sqlite): " & e.message, true);
}
if (NOT cfqSkip) {
    assert("cfquery script: dynamic body filters rows (onlyAlpha)",
        valueList(buildScriptQuery(cfqDs, true).name), "alpha");
    assert("cfquery script: dynamic body unfiltered",
        valueList(buildScriptQuery(cfqDs, false).name), "alpha,beta");
}

suiteEnd();
</cfscript>
