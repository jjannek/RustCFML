<cfscript>
suiteBegin("For-in keyword member access");

// Wrapped in a function so `var` is in a valid context (Lucee rejects `var`
// at page scope). The loop target is a dotted variable whose final member
// name is a keyword (`package`) — Lucee treats it as ordinary member access.
function runForInKeyword() {
    var items = ["ok"];
    for (var local.package in items) {
        // assign each item into local.package
    }
    return local.package;
}

assert("script for-in allows keyword property in dotted variable", runForInKeyword(), "ok");

suiteEnd();
</cfscript>
