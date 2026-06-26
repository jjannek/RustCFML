<cfscript>
suiteBegin("implements unqualified sibling interface (issue 206)");

// A component loaded via a package path that declares an UNQUALIFIED
// implements="X" must resolve X relative to its own directory (like extends),
// not against the calling template's dir. Previously threw
// "Interface 'SiblingIFace' not found".
o = createObject("component", "oop.ifacepkg.SiblingMock");
assert("createObject + unqualified sibling implements works", o.foo("z"), "ok:z");
assertTrue("isInstanceOf recognises the sibling interface", isInstanceOf(o, "oop.ifacepkg.SiblingIFace"));

// The `new` instantiation form must honour it identically.
n = new oop.ifacepkg.SiblingMock();
assert("new + unqualified sibling implements works", n.foo("y"), "ok:y");

suiteEnd();
</cfscript>
