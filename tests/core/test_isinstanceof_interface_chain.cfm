<cfscript>
suiteBegin("Core: isInstanceOf walks the interface extends chain");

// ============================================================
// Background
// ============================================================
// isInstanceOf() must recognise INHERITED interfaces, not just the directly
// declared ones. The fixtures form a chain:
//   DeclDogFixture  implements  IDeclDog  extends  IDeclCreature
// so an instance is an IDeclDog (direct) AND an IDeclCreature (via the
// interface's own extends). On Lucee/Adobe CF/BoxLang both are true.
//
// RustCFML built the transitive __implements_chain only for `new X()`, not for
// `createObject("component", …)`, so createObject-instantiated components
// reported isInstanceOf(comp, grandparentInterface) == false. Both instantiation
// forms now honour interface inheritance identically.
// ============================================================

dogNew = new DeclDogFixture();
dogCreate = createObject("component", "DeclDogFixture");

// direct interface (worked before) -------------------------------------------
assert("new: instance is its directly-implemented interface", isInstanceOf(dogNew, "IDeclDog"), true);
assert("createObject: instance is its directly-implemented interface", isInstanceOf(dogCreate, "IDeclDog"), true);

// inherited interface (the gap) ----------------------------------------------
assert("new: instance is the interface's PARENT interface", isInstanceOf(dogNew, "IDeclCreature"), true);
assert("createObject: instance is the interface's PARENT interface", isInstanceOf(dogCreate, "IDeclCreature"), true);

// negative control -----------------------------------------------------------
assert("not an instance of an unrelated type", isInstanceOf(dogCreate, "SomethingElse"), false);

suiteEnd();
</cfscript>
