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

// Issue #157: a never-generated, syntactically-valid 64-hex string must NOT
// verify (the bug accepted ANY 64-hex token, defeating CSRF protection).
forged = repeatString("a", 64);
assert("forged 64-hex token rejected", csrfVerifyToken(forged), false);
// Obviously-wrong shapes stay rejected too.
assert("short token rejected", csrfVerifyToken(repeatString("a", 32)), false);
assert("non-hex token rejected", csrfVerifyToken(repeatString("z", 64)), false);
assert("empty token rejected", csrfVerifyToken(""), false);

// Tokens are namespaced by key: a token issued for one key must not verify
// under a different key.
tokenA = csrfGenerateToken("keyA");
tokenB = csrfGenerateToken("keyB");
assert("keyed token verifies under its key", csrfVerifyToken(tokenA, "keyA"), true);
assert("keyed token rejected under other key", csrfVerifyToken(tokenA, "keyB"), false);
assert("keyA and keyB tokens differ", tokenA != tokenB, true);

// Default behaviour reuses the same token per key; forceNew mints a new one.
again = csrfGenerateToken("keyA");
assert("default reuses the per-key token", again, tokenA);
fresh = csrfGenerateToken("keyA", true);
if (isRustCFML()) assert("forceNew mints a new token", fresh != tokenA, true);
assert("forceNew token verifies", csrfVerifyToken(fresh, "keyA"), true);

suiteEnd();
</cfscript>
