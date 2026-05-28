<cfscript>
suiteBegin("Core: local scope at template (page) scope");

// ============================================================
// Background
// ============================================================
// Inside a function, `local` is the magic auto-vivifying scope for the
// function-local variables. Outside a function — at the top of a .cfm
// template, inside an included script block, or in a CFC pseudo-
// constructor — CFML engines have historically treated `local` as a
// regular auto-vivifying struct on first subscript-write.
//
// Lucee 5/6/7, Adobe ColdFusion 2018-2025, and BoxLang all allow:
//     local.x = 42;
//     writeOutput(local.x);    -- prints 42
//
// CFWheels/Wheels uses this pattern in 581 sites across `vendor/wheels/`
// .cfm templates -- every test runner, every populate.cfm, etc. Without
// it, the framework's test harness throws on the very first assignment.
//
// This file exercises the at-template-scope behavior. The in-function
// behavior of `local` is already covered by tests/core/test_scopes.cfm
// and tests/core/test_localmode.cfm.
// ============================================================

// ------------------------------------------------------------
// Auto-vivify on first subscript write
// ------------------------------------------------------------
local.x = 42;
assert("local.x reads back as 42 after auto-vivify",
    local.x, 42);

// ------------------------------------------------------------
// local should be a struct after vivification
// ------------------------------------------------------------
assertTrue("local is a struct at template scope",
    isStruct(local));

// ------------------------------------------------------------
// Multiple keys on the same auto-vivified local
// ------------------------------------------------------------
local.greeting = "hello";
local.target   = "world";
assert("multi-key local: greeting", local.greeting, "hello");
assert("multi-key local: target",   local.target,   "world");

// structKeyExists works as expected
assertTrue("structKeyExists(local, 'x') after write",
    structKeyExists(local, "x"));
assertFalse("structKeyExists(local, 'never_written') is false",
    structKeyExists(local, "never_written"));

// ------------------------------------------------------------
// Nested keys
// ------------------------------------------------------------
local.config = {host: "localhost", port: 8585};
assert("nested struct: local.config.host",
    local.config.host, "localhost");
assert("nested struct: local.config.port",
    local.config.port, 8585);

// ------------------------------------------------------------
// for (key in struct) iterating local
// ------------------------------------------------------------
total = 0;
keys  = [];
for (k in local) {
    arrayAppend(keys, k);
    total = total + 1;
}
assertTrue("for-in over local visits all keys (total >= 4)",
    total >= 4);

suiteEnd();
</cfscript>
