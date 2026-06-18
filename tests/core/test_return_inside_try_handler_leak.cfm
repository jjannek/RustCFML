<cfscript>
suiteBegin("Exceptions: return from inside a try does not leak its handler");

// Regression for the TestBox CoverageServiceTest fatal recursion. A `return`
// (or cfexit / running off the end) from *inside* an open try block jumps
// straight to the function epilogue without reaching the matching TryEnd, so
// the handler pushed by TryStart was left dangling on the VM's try-stack. A
// later throw in an unrelated frame was then misrouted to that stale handler's
// catch_ip, re-executing code in a tight loop (toBeTrue -> isTrue -> fail ...)
// until the recursion guard tripped. Each frame must now truncate the try-stack
// back to its entry depth on exit.

function returnsInsideTry() {
    try {
        // succeeds — no exception — and returns from WITHIN the try block,
        // so the matching `catch`/end is never reached.
        var x = 1;
        return true;
    } catch (any e) {
        return false;
    }
}

function doThrow() {
    throw(type = "MyError", message = "boom");
}

// 1. Prime the (buggy) leak: call a fn that returns from inside a try.
flag = returnsInsideTry();
assert("function returning from inside a try yields its value", flag, true);

// 2. A subsequent uncaught throw must propagate exactly once, NOT be captured
//    by the leaked handler and looped. assertThrows confirms it surfaces.
assertThrows("uncaught throw after return-inside-try propagates (not looped)", function() {
    doThrow();
});

// 3. A throw that IS wrapped in its own try must be caught by THAT try, once,
//    even after a prior return-inside-try primed the stack.
flag2 = returnsInsideTry();
caught = "";
try {
    doThrow();
} catch (any e) {
    caught = e.message;
}
assert("a real surrounding try catches the throw exactly once", caught, "boom");

// 4. Nested: an inner fn that returns-inside-try, called from within an outer
//    try, must not steal the outer try's handler when a sibling throw fires.
hits = [];
try {
    if (returnsInsideTry()) {
        throw(type = "Sibling", message = "s");
    }
} catch (any e) {
    arrayAppend(hits, e.message);
}
assert("sibling throw routes to the correct enclosing handler once", hits.toList(), "s");

suiteEnd();
</cfscript>
