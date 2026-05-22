<cfscript>
// Exercises the VM's register_native_class pathway. Skipped unless
// RUSTCFML_NATIVE_SMOKE_TEST=1 — same flag as test_native_fn.cfm — because
// the smoke-test Counter class only gets registered when that env var is set.
smokeFlag = "";
try { smokeFlag = createObject("java", "java.lang.System").getenv("RUSTCFML_NATIVE_SMOKE_TEST"); } catch (any e) {}
if (isNull(smokeFlag)) smokeFlag = "";
if (smokeFlag != "1") {
    suiteBegin("Native classes (skipped — set RUSTCFML_NATIVE_SMOKE_TEST=1 to run)");
    suiteEnd();
    return;
}

suiteBegin("Native classes via createObject('rust', ...)");

// --- Construction ---
c = createObject("rust", "Counter");
assert("initial value is 0", c.get(), 0);
assert("increment returns new value", c.increment(), 1);
assert("increment again", c.increment(), 2);
assert("value persists across calls", c.get(), 2);

// --- Constructor args forwarded ---
c2 = createObject("rust", "Counter", 100);
assert("counter with seed value", c2.get(), 100);
assert("add 5 to seeded", c2.add(5), 105);

// --- Case-insensitive class name + method name ---
c3 = createObject("rust", "counter");
assert("lowercase class name works", c3.get(), 0);
c3.INCREMENT();
assert("uppercase method name dispatches", c3.GET(), 1);

// --- Independent instances don't share state ---
a = createObject("rust", "Counter");
b = createObject("rust", "Counter");
a.add(10);
assert("instance a got the add", a.get(), 10);
assert("instance b is untouched", b.get(), 0);

// --- reset returns null and zeroes ---
c.reset();
assert("after reset value is 0", c.get(), 0);

// --- Unknown method gives a clean error ---
threw = false;
try { c.nonsense(); } catch (any e) { threw = true; }
assertTrue("unknown method throws", threw);

// --- Unknown class gives a clean error ---
threw = false;
try { createObject("rust", "NoSuchThing"); } catch (any e) { threw = true; }
assertTrue("unknown class throws", threw);

// --- isObject recognises NativeObjects ---
nobj = createObject("rust", "Counter");
assertTrue("isObject(NativeObject) is true", isObject(nobj));
assertFalse("isObject(plain struct) is false", isObject({a:1}));
assertFalse("isObject(string) is false", isObject("hello"));

// --- Identity equality: same Arc compares equal ---
ref1 = createObject("rust", "Counter");
ref2 = ref1;
assertTrue("two references to the same NativeObject are ==", ref1 == ref2);
fresh = createObject("rust", "Counter");
assertFalse("two separately-constructed NativeObjects are NOT ==", ref1 == fresh);

// --- writeDump on a NativeObject must not throw ---
threw = false;
try { writeDump(ref1); } catch (any e) { threw = true; }
assertFalse("writeDump on NativeObject does not throw", threw);

// --- NativeObject inside an array survives and mutates correctly ---
arr = [createObject("rust", "Counter"), createObject("rust", "Counter")];
arr[1].add(10);
arr[2].add(20);
assert("array slot 1 retained its state", arr[1].get(), 10);
assert("array slot 2 retained its state", arr[2].get(), 20);
assertFalse("array slots are distinct instances", arr[1] == arr[2]);

// --- NativeObject inside a struct ---
holder = { counter: createObject("rust", "Counter") };
holder.counter.add(7);
assert("struct-held NativeObject state", holder.counter.get(), 7);

// --- NativeObject as a function argument (no copying, mutations stick) ---
function bumpTwice(c) {
    c.increment();
    c.increment();
}
shared = createObject("rust", "Counter");
bumpTwice(shared);
assert("mutations through a function arg propagate", shared.get(), 2);

// --- NativeObject returned from a function ---
function makeCounter(seed) {
    return createObject("rust", "Counter", seed);
}
made = makeCounter(50);
assert("function returning a NativeObject", made.get(), 50);
made.add(5);
assert("returned NativeObject is mutable", made.get(), 55);

// --- arrayMap over an array of NativeObjects ---
counters = [createObject("rust", "Counter"), createObject("rust", "Counter"), createObject("rust", "Counter")];
counters.each(function(c) { c.add(3); });
assert("each() applied to all elements", counters[1].get(), 3);
assert("each() applied to all elements", counters[2].get(), 3);
assert("each() applied to all elements", counters[3].get(), 3);

suiteEnd();
</cfscript>
