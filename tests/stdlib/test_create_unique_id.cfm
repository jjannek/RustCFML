<cfscript>
suiteBegin("CreateUniqueID");

firstId = createUniqueID();
secondId = createUniqueID();

assert("createUniqueID length", len(firstId), 32);
assertTrue("createUniqueID returns uppercase hex", reFind("^[0-9A-F]{32}$", firstId) == 1);
assertTrue("createUniqueID returns distinct values", firstId != secondId);

suiteEnd();
</cfscript>
