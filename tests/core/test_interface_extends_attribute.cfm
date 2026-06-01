<cfscript>
suiteBegin("Core: interface declaration extends attribute");

// ============================================================
// Background
// ============================================================
// An interface declaration may declare its parent(s) in either the bareword
// form (`interface extends Foo`) or the attribute form (`interface
// extends="Foo"` / `extends="A,B"`), and — like component headers — the
// attributes are order-independent. RustCFML previously only accepted the
// bareword form, so `interface extends="Foo" {` failed to parse at the `=`
// ("Expected LBrace, found Equal"). The Wheels framework uses the attribute
// form in vendor/wheels/interfaces/ (MiddlewareInterface, AuthStrategy, ...).
//
// Interfaces can't be instantiated, so these fixtures are probed via
// getComponentMetaData (a parse failure throws) and via a component that
// implements the child interface (which must instantiate and run).
// ============================================================

function ifaceParses(required string name) {
	try {
		return isStruct(getComponentMetaData(arguments.name)) ? "ok" : "not-struct";
	} catch (any e) {
		return "PARSE-FAIL";
	}
}

// --- the parse gap: `extends="..."` on an interface --------------------------

assert("interface with an extends attribute parses", ifaceParses("IDeclDog"), "ok");
assert("interface with extends after another attribute parses", ifaceParses("IDeclDogAfterAttr"), "ok");

// --- the parsed interface chain is usable ------------------------------------

dog = createObject("component", "DeclDogFixture");
assert("component implementing the extends-chain interface instantiates", isObject(dog), true);
assert("its declared methods run", dog.species() & "/" & dog.bark(), "canine/woof");
assert("direct isInstanceOf of the implemented interface", isInstanceOf(dog, "IDeclDog"), true);

suiteEnd();
</cfscript>
