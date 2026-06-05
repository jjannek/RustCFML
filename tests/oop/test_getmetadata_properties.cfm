<cfscript>
suiteBegin("getMetadata() surfaces declared property annotations");

// Regression: getMetadata(componentInstance).properties must list DECLARED
// properties with their custom annotations (inject, type, ...), matching
// getComponentMetadata() and Lucee/ACF. It previously enumerated only
// top-level non-function instance keys and emitted just {name,type}, dropping
// the `inject` annotation entirely — so WireBox's Util.getInheritedMetaData
// (which calls getMetadata() on a built instance) saw no injectable
// properties and silently skipped property/DSL injection.

o  = new MetaPropBag();
md = getMetadata( o );

// find the propDep entry
propDep = "";
for ( p in md.properties ) {
	if ( p.name == "propDep" ) { propDep = p; }
}

assertTrue("declared property appears in metadata", isStruct(propDep));
assert("property name", propDep.name, "propDep");
assert("inject annotation preserved", propDep.inject, "model:ServiceX");

// typed property surfaces its type
typed = "";
for ( p in md.properties ) {
	if ( p.name == "count" ) { typed = p; }
}
assertTrue("typed property appears", isStruct(typed));
assert("property type preserved", typed.type, "numeric");

// getMetadata and getComponentMetadata agree on property count
assert(
	"getMetadata and getComponentMetadata agree on property count",
	arrayLen( md.properties ),
	arrayLen( getComponentMetadata( "oop.MetaPropBag" ).properties )
);

suiteEnd();
</cfscript>
