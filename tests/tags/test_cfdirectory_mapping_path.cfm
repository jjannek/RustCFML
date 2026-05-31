<cfscript>
suiteBegin("cfdirectory mapping paths");
</cfscript>
<cfdirectory action="list"
    directory="/oop/native_cfcs"
    name="mappedCfcFiles"
    filter="*.cfc">
<cfscript>
assert("mapped directory resolves via Application.cfc mapping", mappedCfcFiles.recordCount, 3);
suiteEnd();
</cfscript>
