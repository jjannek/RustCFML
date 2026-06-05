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

suiteEnd();
</cfscript>
