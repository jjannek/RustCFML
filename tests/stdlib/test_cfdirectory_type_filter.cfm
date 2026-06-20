<!---
  Regression for the Wheels plugin-loader fixes (v0.238.0):
  - cfdirectory action="list" type="dir"|"file" must filter by entry type
    (it previously ignored `type` and returned files as folders, which made
    Wheels' $folders() pass a stray .cfc to directoryList()).
  - directoryList() called on a FILE path must return an empty listing, not
    throw ENOTDIR (os error 20) — matches Lucee.
  Passes on RustCFML + Lucee 7.
--->
<cfscript>
suiteBegin("cfdirectory type filter + directoryList on a file");

base = getTempDirectory() & "/rcfml_dirtype_" & getTickCount();
directoryCreate(base & "/subdir", true);
fileWrite(base & "/afile.cfc", "component {}");
fileWrite(base & "/subdir/inner.txt", "x");

dirsOnly = directoryList(base, false, "query", "*", "name asc", "dir");
assert("type=dir lists only the subdirectory", dirsOnly.recordCount, 1);
assertTrue("the listed entry is the dir", dirsOnly.name[1] == "subdir");

filesOnly = directoryList(base, false, "query", "*", "name asc", "file");
assert("type=file lists only the file", filesOnly.recordCount, 1);
assertTrue("the listed entry is the file", filesOnly.name[1] == "afile.cfc");

all = directoryList(base, false, "query");
assert("type=all (default) lists both", all.recordCount, 2);

// directoryList on a FILE returns empty, not ENOTDIR
onFile = directoryList(base & "/afile.cfc", false, "query");
assert("directoryList on a file returns empty", onFile.recordCount, 0);

directoryDelete(base, true);
suiteEnd();
</cfscript>
