<cfscript>
suiteBegin("Array: append fast-path + index auto-grow");

// ---------------------------------------------------------------------------
// arrayAppend in-place fast path (bug #11: was O(n²), now O(1) amortized).
// These assert correctness of the fused ArrayAppendLocal op, not speed.
// ---------------------------------------------------------------------------

// Basic loop append
a = [];
for (i = 1; i <= 100; i++) {
    arrayAppend(a, i);
}
assert("loop append length", arrayLen(a), 100);
assert("loop append first", a[1], 1);
assert("loop append last", a[100], 100);

// Append starting from a non-empty array
b = [10, 20];
arrayAppend(b, 30);
assert("append to non-empty length", arrayLen(b), 3);
assert("append to non-empty value", b[3], 30);

// Self-append: the appended value must be a real snapshot of the array (NOT a
// null/corrupted slot, which a naive "take the slot out first" fast path would
// produce). The value is evaluated before the in-place mutation, so the nested
// element is a genuine array, not null.
// NB: the nested element's contents differ across engines — RustCFML arrays are
// value types (nested = copy of pre-append c), Lucee arrays are reference types
// (nested = self-reference) — so only the outer shape is asserted here.
c = [1, 2];
arrayAppend(c, c);
assert("self-append outer length", arrayLen(c), 3);
assertTrue("self-append nested is array", isArray(c[3]));
assertFalse("self-append nested is not null", isNull(c[3]));

// The appended value can reference an element of the same array.
d = ["first", "second"];
arrayAppend(d, d[1]);
assert("append own element length", arrayLen(d), 3);
assert("append own element value", d[3], "first");

// A closure that captures the array must still see appends made after capture.
function makeAppender() {
    var local_arr = [];
    var add = function(v) { arrayAppend(local_arr, v); };
    add(1);
    add(2);
    add(3);
    return local_arr;
}
captured = makeAppender();
assert("closure-captured append length", arrayLen(captured), 3);
assert("closure-captured append last", captured[3], 3);

// Members syntax still works (.append) — uses the generic path, not the fused op.
m = [1];
m.append(2);
assert("member append length", arrayLen(m), 2);
assert("member append value", m[2], 2);

// 3-arg form stays on the generic path (the fused fast path is 2-arg only).
// merge=true flattens an array value into the target (Lucee/ACF semantics).
merged = [1, 2];
arrayAppend(merged, [3, 4], true);
assert("merge append length", arrayLen(merged), 4);
assert("merge append element 3", merged[3], 3);
assert("merge append element 4", merged[4], 4);

// merge=false (or omitted) appends an array value as a single nested element.
nested = [1, 2];
arrayAppend(nested, [3, 4], false);
assert("no-merge append length", arrayLen(nested), 3);
assertTrue("no-merge append nested is array", isArray(nested[3]));

// ---------------------------------------------------------------------------
// Index assignment past the end auto-grows (bug #10).
// Lucee semantics: the length grows but skipped slots are non-existent (null).
// ---------------------------------------------------------------------------

g = [];
g[1] = "x";
assert("grow empty to 1 length", arrayLen(g), 1);
assert("grow empty to 1 value", g[1], "x");

// Lucee grows the length but leaves skipped slots as non-existent (null) holes.
g2 = [];
g2[3] = "z";
assert("grow with gap length", arrayLen(g2), 3);
assertTrue("grow with gap leaves null hole 1", isNull(g2[1]));
assertTrue("grow with gap leaves null hole 2", isNull(g2[2]));
assert("grow with gap value", g2[3], "z");
assert("grow with gap serializes holes as null", serializeJSON(g2), '[null,null,"z"]');

g3 = [1, 2];
g3[5] = 5;
assert("grow non-empty length", arrayLen(g3), 5);
assert("grow non-empty keeps existing", g3[2], 2);
assertTrue("grow non-empty leaves null hole", isNull(g3[3]));
assert("grow non-empty value", g3[5], 5);

// Assigning to an existing index still overwrites (not grow).
g4 = ["a", "b", "c"];
g4[2] = "B";
assert("overwrite existing length", arrayLen(g4), 3);
assert("overwrite existing value", g4[2], "B");

suiteEnd();
</cfscript>
