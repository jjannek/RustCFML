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
assert("disallowedFunctions blocks listed name", blocked, true);

// CSRF is enabled in the fixture — token generation must succeed and produce
// a 64-char hex string.
token = csrfGenerateToken();
assert("csrf token length", len(token), 64);
assert("csrf token verifies", csrfVerifyToken(token), true);

suiteEnd();
</cfscript>
