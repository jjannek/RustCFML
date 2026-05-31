<cfscript>
suiteBegin("Len scalar coercion");

assert("len(false) stringifies scalar", len(false), 5);
assert("len(true) stringifies scalar", len(true), 4);
assert("len(number) stringifies scalar", len(42), 2);

suiteEnd();
</cfscript>
