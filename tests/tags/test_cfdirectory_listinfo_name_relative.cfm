<cfscript>
// cfdirectory action="list" recurse=true listinfo="name" must return each
// entry's path RELATIVE to the listed directory (subdirectories included,
// "/"-separated) in the `name` column — NOT the bare filename (Lucee/ACF
// parity).
//
// Regression: RustCFML returned only the basename, so ColdBox WireBox's
// `mapDirectory` (which scans recursively with listinfo="name" and converts the
// returned path's "/" to ".") computed the WRONG instantiation path for any
// nested component — e.g. `preside.system.services.features.FeatureService`
// collapsed to `preside.system.services.FeatureService`, and the component
// failed to load (broke Preside CMS boot).
//
// NB: the directoryList() BIF with listinfo="name" returns basenames on BOTH
// engines (verified) — only the cfdirectory TAG's query uses the relative path.

suiteBegin("cfdirectory listinfo=name returns subdir-relative paths");

base = getTempDirectory() & "/rcfml_listinfo_" & lCase(left(replace(createUUID(), "-", "", "all"), 8));
skip = false;
try {
    directoryCreate(base & "/features", true);
    directoryCreate(base & "/utility", true);
    fileWrite(base & "/Top.cfc", "component {}");
    fileWrite(base & "/features/FeatureService.cfc", "component {}");
    fileWrite(base & "/utility/IgnoreFileService.cfc", "component {}");
} catch (any e) {
    skip = true;
    writeOutput("  (skipped — cannot create fixture dir: " & e.message & ")" & chr(10));
}
</cfscript>

<cfif NOT skip>
    <cfdirectory action="list" directory="#base#" filter="*.cfc" recurse="true" listinfo="name" name="qNames">

    <cfscript>
    // Collect the names into a list for order-independent assertions.
    names = "";
    for ( row in qNames ) {
        names = listAppend( names, row.name );
    }

    assert("listinfo=name returns one column: name", qNames.columnList, "name");
    assert("recurse found all three .cfc files", qNames.recordCount, 3);
    assertTrue("top-level file is its bare name", listFindNoCase(names, "Top.cfc") GT 0);
    assertTrue("nested file carries its subdirectory (features/FeatureService.cfc)", listFindNoCase(names, "features/FeatureService.cfc") GT 0);
    assertTrue("nested file carries its subdirectory (utility/IgnoreFileService.cfc)", listFindNoCase(names, "utility/IgnoreFileService.cfc") GT 0);

    // The default listinfo="all" query keeps name = basename + a directory column.
    </cfscript>

    <cfdirectory action="list" directory="#base#" filter="*.cfc" recurse="true" name="qAll">

    <cfscript>
    allNames = "";
    for ( row in qAll ) {
        allNames = listAppend( allNames, row.name );
    }
    assertTrue("listinfo=all keeps the bare basename in name", listFindNoCase(allNames, "FeatureService.cfc") GT 0);
    assertFalse("listinfo=all does NOT prepend the subdir in name", listFindNoCase(allNames, "features/FeatureService.cfc") GT 0);

    try { directoryDelete(base, true); } catch (any e) {}
    </cfscript>
</cfif>

<cfscript>
suiteEnd();
</cfscript>
