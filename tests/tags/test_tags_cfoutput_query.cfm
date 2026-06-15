<cfscript>suiteBegin("Tags: cfoutput query");

q = queryNew("name,age", "varchar,integer", [
    ["alice", 30], ["bob", 25], ["carol", 40]
]);
</cfscript>

<!--- Basic query-driven looping: one body pass per row, bare column refs
      resolve to the current row and #q.col# is the row scalar. --->
<cfsavecontent variable="basic"><cfoutput query="q">[#name#=#q.age#]</cfoutput></cfsavecontent>
<cfscript>
assert("cfoutput query iterates every row", basic, "[alice=30][bob=25][carol=40]");
</cfscript>

<!--- currentRow / recordCount / columnList are exposed on the query var. --->
<cfsavecontent variable="meta"><cfoutput query="q">#q.currentRow#/#q.recordCount# </cfoutput></cfsavecontent>
<cfscript>
assert("cfoutput query currentRow/recordCount", trim(meta), "1/3 2/3 3/3");
</cfscript>

<!--- startrow / maxrows bound the iteration; currentRow stays absolute. --->
<cfsavecontent variable="bounded"><cfoutput query="q" startrow="2" maxrows="1">#q.currentRow#:#name#</cfoutput></cfsavecontent>
<cfscript>
assert("cfoutput query startrow/maxrows", bounded, "2:bob");
</cfscript>

<!--- The query variable is restored after the loop (not left as a row). --->
<cfscript>
assertTrue("cfoutput query leaves the query intact", isQuery(q));
assert("cfoutput query recordCount after loop", q.recordCount, 3);
</cfscript>

<!--- Empty query: body never runs. --->
<cfscript>empty = queryNew("x", "integer");</cfscript>
<cfsavecontent variable="emptyOut"><cfoutput query="empty">SHOULD-NOT-APPEAR</cfoutput></cfsavecontent>
<cfscript>
assert("cfoutput query empty produces nothing", emptyOut, "");
</cfscript>

<!--- Grouped (control-break) output with a nested detail block. --->
<cfscript>
g = queryNew("dept,name,team", "varchar,varchar,varchar", [
    ["eng", "alice", "core"], ["eng", "bob", "core"], ["eng", "carol", "infra"],
    ["sales", "dave", "west"], ["sales", "erin", "west"]
]);
</cfscript>
<cfsavecontent variable="grouped"><cfoutput query="g" group="dept">[#dept#:<cfoutput>#name#,</cfoutput>]</cfoutput></cfsavecontent>
<cfscript>
assert("cfoutput group single level", grouped, "[eng:alice,bob,carol,][sales:dave,erin,]");
</cfscript>

<!--- Two-level grouping: dept -> team -> detail. --->
<cfsavecontent variable="grouped2"><cfoutput query="g" group="dept">(#dept#<cfoutput group="team">{#team#<cfoutput>#name# </cfoutput>}</cfoutput>)</cfoutput></cfsavecontent>
<cfscript>
assert("cfoutput group two levels", trim(grouped2),
    "(eng{corealice bob }{infracarol })(sales{westdave erin })");
</cfscript>

<!--- Works the same over a Query-of-Queries result (also a Query value). --->
<cfscript>
qoq = queryExecute(
    "SELECT dept, name FROM g WHERE team = 'core' OR team = 'west' ORDER BY dept, name",
    [], {dbtype: "query"});
</cfscript>
<cfsavecontent variable="qoqOut"><cfoutput query="qoq">[#name#]</cfoutput></cfsavecontent>
<cfscript>
assert("cfoutput query over QoQ result", qoqOut, "[alice][bob][dave][erin]");
</cfscript>
<cfsavecontent variable="qoqGrouped"><cfoutput query="qoq" group="dept">#dept#(<cfoutput>#name# </cfoutput>)</cfoutput></cfsavecontent>
<cfscript>
assert("cfoutput group over QoQ result", trim(qoqGrouped), "eng(alice bob )sales(dave erin )");
</cfscript>

<cfscript>suiteEnd();</cfscript>
