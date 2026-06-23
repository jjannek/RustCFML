<cfscript>
suiteBegin("getMetadata implements / interface extends");

// --- `implements` via getMetadata(instance) ---
// Wheels' ServiceProviderInterfaceSpec uses GetMetadata(provider) and expects
// meta.implements to be a struct keyed by the declared interface FQN.
obj = createObject("component", "oop.MetaImplFixture");
m = getMetadata(obj);
assertTrue("getMetadata has implements key", structKeyExists(m, "implements"));
assertTrue("getMetadata implements is a struct", isStruct(m.implements));
assertTrue("getMetadata implements declares the interface", structKeyExists(m.implements, "oop.MetaIFaceFixture"));

// --- `implements` via getComponentMetaData(path) ---
// Wheels' CompileTimeEnforcementSpec uses getComponentMetaData(name).
cm = getComponentMetaData("oop.MetaImplFixture");
assertTrue("getComponentMetaData has implements key", structKeyExists(cm, "implements"));
assertTrue("getComponentMetaData implements is a struct", isStruct(cm.implements));
assertTrue("getComponentMetaData implements declares the interface", structKeyExists(cm.implements, "oop.MetaIFaceFixture"));

// --- interface `extends` via getComponentMetaData(path) ---
// Wheels' InterfaceCompilationSpec checks an interface that extends another
// exposes a non-empty `extends` struct.
si = getComponentMetaData("oop.MetaSubIFaceFixture");
assertTrue("interface meta has extends key", structKeyExists(si, "extends"));
assertTrue("interface extends is a non-empty struct", isStruct(si.extends) && !structIsEmpty(si.extends));

suiteEnd();
</cfscript>
