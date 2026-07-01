<cfscript>
suiteBegin("Integer-literal member access (x.1 == x[1])");

// Lucee/ACF accept an integer literal as a member key: `a.1` is equivalent to
// `a[1]` for arrays (1-based index) and `s.1` to `s["1"]` for structs. RustCFML
// previously threw a *parse* error ("Expected RParen, found Integer"), which
// aborted the whole template/bundle. See GH #222 (ColdBox WireBox/DSL suite uses
// `callArgs.1`).

// 1. Array: integer-literal key is a 1-based index
a = [ "first", "second", "third" ];
assert("a.1 == a[1]", a.1, "first");
assert("a.2 == a[2]", a.2, "second");
assert("a.3 == a[3]", a.3, "third");
assert("a.1 equals bracket form", a.1, a[1]);

// 2. Struct: integer-literal key resolves the string key
s = { "1" : "one", "2" : "two" };
assert("s.1 == s['1']", s.1, "one");
assert("s.2 == s['2']", s.2, "two");
assert("s.1 equals bracket form", s.1, s[1]);

// 3. Chained integer keys (lexer emits a single Double token here)
m = { "1" : { "2" : "deep" } };
assert("m.1.2 chained", m.1.2, "deep");

// 4. Integer key followed by a normal member
b = [ { name = "x" }, { name = "y" } ];
assert("b.2.name", b.2.name, "y");

// 5. Normal member followed by an integer key
wrap = { items = [ "alpha", "beta" ] };
assert("wrap.items.1", wrap.items.1, "alpha");

// 6. Nested arrays
grid = [ [ 10, 20 ], [ 30, 40 ] ];
assert("grid.2.1", grid.2.1, 30);

suiteEnd();
</cfscript>
