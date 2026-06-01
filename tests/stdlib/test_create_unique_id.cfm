<cfscript>
suiteBegin("CreateUniqueID");

firstId = createUniqueID();
secondId = createUniqueID();

// Lucee returns a 22-char URL-safe Base64 encoded UUID (no padding).
assert("createUniqueID length", len(firstId), 22);
assertTrue("createUniqueID returns URL-safe base64", reFind("^[0-9A-Za-z_\-]{22}$", firstId) == 1);
assertTrue("createUniqueID returns distinct values", firstId != secondId);

// The "counter" form returns an incrementing per-instance integer.
c1 = createUniqueID("counter");
c2 = createUniqueID("counter");
assertTrue("createUniqueID counter is numeric", isNumeric(c1));
assertTrue("createUniqueID counter increments", val(c2) == val(c1) + 1);

suiteEnd();
</cfscript>
