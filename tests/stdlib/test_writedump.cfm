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

// Labelled dump (script form) emits the label.
savecontent variable="labelled" {
    writeDump(var=data, label="My Label");
}
assertTrue("dump emits label", findNoCase("My Label", labelled) GT 0);

// writeDump of a scalar does not throw and renders the value.
savecontent variable="scalar" {
    writeDump("hello");
}
assertTrue("scalar dump renders value", findNoCase("hello", scalar) GT 0);

// Java shim objects render as "Java <class>" not a raw struct of __ markers.
shim = createObject("java", "java.util.Date").init(0);
savecontent variable="shimdump" {
    writeDump(shim);
}
assertTrue("java shim labelled Java", findNoCase("Java", shimdump) GT 0);
assertTrue("java shim shows class", findNoCase("java.util.date", shimdump) GT 0);
assertFalse("java shim hides __java_shim marker", findNoCase("__java_shim", shimdump) GT 0);

suiteEnd();
</cfscript>
