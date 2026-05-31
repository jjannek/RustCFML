<cfscript>
// Struct/deep-mutation cases adapted from Blute's PR #11 ("Support cfloop
// collection item index"). The scalar-reassign and item-only assertions are
// added here to pin down the full Lucee matrix: `index` aliases `key`, `item`
// binds the value, struct mutations write back via reference types, but a
// scalar reassignment of `item` does NOT write back. Passes on RustCFML + Lucee 7.
suiteBegin("CFLoop collection item and index");

tables = { route = { fields = { profiles = {} } } };
</cfscript>
<cfloop collection="#tables#" item="table" index="tableName">
    <cfset table.visited = tableName />
</cfloop>
<cfscript>
assert("collection cfloop exposes item value and key index", tables.route.visited ?: "missing", "route");

schema = { route = { fields = { profiles = {} } } };
</cfscript>
<cfloop collection="#schema.route.fields#" item="field" index="fieldName">
    <cfset field.generated = fieldName />
</cfloop>
<cfscript>
assert("collection cfloop item mutation writes back", schema.route.fields.profiles.generated ?: "missing", "profiles");

// item alone names the KEY (classic CFML), not the value.
keyOnly = { onlyKey = "ignored" };
itemAloneSaw = "";
</cfscript>
<cfloop collection="#keyOnly#" item="k">
    <cfset itemAloneSaw = k />
</cfloop>
<cfscript>
assert("collection cfloop item-only yields the key", itemAloneSaw, "onlyKey");

// Reassigning a scalar item must NOT write back into the collection (Lucee).
scalars = { x = "old" };
</cfscript>
<cfloop collection="#scalars#" item="v" index="k">
    <cfset v = "changed" />
</cfloop>
<cfscript>
assert("collection cfloop scalar item reassign does not write back", scalars.x, "old");

suiteEnd();
</cfscript>
