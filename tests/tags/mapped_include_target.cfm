<cfscript>
// Include target for test_mapping_include.cfm. Sets a request marker so the
// includer can confirm this file was actually resolved (via the /tags mapping)
// and executed. Lives in tests/tags/, which tests/Application.cfc maps to "/tags".
request.mappedIncludeMarker = "MAPPED_INCLUDE_OK";
</cfscript>
