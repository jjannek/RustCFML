<cfscript>
suiteBegin("getMetadata().extends carries the superclass functions (GitHub 210)");

md = getMetadata( new oop.MetaExtendsChild210() );

// Child's own metadata lists ONLY its own methods (Lucee/ACF — NOT flattened
// with inherited methods). A flattened `.functions` plus a recursive `.extends`
// would double-count inherited lifecycle hooks in TestBox.
ownFns = md.functions.map( function(f){ return arguments.f.name; } );
assert("child .functions is own-only (count)", arrayLen(ownFns), 1);
assertTrue("child .functions has cMethod", arrayFindNoCase(ownFns, "cMethod") gt 0);
assertFalse("child .functions does NOT include inherited pMethod", arrayFindNoCase(ownFns, "pMethod") gt 0);

// .extends is present and carries the parent's full metadata, including its
// own functions array (the gap reported in #210).
assertTrue("extends present", structKeyExists(md, "extends"));
assertTrue("extends.functions present", structKeyExists(md.extends, "functions"));
parentFns = md.extends.functions.map( function(f){ return arguments.f.name; } );
assertTrue("parent functions include pMethod", arrayFindNoCase(parentFns, "pMethod") gt 0);
assertTrue("parent functions include pSetup", arrayFindNoCase(parentFns, "pSetup") gt 0);

// Inherited lifecycle annotation is discoverable as a FLAT key on the parent's
// function struct — the exact shape TestBox's getAnnotatedMethods() reads
// (`structKeyExists( thisFunction, annotation )`).
pSetupMeta = "";
for (f in md.extends.functions) { if (f.name == "pSetup") { pSetupMeta = f; break; } }
assertTrue("found pSetup in parent metadata", isStruct(pSetupMeta));
assertTrue("inherited @beforeAll is a flat key on the parent function", structKeyExists(pSetupMeta, "beforeAll"));

// Component-level displayName is NOT propagated onto the child's leaf (it lives
// on the parent, reachable via .extends).
assertFalse("child leaf has no inherited displayName", structKeyExists(md, "displayName"));
assert("parent retains its own displayName", md.extends.displayName, "Parent210Label");

suiteEnd();
</cfscript>
