<!doctype html>
<html><head><meta charset="utf-8"><title>RustCFML writeDump examples</title>
<style>
	body{font-family:system-ui,sans-serif;margin:40px auto;max-width:760px;color:#2b2018;line-height:1.5}
	h1{color:#8a2a12} a{color:#b7410e;font-weight:600}
	.card{display:block;border:1px solid #e9d3c4;border-left:4px solid #b7410e;border-radius:6px;padding:14px 18px;margin:14px 0;text-decoration:none;color:inherit}
	.card:hover{background:#faece3} .card h2{margin:0 0 4px;font-size:18px;color:#b7410e} .card p{margin:0;color:#555}
	code{background:#f3eee9;padding:1px 5px;border-radius:3px}
</style>
</head><body>
	<h1>RustCFML <code>writeDump</code> / <code>&lt;cfdump&gt;</code></h1>
	<p>A collapsible, Rust-themed dump renderer. Serve this folder and open the demos below:</p>
	<p><code>rustcfml --serve examples/dumpdemo</code></p>

	<a class="card" href="dump.cfm">
		<h2>1. Value types &rarr;</h2>
		<p>Structs, arrays, queries, a CFC component, a Java shim object, labelled &amp;
		collapsed dumps, depth limiting (<code>top</code>), and type-coloured scalars.</p>
	</a>

	<a class="card" href="qoq.cfm">
		<h2>2. Query-of-Queries &rarr;</h2>
		<p>Joins, <code>GROUP BY</code> + aggregates, <code>HAVING</code>, an inline
		<code>group_concat</code> aggregate UDF, and a heavy self-join so you can watch
		the execution time in the query footer.</p>
	</a>
</body></html>
