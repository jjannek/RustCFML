<cfscript>
suiteBegin("Ordered struct literals");

indexes = [
    "idx_one": { "type": "btree", "fields": "name" },
    "idx_two": { "type": "btree", "fields": "created_at" }
];

assertTrue("bracketed key-value literal builds struct", structKeyExists(indexes, "idx_one"));
assert("bracketed key-value literal preserves member access", indexes.idx_two.fields, "created_at");

suiteEnd();
</cfscript>
