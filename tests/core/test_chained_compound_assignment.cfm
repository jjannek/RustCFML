<cfscript>
suiteBegin("Core: chained / nested compound assignment");

// ============================================================
// Background  (parse gap surfaced in PR #32 by bpamiri)
// ============================================================
// CFML assignment is right-associative, so a compound-assignment operator may
// appear as the right-hand side of an assignment: `a = b &= c` means
// `a = (b = b & c)`. Lucee 5/6/7, Adobe CF 2018-2025, and BoxLang accept it;
// RustCFML used to reject it ("Invalid assignment target"). Used in
// vendor/wheels/migrator/Base.cfc: `local.sql = local.sql &= ";"`.
//
// (Top-level statement compound assignment — `a += 1;` — is unchanged; only the
// RHS-of-an-assignment position is newly accepted.)
// ============================================================

sql = "SELECT 1";
sql = sql &= ";";
assert("chained &= concatenates and assigns through", sql, "SELECT 1;");

n = 5;
n = n += 3;
assert("chained += adds and assigns through", n, 8);

// nested member-access target on both sides
obj = { v = "a" };
obj.v = obj.v &= "b";
assert("chained &= on a struct member", obj.v, "ab");

// plain statement compound assignment still works (regression guard)
m = 10;
m += 5;
assert("plain += statement unaffected", m, 15);

suiteEnd();
</cfscript>
