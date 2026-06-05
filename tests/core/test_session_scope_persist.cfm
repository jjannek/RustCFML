<cfscript>
suiteBegin("Session scope is a live reference within a request");

// Regression: reading the `session` scope returns a LIVE reference (not a
// snapshot), so the CFML "scope pointer" pattern writes through — matching
// Lucee/ACF and what WireBox's ScopeStorage relies on for session-scoped
// caching. Requires session management (enabled in tests/Application.cfc).

session.sProbe = "v1";
assert("session write is readable", session.sProbe, "v1");

// scope-pointer write-through
p = session;
p.viaPointer = "ptr";
assert("scope-pointer write is visible on session", session.viaPointer, "ptr");

// write through `session` visible on the held pointer
session.backRef = "br";
assert("session write is visible on the held pointer", p.backRef, "br");

// nested-key mutation through a held reference
session.bag = {};
ref = session.bag;
ref.k = "deep";
assert("nested mutation through a held session reference persists", session.bag.k, "deep");

// persists across a function call boundary
function bumpSession(){
	session.sProbe = "v2";
}
bumpSession();
assert("session mutation in a function persists", session.sProbe, "v2");

suiteEnd();
</cfscript>
