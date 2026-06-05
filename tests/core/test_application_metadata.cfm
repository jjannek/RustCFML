<cfscript>
suiteBegin("getApplicationMetadata exposes Application.cfc settings");

// Regression: getApplicationMetadata() used to be a stub returning {name:""}.
// It now reflects the live Application.cfc `this` settings (name,
// sessionManagement, ...). WireBox's ScopeStorage reads
// getApplicationMetadata().sessionManagement to decide whether the session
// scope is available.

md = getApplicationMetadata();
assert("application name is reported", md.name, "RustCFMLTests");
assertTrue("sessionManagement is exposed", structKeyExists(md, "sessionManagement"));
assertTrue("sessionManagement is true (set in tests/Application.cfc)", md.sessionManagement);

suiteEnd();
</cfscript>
