<cfscript>
suiteBegin("Tags: cfquery double-quoted SQL identifier");
</cfscript>

<!---
    ============================================================
    Background
    ============================================================
    A <cfquery> body is SQL, and SQL allows double-quoted identifiers
    (e.g. SELECT pg_get_expr(...) AS "where"). Postgres uses them for reserved
    words and case-sensitive names. On Lucee/ACF the cfquery body is treated as
    SQL text with only #...# interpolated, so a double-quoted identifier is fine.

    RustCFML emits the cfquery body as a double-quoted CFScript string and
    escaped an embedded " as \" (backslash). CFScript strings do not treat \" as
    an escaped quote -- a double quote is escaped by DOUBLING it ("") -- so the
    embedded quote terminated the string literal early and the leftover text
    parsed as stray tokens. Any cfquery whose SQL contains a double-quoted
    identifier failed to PARSE.

    Parse-class gap (escapes try/catch, would abort the runner), so the cases
    live in runtime-instantiated fixtures. The control (single-quoted SQL string
    literal only) parses today and guards the fixture wiring; the gap fixture is
    expected to fail on current upstream until embedded quotes are doubled. The
    query is guarded by <cfif false> so it is compiled but never executed.

    Why it matters for Moopa: code/apps/hub/lib/schemaSync.cfc (getTableIndexes)
    selects COALESCE(pg_get_expr(...), '') AS "where".
    ============================================================
--->

<cfscript>
// Instantiate a fixture and run run(); returns "ok" when the fixture parsed and
// ran, or a diagnostic string when it did not (so a parse failure becomes a
// clean assertion mismatch rather than an aborted suite).
function loadRun(required string name) {
    try {
        var o = createObject("component", arguments.name);
        if (!isObject(o)) {
            return "NOT-A-COMPONENT";
        }
        return o.run();
    } catch (any e) {
        return "THREW: " & e.message;
    }
}

// --- control: single-quoted SQL string literal already parses ----------------

assert("control: cfquery with single-quoted SQL string parses",
    loadRun("CfqueryQuotedIdentControlFixture"), "ok");

// --- gap: double-quoted SQL identifier in the cfquery body -------------------

assert("cfquery with double-quoted SQL identifier parses",
    loadRun("CfqueryQuotedIdentFixture"), "ok");

suiteEnd();
</cfscript>
