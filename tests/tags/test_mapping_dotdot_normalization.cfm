<cfscript>
suiteBegin("Mappings: '..' in mapping target normalized consistently across file BIFs");

// The "/dotdotprobe" mapping (declared in tests/Application.cfc) points at
// "<webroot>/tags/../oop/". A correct engine collapses the ".." so that
// expandPath, directoryList and fileExists all agree on the canonical path.
// This is the exact shape of Preside's "/preside" -> "../system" mapping: the
// regression (fixed alongside this test) was that directoryList returned a
// literal ".../tags/../oop/..." prefix while expandPath had canonicalized it
// away to ".../oop/...", so Preside's _getAllObjectPaths could not strip the
// expandPath prefix off each entry and produced a malformed doubled path.

expanded = expandPath("/dotdotprobe");

// 1. expandPath collapses the ".." — no "/../" survives.
assertFalse("expandPath result contains no '/../'", expanded contains "/..");

// 2. directoryList entries share the expandPath prefix (i.e. ".." collapsed the
//    same way), so a Preside-style prefix strip yields a clean remainder.
files = directoryList(path="/dotdotprobe", recurse=false, filter="Greeter.cfc");
assert("directoryList found the fixture", arrayLen(files), 1);
entry = files[1];
assertFalse("directoryList entry contains no '/..'", entry contains "/..");
assertTrue("directoryList entry begins with expandPath(dir)", entry.startsWith(expanded));

// 3. The Preside _getAllObjectPaths operation: stripping expandPath(dir) off the
//    entry leaves only the relative tail, never a leftover absolute path.
remainder = replace(entry, expanded, "");
assert("prefix strip leaves clean relative remainder", remainder, "/Greeter.cfc");

// 4. fileExists agrees on the mapping path (resolves through the same code).
assertTrue("fileExists resolves '/dotdotprobe/Greeter.cfc'", fileExists("/dotdotprobe/Greeter.cfc"));

suiteEnd();
</cfscript>
