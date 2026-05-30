<cfscript>
suiteBegin("Array: reference semantics (Lucee-compatible)");

// Arrays are reference types: assigning shares the same underlying array, so a
// mutation through one binding is visible through every alias. duplicate()
// breaks the reference. These match Lucee, the reference implementation.

// --- Alias + arrayAppend propagates ---
a = [1, 2, 3];
b = a;
arrayAppend(b, 4);
assert("alias append visible via original", arrayLen(a), 4);
assert("alias append visible via alias", arrayLen(b), 4);
assert("alias append same last element", a[4], 4);

// --- Alias + index assignment propagates ---
c = [1, 2, 3];
d = c;
d[1] = 99;
assert("alias index-set visible via original", c[1], 99);
assert("alias index-set visible via alias", d[1], 99);

// --- Alias + index auto-grow propagates ---
e = [1];
fAlias = e;
fAlias[3] = "z";
assert("alias auto-grow length via original", arrayLen(e), 3);
assert("alias auto-grow value via original", e[3], "z");

// --- Array held in a struct, struct aliased ---
holder = { arr = [1, 2] };
holder2 = holder;
arrayAppend(holder2.arr, 3);
assert("struct-held array shared via alias", arrayLen(holder.arr), 3);

// --- Extracted nested array is the same reference ---
matrix = [[1, 2], [3, 4]];
firstRow = matrix[1];
arrayAppend(firstRow, 99);
assert("extracted row is same reference", arrayLen(matrix[1]), 3);
assert("extracted handle sees mutation", arrayLen(firstRow), 3);

// --- arrayPrepend, arraySort, arrayClear mutate in place (visible to alias) ---
p = [3, 1, 2];
pAlias = p;
arrayPrepend(pAlias, 0);
assert("prepend visible via original", p[1], 0);

s = [3, 1, 2];
sAlias = s;
arraySort(sAlias, "numeric");
assert("sort in place visible via original", s[1], 1);
assert("sort in place last via original", s[3], 3);

cl = [1, 2, 3];
clAlias = cl;
arrayClear(clAlias);
assert("clear in place visible via original", arrayLen(cl), 0);

// --- duplicate() breaks the reference (deep copy) ---
orig = [1, 2, 3];
dup = duplicate(orig);
arrayAppend(dup, 4);
assert("duplicate is independent - original unchanged", arrayLen(orig), 3);
assert("duplicate is independent - copy grew", arrayLen(dup), 4);

// --- duplicate() deep-copies nested arrays ---
nestedOrig = [[1, 2], [3, 4]];
nestedDup = duplicate(nestedOrig);
arrayAppend(nestedDup[1], 99);
assert("duplicate deep-copies nested array", arrayLen(nestedOrig[1]), 2);

// --- A fresh array literal is NOT aliased to a previous one ---
g1 = [1, 2];
g2 = [1, 2];
arrayAppend(g2, 3);
assert("distinct literals are independent", arrayLen(g1), 2);

suiteEnd();
</cfscript>
