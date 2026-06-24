<cfscript>
suiteBegin("Nested-CFC-property method variables-writeback (Wheels model errorsSpec)");

// Regression: calling a `variables`-mutating method on a CFC that is stored as a
// PROPERTY of another CFC (`user.author.addError(...)`) must write the method's
// `variables` mutations back to the RECEIVER (`user.author`), not the OUTER
// object (`user`). The CallMethod variables-writeback used `path[..len-1]`,
// dropping the receiver's last path segment, so for write_back=["user","author"]
// it merged the author's variables into `user`'s __variables — clobbering it.
// In Wheels this made `user.$classData()` report the AUTHOR class after building
// `user.author`, so allErrors(includeAssociations=true) looped the wrong
// associations and returned 1 error instead of 2. Lucee keeps them isolated.

user   = new NestedWbThing("USER");
author = new NestedWbThing("AUTHOR");
user.child = author;

assert("outer object class before nested mutation", user.getKlass(), "USER");

// mutating method on the nested CFC property
user.child.touch();

assert("outer object class survives nested method writeback", user.getKlass(), "USER");
assert("nested object class is correct", user.child.getKlass(), "AUTHOR");

// deeper nesting: a.b.c.mutate() must hit `c`, not `a`/`b`
a = new NestedWbThing("A");
b = new NestedWbThing("B");
c = new NestedWbThing("C");
a.b = b;
a.b.c = c;
a.b.c.touch();
assert("3-level: root class survives", a.getKlass(), "A");
assert("3-level: mid class survives", a.b.getKlass(), "B");
assert("3-level: leaf class correct", a.b.c.getKlass(), "C");

suiteEnd();
</cfscript>
