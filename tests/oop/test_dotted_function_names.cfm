<cfscript>
suiteBegin("Dotted function names");

route = createObject("component", "oop.MetadataDottedFunctionRoute");
md = getMetaData(route);

functionNames = [];
for (fn in md.functions) {
    arrayAppend(functionNames, fn.name);
}

assertTrue(
    "component metadata preserves dotted function name",
    arrayFind(functionNames, "uploadFileToServerWithProgress.profile_picture_id") > 0
);
assert("dotted function is callable", route["uploadFileToServerWithProgress.profile_picture_id"](), "ok");

suiteEnd();
</cfscript>
