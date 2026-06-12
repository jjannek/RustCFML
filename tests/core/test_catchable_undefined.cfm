<cfscript>
// Calling an undefined function (or reading an undefined variable) must raise
// a CATCHABLE exception with a populated message — the standard CFML
// feature-detection idiom (`try { newerBif() } catch (any e) {}`) depends on
// it. Surfaced by RustCFML issue #90: `try { dbinfo(...) } catch` aborted the
// whole request instead of entering the catch block.
suiteBegin("Catchable Undefined Errors");

// --- undefined function call, positional args ---
caught = false;
msg = "";
try {
    thisFunctionDoesNotExist(1, 2);
} catch (any e) {
    caught = true;
    msg = e.message;
}
assertTrue("undefined function call is catchable", caught);
assertTrue("undefined function call has message", len(msg) GT 0);

// --- undefined function call, named args (the dbinfo shape from issue ##90) ---
caught = false;
msg = "";
try {
    thisOneDoesNotExistEither(type = "version", name = "x");
} catch (any e) {
    caught = true;
    msg = e.message;
}
assertTrue("undefined named-arg function call is catchable", caught);
assertTrue("undefined named-arg function call has message", len(msg) GT 0);

// --- execution continues normally after catching ---
afterValue = "reached";
assert("execution continues after caught undefined call", afterValue, "reached");

// --- undefined variable read populates the catch variable ---
caught = false;
msg = "";
try {
    x = thisVariableDoesNotExist;
} catch (any e) {
    caught = true;
    msg = e.message;
}
assertTrue("undefined variable read is catchable", caught);
assertTrue("undefined variable read has message", len(msg) GT 0);

// --- inside a function frame ---
function probeUndefinedInsideFunction() {
    var got = "";
    try {
        totallyMissingFn(a = 1);
    } catch (any e) {
        got = e.message;
    }
    return got;
}
assertTrue("undefined call catchable inside function", len(probeUndefinedInsideFunction()) GT 0);

// --- typed catch `expression` still matches engine-thrown undefined errors ---
// Lucee throws type="expression" for these; assert the catch-any path at
// minimum and that a typed expression catch does not break compilation.
caught = false;
try {
    yetAnotherMissingFn();
} catch (expression e) {
    caught = true;
} catch (any e) {
    caught = true;
}
assertTrue("undefined call caught by typed/any chain", caught);

suiteEnd();
</cfscript>
