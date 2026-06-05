<cfscript>
suiteBegin("Application scope persists within a run");

// Regression: a run with no Application.cfc still gets a process/request-lifetime
// application scope, so `application.*` writes persist within the run instead of
// being silently dropped. (Previously CLI/embedded execution left
// application_scope unset, so every `application.x` write vanished.)

application.appProbe = "v1";
assert("application write is readable", application.appProbe, "v1");
assertTrue("structKeyExists sees the application key", structKeyExists(application, "appProbe"));

// persists across a function call boundary
function bumpApp(){
	application.appProbe = "v2";
}
bumpApp();
assert("application mutation in a function persists", application.appProbe, "v2");

// LIVE-REFERENCE semantics (Lucee/ACF): reading the application scope returns a
// live reference, not a snapshot, so the "scope pointer" pattern writes through.
// This is what WireBox's ScopeStorage relies on to cache app-scoped instances.
p = application;
p.viaPointer = "ptr";
assert("scope-pointer write is visible on application", application.viaPointer, "ptr");

// and a write through `application` is visible on the held pointer
application.backRef = "br";
assert("application write is visible on the held pointer", p.backRef, "br");

// nested-key mutation through a held reference also writes through
application.bag = {};
ref = application.bag;
ref.k = "deep";
assert("nested mutation through a held scope reference persists", application.bag.k, "deep");

suiteEnd();
</cfscript>
