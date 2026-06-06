<cfscript>
suiteBegin("Tags: cfexecute attribute interpolation");
</cfscript>

<!---
    ============================================================
    Background
    ============================================================
    A quoted <cfexecute> attribute value may carry #...# interpolation and may
    contain embedded quotes. The cfexecute transpiler emitted `timeout` and
    `arguments` verbatim: timeout="#x#" produced "timeout: #x#" (literal hashes
    in CFScript) and an embedded quote was escaped with a backslash
    ("say \"hi\""), which CFML does not honor (a quote is escaped by doubling).
    Both made the template fail to PARSE. Literal hash-free, quote-free values
    happened to work.

    Because that is a hard parse error (escapes try/catch, would abort the
    runner), the cases live in runtime-instantiated fixture CFCs. The control
    (literal timeout/arguments) parses today and guards the fixture wiring; the
    interpolated and embedded-quote fixtures pin the fix. Each cfexecute call is
    behind <cfif false> so it is compiled but never executed (no process spawn).

    Sibling of test_cfhttp_timeout_interpolation.cfm; same class of bug surfaced
    while reviewing the cfhttp timeout fix.
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

// --- control: literal timeout/arguments already parse (regression guard) -----

assert("control: cfexecute with literal timeout/arguments parses",
    loadRun("CfexecuteControlFixture"), "ok");

// --- gaps --------------------------------------------------------------------

assert("cfexecute with interpolated timeout/arguments parses",
    loadRun("CfexecuteInterpFixture"), "ok");
assert("cfexecute with embedded-quote arguments parses",
    loadRun("CfexecuteQuotedArgFixture"), "ok");

suiteEnd();
</cfscript>
