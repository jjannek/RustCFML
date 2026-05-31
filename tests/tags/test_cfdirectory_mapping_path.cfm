<cfscript>
suiteBegin("cfdirectory mapping paths");
</cfscript>
<cfdirectory action="list"
    directory="/oop/native_cfcs"
    name="mappedCfcFiles"
    filter="*.cfc">
<cfscript>
assert("mapped directory resolves via Application.cfc mapping", mappedCfcFiles.recordCount, 3);
assertTrue("cfdirectory list result is query", isQuery(mappedCfcFiles));

if (isQuery(mappedCfcFiles)) {
    assertTrue("cfdirectory query has name column", queryColumnExists(mappedCfcFiles, "name"));
    assertTrue("cfdirectory query has directory column", queryColumnExists(mappedCfcFiles, "directory"));
    assertTrue("cfdirectory query has size column", queryColumnExists(mappedCfcFiles, "size"));
    assertTrue("cfdirectory query has type column", queryColumnExists(mappedCfcFiles, "type"));
    assertTrue("cfdirectory query has dateLastModified column", queryColumnExists(mappedCfcFiles, "dateLastModified"));
    assertTrue("cfdirectory query has attributes column", queryColumnExists(mappedCfcFiles, "attributes"));
    assertTrue("cfdirectory query has mode column", queryColumnExists(mappedCfcFiles, "mode"));

    firstMappedFile = queryGetRow(mappedCfcFiles, 1);
    assertTrue("cfdirectory row exposes cfc file name", listFindNoCase(valueList(mappedCfcFiles.name), "counter_child.cfc") > 0);
    assertTrue("cfdirectory row exposes mapped directory", findNoCase("/tests/oop/native_cfcs", replace(firstMappedFile.directory, "\", "/", "all")) > 0);
    assert("cfdirectory row type is file", firstMappedFile.type, "file");
}
suiteEnd();
</cfscript>
