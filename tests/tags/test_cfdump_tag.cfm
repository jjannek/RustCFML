<cfscript>
suiteBegin("cfdump tag");
</cfscript>

<cfsavecontent variable="out"><cfdump var="#{ name: 'RustCFML', items: [1,2] }#" label="Tag Label" expand="false" top="1"></cfsavecontent>

<cfscript>
// The tag forwards label/expand/top to writeDump as named args.
assertTrue("cfdump tag emits label", findNoCase("Tag Label", out) GT 0);
assertTrue("cfdump tag renders struct", findNoCase("Struct", out) GT 0);
assertTrue("cfdump tag renders key", findNoCase("name", out) GT 0);
suiteEnd();
</cfscript>
