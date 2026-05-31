<cfscript>
suiteBegin("String member regex functions");

route = "/sysadmin/routes/[route_id]";
assert("string.reFind passes pattern before receiver", route.reFind("\[\w+\]"), 18);
assert("string.reFindNoCase passes pattern before receiver", route.reFindNoCase("\[ROUTE_\w+\]"), 18);
assert("string.reMatch passes pattern before receiver", arrayToList(route.reMatch("\[\w+\]")), "[route_id]");
assert("string.reMatchNoCase passes pattern before receiver", arrayToList(route.reMatchNoCase("\[ROUTE_\w+\]")), "[route_id]");

// reReplace is string-first: receiver stays as the first argument (no swap).
assert("string.reReplace keeps receiver as first arg", route.reReplace("\[\w+\]", "ID"), "/sysadmin/routes/ID");
assert("string.reReplaceNoCase keeps receiver as first arg", route.reReplaceNoCase("\[ROUTE_\w+\]", "ID"), "/sysadmin/routes/ID");

suiteEnd();
</cfscript>
