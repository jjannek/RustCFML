<cfscript>
suiteBegin("server scope is writable (Lucee/ACF parity)");

// The server scope used to be synthetic & rebuilt per access, so writes to it
// vanished silently. cbjavaloader caches its classloader in server[key] and
// reads it back, so this write-through is required for Preside (ColdBox) boot.

// --- dot write/read ---
server.test_dotkey = "DOT";
assertTrue("server.x dot write persists (structKeyExists)", structKeyExists(server, "test_dotkey"));
assert("server.x dot write reads back", server.test_dotkey, "DOT");

// --- bracket write/read ---
server["test_brkkey"] = "BRK";
assert("server[k] bracket write reads back", server["test_brkkey"], "BRK");

// --- dynamic (computed) key, the cbjavaloader pattern ---
dynKey = "cbox-javaloader-" & "deadbeef";
if (NOT structKeyExists(server, dynKey)) {
	server[dynKey] = { loaded = true, name = "NetworkClassLoader" };
}
assertTrue("server[computedKey] persists", structKeyExists(server, dynKey));
assert("server[computedKey] struct value survives", server[dynKey].name, "NetworkClassLoader");

// --- the synthetic baseline is preserved alongside user writes ---
assertTrue("baseline server.os still present", structKeyExists(server, "os"));
assertTrue("baseline server.separator still present", structKeyExists(server, "separator"));

// --- overwrite an existing user key ---
server.test_dotkey = "DOT2";
assert("server user key overwrite", server.test_dotkey, "DOT2");

// --- nested write ---
server.test_nested = {};
server.test_nested.child = "NESTED";
assert("server nested write reads back", server.test_nested.child, "NESTED");

// --- writes visible through a cross-function call (page-level server is global) ---
function readServerKey(required string k) {
	return structKeyExists(server, k) ? server[k] : "MISSING";
}
assert("server write visible inside a function", readServerKey("test_brkkey"), "BRK");

suiteEnd();
</cfscript>
