<cfscript>
suiteBegin("Tags: attribute interpolation sweep");
</cfscript>

<!---
    ============================================================
    Background
    ============================================================
    The tag-attribute interpolation work (#62) and the cflog/cfmail follow-up
    (#72) established that a quoted tag attribute carrying #...# interpolation
    must route through format_attr_value: it handles a whole-value #expr#, a
    MIXED literal+#expr# value (url="/page?id=#id#"), and embedded quotes (which
    CFML escapes by DOUBLING, "", not with a backslash — the lexer ends a string
    at a lone ").

    Several other tag arms still hand-rolled strip_hashes + backslash escaping:
    cfheader, cfcontent, cflocation, cfcookie, cfsetting, cfcache, cfdirectory,
    cfzip, cflock, cfthread (custom attrs + name), cfexecute (name), and the
    cfqueryparam value builder. A mixed value or an embedded quote on any of them
    failed to PARSE. This sweep pins all of them.

    Because that is a hard parse error (escapes try/catch, would abort the
    runner), the cases live in runtime-instantiated fixture CFCs, and every tag
    is behind <cfif false> so it is compiled but never executed (no header set,
    no redirect, no thread spawned, no filesystem touched).
    ============================================================
--->

<cfscript>
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

assert("mixed literal + interpolation attributes across many tags parse",
    loadRun("TagAttrInterpSweepFixture"), "ok");
assert("embedded-quote attributes across many tags parse",
    loadRun("TagAttrQuotedSweepFixture"), "ok");

suiteEnd();
</cfscript>
