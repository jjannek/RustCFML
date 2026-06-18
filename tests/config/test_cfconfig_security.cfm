<cfscript>
suiteBegin("cfconfig — security");

// disallowedFunctions: the fixture bans "cfconfigSecurityProbe". Define a
// user function with that exact name and confirm the call is refused —
// the disallow check fires regardless of whether the target is a builtin
// or user-defined.
function cfconfigSecurityProbe() {
    return "should not run";
}
blocked = false;
try {
    cfconfigSecurityProbe();
} catch (any e) {
    if (findNoCase("disallowed by security policy", e.message)) {
        blocked = true;
    }
}
// RustCFML-only: the disallowedFunctions security policy is enforced from
// RustCFML's .cfconfig.json; the Lucee test server has no equivalent loaded.
if (isRustCFML()) assert("disallowedFunctions blocks listed name", blocked, true);

// CSRF is enabled in the fixture — token generation must succeed and produce
// a token. Length is implementation-defined: RustCFML emits a 64-char hex
// string, Lucee 7.0.4 a 40-char one — so the length assert is RustCFML-only.
token = csrfGenerateToken();
if (isRustCFML()) assert("csrf token length", len(token), 64);
assert("csrf token verifies", csrfVerifyToken(token), true);

suiteEnd();
</cfscript>
