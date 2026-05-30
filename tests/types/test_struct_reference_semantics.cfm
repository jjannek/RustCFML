<cfscript>
suiteBegin("Struct: reference semantics (Lucee-compatible)");

// Structs are reference types: assigning shares the same underlying struct, so
// a mutation through one binding is visible through every alias. duplicate()
// breaks the reference; structCopy() is a shallow copy. These match Lucee, the
// reference implementation.

// --- Alias + member-set propagates ---
a = { x = 1 };
b = a;
b.x = 99;
assert("alias member-set visible via original", a.x, 99);
assert("alias member-set visible via alias", b.x, 99);

// --- Alias + new key via member-set propagates ---
o = { n = 1 };
p = o;
p.n2 = "new";
assert("alias new-key visible via original", o.n2, "new");

// --- Alias + structInsert propagates ---
s = {};
t = s;
structInsert(t, "k", 5);
assert("alias structInsert key exists via original", structKeyExists(s, "k"), true);
assert("alias structInsert value via original", s.k, 5);

// --- Alias + structDelete propagates ---
d1 = { a = 1, b = 2 };
d2 = d1;
structDelete(d2, "a");
assert("alias structDelete visible via original", structKeyExists(d1, "a"), false);
assert("alias structDelete keeps other key", d1.b, 2);

// --- Alias + structAppend mutates the target in place ---
ap1 = { a = 1 };
ap2 = ap1;
structAppend(ap1, { b = 2 });
assert("structAppend visible via alias", ap2.b, 2);

// --- Struct held in a struct, outer aliased ---
holder = { inner = { v = 1 } };
holder2 = holder;
holder2.inner.v = 42;
assert("nested struct shared via alias", holder.inner.v, 42);

// --- Extracted nested struct is the same reference ---
parent = { child = { count = 1 } };
childRef = parent.child;
childRef.count = 7;
assert("extracted nested struct is same reference", parent.child.count, 7);

// --- Array held in a struct (cross-type reference) ---
ah = { list = [1, 2] };
ah2 = ah;
arrayAppend(ah2.list, 3);
assert("struct-held array shared via alias", arrayLen(ah.list), 3);

// --- CFC instance aliasing (a component instance is a struct under the hood) ---
c1 = new RefThing(1);
c2 = c1;
c2.setVal(7);
assert("CFC instance aliasing - mutation visible via original", c1.getVal(), 7);

// --- duplicate() breaks the reference (deep copy) ---
orig = { x = 1, nested = { y = 2 } };
dup = duplicate(orig);
dup.x = 0;
dup.nested.y = 0;
assert("duplicate independent - top-level unchanged", orig.x, 1);
assert("duplicate deep-copies nested struct", orig.nested.y, 2);

// --- structCopy() is a SHALLOW copy: top level independent... ---
scOrig = { x = 1, nested = { y = 2 } };
scCopy = structCopy(scOrig);
scCopy.x = 0;
assert("structCopy top-level independent", scOrig.x, 1);
// ...but nested references are shared (shallow)
scCopy.nested.y = 99;
assert("structCopy nested is shared (shallow)", scOrig.nested.y, 99);

// --- A fresh struct literal is NOT aliased to a previous one ---
g1 = { v = 1 };
g2 = { v = 1 };
g2.v = 2;
assert("distinct literals are independent", g1.v, 1);

// --- In-place mutation through a function parameter propagates (reference) ---
function mutateArg(s) { s.k = "Z"; structInsert(s, "added", true); }
fnArg = { k = "A" };
mutateArg(fnArg);
assert("fn-arg member-set propagates", fnArg.k, "Z");
assert("fn-arg structInsert propagates", structKeyExists(fnArg, "added"), true);

// --- REASSIGNING a parameter does NOT affect the caller (Lucee semantics) ---
function reassignArg(s) { s = { other = 1 }; }
keepMe = { v = 1 };
reassignArg(keepMe);
assert("param reassignment does not leak to caller", keepMe.v, 1);
assert("param reassignment does not add caller keys", structKeyExists(keepMe, "other"), false);

suiteEnd();
</cfscript>
