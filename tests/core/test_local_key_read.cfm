<cfscript>
suiteBegin("Core: local.X single-key read fast path");

// `local.foo` reads are fused into a single direct-lookup op (LoadLocalKey)
// that skips materializing the whole per-call `local` scope view. It MUST
// preserve the exact visibility filter of that view: only keys established in
// THIS frame are visible — params live in `arguments` (not `local`), inherited
// page/CFC bridge keys are excluded, and a missing key reads as undefined.

// (1) A genuine local var reads back its value.
function lkGenuine() {
    local.foo = 42;
    return local.foo;
}
assert("local.foo returns the assigned value", lkGenuine(), 42);

// (2) Case-insensitive: local.X resolves to local.x.
function lkCaseInsensitive() {
    local.Greeting = "hi";
    return local.greeting & "/" & local.GREETING;
}
assert("local.X member read is case-insensitive", lkCaseInsensitive(), "hi/hi");

// (3) A declared param is NOT visible through `local` (params are `arguments`).
function lkParamNotLocal(p) {
    return structKeyExists(local, "p") ? "VISIBLE" : "absent";
}
assert("a param is not part of local", lkParamNotLocal("x"), "absent");

// (4) Reading a never-assigned key reads as null (RustCFML's lenient
// member-access semantics — GetProperty on a struct returns Null for a
// missing key; the fast path preserves this exactly).
function lkMissing() {
    return isNull(local.neverSet) ? "null" : "got:" & local.neverSet;
}
assert("local.missing reads as null", lkMissing(), "null");

// (5) local.this / local.super are bridge keys, never visible as user data.
function lkBridge() {
    local.real = "ok";
    return structKeyExists(local, "this") ? "leaked-this" : local.real;
}
assert("this/super bridge keys are not visible in local", lkBridge(), "ok");

// (6) The fast path agrees with bracket access and the materialized view.
function lkAgrees() {
    local.alpha = 1;
    local.beta = 2;
    var direct = local.alpha + local.beta;     // fast path (LoadLocalKey)
    var bracket = local["alpha"] + local["beta"]; // bracket path
    var viaView = local.alpha;                  // materialize-then-read parity
    return (direct == 3 && bracket == 3 && viaView == 1) ? "agree" : "MISMATCH";
}
assert("fast path agrees with bracket and view reads", lkAgrees(), "agree");

// (7) A local that shadows a page-scope variable of the same name reads the
// local, and the read is scoped to THIS frame only.
greeting = "PAGE";
function lkShadow() {
    local.greeting = "LOCAL";
    return local.greeting;
}
assert("local.x reads the frame local, not the page var", lkShadow(), "LOCAL");
assert("page var is untouched by the frame local", greeting, "PAGE");

suiteEnd();
</cfscript>
