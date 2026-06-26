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

// --- interface `functions` via getComponentMetaData(path) (issue #205) ---
// getComponentMetadata() on an interface must populate `functions` with the
// declared method signatures (Lucee/ACF do). MockBox createStub(implements=…)
// reads `.functions` (and each function's `.parameters`) to generate stubs.
im = getComponentMetaData("oop.MetaIFaceFixture");
assertTrue("interface meta has functions key", structKeyExists(im, "functions"));
assertTrue("interface functions is a non-empty array", isArray(im.functions) && arrayLen(im.functions) == 1);
assert("interface function name", im.functions[1].name, "greet");
assert("interface function returntype", im.functions[1].returntype, "string");
assertTrue("interface function has parameters array", structKeyExists(im.functions[1], "parameters") && isArray(im.functions[1].parameters));

// Declared parameters surface with name/type/required (a method that has args).
ip = getComponentMetaData("oop.MetaIFaceParamFixture");
pfn = ip.functions[1];
assert("interface param method name", pfn.name, "configure");
assertTrue("interface method has a parameter", arrayLen(pfn.parameters) == 1);
assert("interface param name", pfn.parameters[1].name, "id");
assertTrue("interface param required", pfn.parameters[1].required);

// --- inherited interface methods reachable via extends[fqn].functions ---
// MockBox recurses md.extends[fqn].functions to stub inherited signatures.
sm = getComponentMetaData("oop.MetaSubIFaceFixture");
assertTrue("sub-interface own functions present", arrayLen(sm.functions) == 1 && sm.functions[1].name == "farewell");
parentMeta = sm.extends["oop.MetaIFaceFixture"];
assertTrue("parent extends entry has functions", structKeyExists(parentMeta, "functions") && arrayLen(parentMeta.functions) == 1);
assert("inherited function name", parentMeta.functions[1].name, "greet");

suiteEnd();
</cfscript>
