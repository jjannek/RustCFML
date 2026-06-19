<cfscript>
suiteBegin("writeDump rendering");

// Capture writeDump output via cfsavecontent. In the CLI test runner this is
// the plain-text tree (web HTML output is exercised in serve mode separately).
data = { name: "RustCFML", version: 220, tags: ["a","b"], nested: { x: 1 } };

savecontent variable="dumped" {
    writeDump(data);
}

assertTrue("dump labels a struct", findNoCase("Struct", dumped) GT 0);
assertTrue("dump shows a key", findNoCase("name", dumped) GT 0);
assertTrue("dump shows a value", findNoCase("RustCFML", dumped) GT 0);
assertTrue("dump nests arrays", findNoCase("Array", dumped) GT 0);

// Labelled dump emits the label.
savecontent variable="labelled" {
    writeDump(var=data, label="My Label");
}
assertTrue("dump emits label", findNoCase("My Label", labelled) GT 0);

// writeDump of a scalar does not throw and renders the value.
savecontent variable="scalar" {
    writeDump("hello");
}
assertTrue("scalar dump renders value", findNoCase("hello", scalar) GT 0);

suiteEnd();
</cfscript>
