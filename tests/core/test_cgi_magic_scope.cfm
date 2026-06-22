<cfscript>
suiteBegin("CGI magic scope");

// The cgi scope is a Lucee-style "magic" scope: reading ANY unset key returns
// an empty string "" (not null/undefined), while structKeyExists still reports
// the unset key as absent. (Wheels' $cgiScope copies optional keys like
// http_x_requested_with out of cgi unconditionally; without "" defaults the
// copy stored null and the null-delete guard dropped it — breaking csrf specs.)

// Dot read of an unset key -> ""
assert("cgi dot unset key is empty string", cgi.http_x_requested_with, "");

// Bracket read of an unset key -> ""
assert("cgi bracket unset key is empty string", cgi["totally_made_up_xyz"], "");

// structKeyExists on an unset key -> false (Lucee parity)
assertFalse("cgi structKeyExists false for unset key", structKeyExists(cgi, "http_x_requested_with"));

// The Wheels $cgiScope copy idiom: rv[key] = cgi[key] must store "" not null,
// so a later read of the copied value is defined.
rv = {};
rv["http_x_requested_with"] = cgi["http_x_requested_with"];
oldVal = rv.http_x_requested_with;
assertTrue("copied unset cgi key is defined", isDefined("oldVal"));
assert("copied unset cgi key is empty string", oldVal, "");

// The internal magic marker must never leak into introspection.
assertFalse("magic marker hidden from structKeyExists",
	structKeyExists(cgi, "__cfml_empty_default_scope__"));
assertFalse("magic marker hidden from structKeyList",
	listFindNoCase(structKeyList(cgi), "__cfml_empty_default_scope__") > 0);

suiteEnd();
</cfscript>
