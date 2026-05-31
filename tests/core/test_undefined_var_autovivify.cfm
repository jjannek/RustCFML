<cfscript>
suiteBegin("Core: auto-vivify an undefined variable on member-path assignment");

// ============================================================
// Background
// ============================================================
// On Lucee 5/6/7, Adobe ColdFusion 2018-2025, and BoxLang, assigning to a
// MEMBER PATH of a variable that does not yet exist auto-creates that
// variable as a struct ("auto-vivification"):
//
//     initArgs.path = "wheels";     // initArgs was never declared
//     writeOutput(initArgs.path);   // -> "wheels"
//
// The engine implicitly performs `initArgs = {}` on the first subscript
// write.
//
// CFWheels/Wheels relies on this in public/Application.cfc.onApplicationStart:
//
//     application.wo = application.wheelsdi.getInstance("global");
//     initArgs.path     = "wheels";              // <-- UNDECLARED `initArgs`
//     initArgs.filename = "onapplicationstart";
//     application.wheelsdi.getInstance(name="wheels.events.onapplicationstart",
//                                      initArguments=initArgs).$init(this);
//
// RustCFML auto-vivifies the `local` scope (see test_local_at_template_scope)
// but NOT an arbitrary undeclared variable: `initArgs.path = "wheels"` throws
// `Variable 'initArgs' is undefined`, which aborts onApplicationStart and
// leaves the application un-bootstrapped. The work-around is an explicit
// `var initArgs = {}` first -- but every JVM engine makes that implicit.
//
// All assertions below PASS on Lucee/ACF/BoxLang. Each risky write is
// wrapped in try/catch so a RustCFML "undefined" throw fails the assertion
// gracefully instead of aborting the run.
// ============================================================

// ------------------------------------------------------------
// (1) Single-level: undefined var, one member assigned.
//     rcfmlAutoViv1.path = "wheels" should create rcfmlAutoViv1 = {path:"wheels"}.
// ------------------------------------------------------------
singleResult = "(threw)";
singleErr    = "";
try {
    rcfmlAutoViv1.path = "wheels";
    singleResult = rcfmlAutoViv1.path;
} catch (any e) {
    singleErr = e.message;
}
assert("undefined var auto-vivifies on member assign: x.path == 'wheels'",
    singleResult, "wheels");
assertTrue("no exception thrown auto-vivifying an undefined var (got: [" & singleErr & "])",
    len(singleErr) == 0);

// ------------------------------------------------------------
// (2) The exact Wheels shape: two member writes onto one undeclared var,
//     then read both back (mirrors initArgs.path / initArgs.filename).
// ------------------------------------------------------------
pathVal = "(threw)";
fileVal = "(threw)";
twoErr  = "";
try {
    rcfmlAutoViv2.path     = "wheels";
    rcfmlAutoViv2.filename = "onapplicationstart";
    pathVal = rcfmlAutoViv2.path;
    fileVal = rcfmlAutoViv2.filename;
} catch (any e) {
    twoErr = e.message;
}
assert("Wheels initArgs shape: .path",     pathVal, "wheels");
assert("Wheels initArgs shape: .filename", fileVal, "onapplicationstart");

// ------------------------------------------------------------
// (3) The auto-vivified variable is a real struct.
// ------------------------------------------------------------
isStructResult = false;
try {
    rcfmlAutoViv3.k = 1;
    isStructResult = isStruct(rcfmlAutoViv3);
} catch (any e) {
}
assertTrue("auto-vivified variable is a struct", isStructResult);

// ------------------------------------------------------------
// (4) Inside a function body -- onApplicationStart IS a function, so this
//     is the precise context Wheels hits. An undeclared var member-assign
//     must behave identically here.
// ------------------------------------------------------------
function autoVivInFunction() {
    var out = "(threw)";
    try {
        fnLocalUndeclared.a = "1";
        fnLocalUndeclared.b = "2";
        out = fnLocalUndeclared.a & fnLocalUndeclared.b;
    } catch (any e) {
        out = "(threw)";
    }
    return out;
}
assert("auto-vivify works for an undeclared var inside a function body",
    autoVivInFunction(), "12");

suiteEnd();
</cfscript>
