<!doctype html>
<html><head><meta charset="utf-8"><title>RustCFML writeDump demo</title>
<style>body{font-family:system-ui,sans-serif;margin:24px;max-width:1100px} h2{margin-top:32px;border-bottom:2px solid #b7410e;padding-bottom:4px;color:#8a2a12} p{color:#555}</style>
</head><body>
<p><a href="index.cfm" style="color:#b7410e">&larr; back to examples</a></p>
<h1>RustCFML <code>writeDump</code> / <code>&lt;cfdump&gt;</code></h1>
<p>Rust-themed, collapsible. Click any header bar to expand/collapse.</p>

<h2>1. Struct (nested)</h2>
<cfscript>
data = { name: "RustCFML", version: 222, active: true,
  tags: ["cfml","rust","fast"],
  nested: { a: 1, b: [10,20], c: { deep: "value", when: now() } },
  nothing: "" };
writeDump(data);
</cfscript>

<h2>2. Array of mixed types</h2>
<cfscript>writeDump([1, "two", 3.5, true, { x: 1, y: [9,8] }]);</cfscript>

<h2>3. Query (with metainfo: records, execution time, SQL)</h2>
<cfscript>
q = queryNew("id,name,score", "integer,varchar,decimal");
queryAddRow(q); querySetCell(q,"id",1); querySetCell(q,"name","Ada");   querySetCell(q,"score",9.5);
queryAddRow(q); querySetCell(q,"id",2); querySetCell(q,"name","Linus"); querySetCell(q,"score",8.0);
queryAddRow(q); querySetCell(q,"id",3); querySetCell(q,"name","Grace"); querySetCell(q,"score",9.9);
r = queryExecute("SELECT id, name, score FROM q WHERE score > 5 ORDER BY score DESC", [], {dbtype:"query"});
writeDump(r);
</cfscript>

<h2>4. Component (CFC instance)</h2>
<cfscript>writeDump(new Person());</cfscript>

<h2>5. Java shim object</h2>
<cfscript>writeDump(createObject("java","java.util.Date").init(0));</cfscript>

<h2>6. Labelled + collapsed by default (expand=false)</h2>
<cfscript>writeDump(var=data, label="Click to expand", expand=false);</cfscript>

<h2>7. Depth-limited (cfdump tag, top=1)</h2>
<cfdump var="#data#" label="top=1" top="1">

<h2>8. Scalars</h2>
<cfscript>
writeDump("a plain string");
writeDump(42);
writeDump(3.14159);
writeDump(true);
</cfscript>

</body></html>
