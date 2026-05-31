<cfscript>
suiteBegin("cfdirectory mapping paths");

directory action="list"
    directory="/oop/native_cfcs"
    name="mappedCfcFiles"
    filter="*.cfc";

assert("mapped directory resolves via Application.cfc mapping", mappedCfcFiles.recordCount, 3);

suiteEnd();
</cfscript>
