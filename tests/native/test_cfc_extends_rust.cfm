<cfscript>
// Exercises a CFC inheriting from a Rust-registered class via
// `extends="rust:Counter"`. Skipped unless RUSTCFML_NATIVE_SMOKE_TEST=1
// (same smoke gate as test_native_class.cfm) because the Counter class
// only registers under that flag.
smokeFlag = "";
try { smokeFlag = createObject("java", "java.lang.System").getenv("RUSTCFML_NATIVE_SMOKE_TEST"); } catch (any e) {}
if (isNull(smokeFlag)) smokeFlag = "";
if (smokeFlag != "1") {
    suiteBegin("CFC extends rust: (skipped — set RUSTCFML_NATIVE_SMOKE_TEST=1 to run)");
    suiteEnd();
    return;
}

suiteBegin("CFC extends rust: — construction + dispatch");

// Construction: a CFC declaring extends="rust:Counter" gets a default-
// constructed parent attached under __super.
inst = createObject("component", "oop.native_cfcs.counter_child");
assertTrue("instance is a struct", isStruct(inst));
assertTrue("instance has __super after construction", structKeyExists(inst, "__super"));

// Unknown rust class on a CFC parent must error at construction.
threw = false;
try {
    createObject("component", "oop.native_cfcs.counter_bad_parent");
} catch (any e) {
    threw = true;
}
assertTrue("unknown rust parent class throws at construction", threw);

// super.X dispatches to the Rust parent (default-constructed Counter at 0).
assert("super.get() on fresh instance is 0", inst.bumpTwice(), 2);
assert("CFC override of add() calls super.add(n*2)", inst.add(5), 12);

// Implicit fall-through: CFC doesn't define `increment`, so inst.increment()
// should reach the Rust parent's method.
fresh = createObject("component", "oop.native_cfcs.counter_child");
assert("implicit fall-through to parent.increment", fresh.increment(), 1);
assert("implicit fall-through to parent.get",       fresh.get(),       1);

// Each child gets its own parent instance — no shared state.
a = createObject("component", "oop.native_cfcs.counter_child");
b = createObject("component", "oop.native_cfcs.counter_child");
a.increment();
assert("instance a parent state",   a.get(), 1);
assert("instance b parent untouched", b.get(), 0);

// isInstanceOf recognises the rust: parent name.
assertTrue("isInstanceOf(inst, 'rust:Counter') is true", isInstanceOf(inst, "rust:Counter"));
assertFalse("isInstanceOf(inst, 'rust:Other') is false", isInstanceOf(inst, "rust:Other"));

// getMetadata surfaces the rust: parent under extends.name.
md = getMetadata(inst);
assert("getMetadata extends.name carries rust: prefix", md.extends.name, "rust:Counter");

// Explicit super(args) replaces the default-constructed parent.
seeded = createObject("component", "oop.native_cfcs.counter_seeded").init(42);
assert("super(args) seeded parent state", seeded.get(), 42);
assert("seeded parent still dispatches add()", seeded.add(8), 50);

// A separately-constructed seeded instance is independent.
other = createObject("component", "oop.native_cfcs.counter_seeded").init(7);
assert("second seeded instance has its own parent", other.get(), 7);
assert("first seeded instance unaffected", seeded.get(), 50);

// Property fall-through: CFC has no `value` field but parent exposes one
// via get_property/set_property — reads and writes route through the trait.
propInst = createObject("component", "oop.native_cfcs.counter_seeded").init(100);
assert("read this.value falls through to parent", propInst.value, 100);
propInst.value = 250;
assert("write this.value routes to parent.set_property", propInst.value, 250);
assert("parent state visible via super.get()",       propInst.get(),    250);

// Unknown native properties fall back to the CFC struct.
propInst.cfcOnly = "hello";
assert("unknown property writes land on the CFC struct", propInst.cfcOnly, "hello");

suiteEnd();
</cfscript>
