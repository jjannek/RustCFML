<cfscript>
suiteBegin("Wheels batch v0.294 engine fixes");

// --- List functions preserve empty elements, index by non-empty (Lucee) ---
assert("ListSetAt preserves leading empties", listSetAt("......wheels00", 1, "Wheels00", "."), "......Wheels00");
assert("ListSetAt 2nd non-empty", listSetAt(",,a,b", 2, "X", ","), ",,a,X");
assert("ListSetAt keeps interior empty", listSetAt("a,,b", 2, "X", ","), "a,,X");
assert("ListInsertAt before nth non-empty", listInsertAt(",,a,b", 2, "X", ","), ",,a,X,b");
assert("ListDeleteAt nth non-empty", listDeleteAt(",,a,b", 2, ","), ",,a");

// --- GetDirectoryFromPath preserves redundant separators ---
assert("GetDirectoryFromPath keeps //", getDirectoryFromPath("/a/b/models//PhotoGallery.cfc"), "/a/b/models//");
assert("GetDirectoryFromPath single", getDirectoryFromPath("/a/b/x.cfc"), "/a/b/");

// --- isValid component / binary ---
assertTrue("isValid component on java obj", isValid("component", createObject("java", "java.lang.StringBuilder")));
assertFalse("isValid component on query", isValid("component", queryNew("id")));
assertTrue("isValid binary", isValid("binary", toBinary(toBase64("hi"))));

// --- isDate rejects bare numerics, accepts date strings ---
assertFalse("isDate(11)", isDate(11));
assertFalse("isDate('11')", isDate("11"));
assertTrue("isDate iso", isDate("2024-01-15"));
assertTrue("isDate no-seconds 12h", isDate("11/01/1975 12:00 AM"));
// Lucee: a bare time-of-day is a valid date (resolves to today at that time).
assertTrue("isDate time 12h", isDate("6:15 PM"));
assertTrue("isDate time 24h", isDate("18:15"));
assertTrue("isValid date time-of-day", isValid("date", "6:15 PM"));

// --- Canonicalize keeps literal + (no form-decode) ---
assert("Canonicalize keeps +", canonicalize("Istok+Web", false, false), "Istok+Web");

// --- isDefined sees query columns ---
qd = queryNew("id,name");
assertTrue("isDefined query col", isDefined("qd.id"));
assertFalse("isDefined missing query col", isDefined("qd.nope"));

// --- package as a bare struct key ---
e = {package = "missingreq", message = "hi"};
assert("package bare struct key", e.package, "missingreq");

// --- cfdirectory action=list on missing dir returns empty query (no throw) ---
cfdirectory(action="list", directory=expandPath("/no/such/dir_zzz/"), name="dq", filter="*.cfc");
assert("cfdirectory missing dir empty", dq.recordCount, 0);

// --- cfsetting requesttimeout round-trips via getPageContext ---
setting requestTimeout=666;
assert("getRequestTimeout ms", getPageContext().getRequestTimeout(), 666000);

// --- implicit set* must not shadow onMissingMethod for unknown properties ---
omm = new compat_engine.fixtures.OnMissingSetter();
assert("set unknown routes to onMissingMethod", omm.setWidget("x"), "OMM:setWidget");
omm.setColor("red"); // declared property -> implicit setter
assert("set known uses implicit setter", omm.color, "red");

suiteEnd();
</cfscript>
