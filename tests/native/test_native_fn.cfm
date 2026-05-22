<cfscript>
// Exercises the VM's register_native_fn pathway. The runner only registers
// the smoke-test functions when RUSTCFML_NATIVE_SMOKE_TEST=1; when the env
// var is not set, the whole suite no-ops so the rest of the test run stays
// unaffected.
smokeFlag = "";
try { smokeFlag = createObject("java", "java.lang.System").getenv("RUSTCFML_NATIVE_SMOKE_TEST"); } catch (any e) {}
if (isNull(smokeFlag)) smokeFlag = "";
if (smokeFlag != "1") {
    suiteBegin("Native functions (skipped — set RUSTCFML_NATIVE_SMOKE_TEST=1 to run)");
    suiteEnd();
    return;
}

suiteBegin("Native functions registered via register_native_fn");

// --- Direct call ---
assert("nativeAdd(2, 3) returns 5", nativeAdd(2, 3), 5);
assert("nativeAdd handles negatives", nativeAdd(-4, 1), -3);
assert("nativeAdd coerces numeric strings", nativeAdd("10", "5"), 15);
assert("nativeGreet default", nativeGreet(), "Hello, World!");
assert("nativeGreet with arg", nativeGreet("Alex"), "Hello, Alex!");

// --- Case-insensitive dispatch (CFML convention) ---
assert("NATIVEADD (uppercase) dispatches", NATIVEADD(7, 8), 15);
assert("nativegreet (lowercase) dispatches", nativegreet("there"), "Hello, there!");

// --- First-class value: assign to variable and call ---
adder = nativeAdd;
assert("native fn assigned to var, invoked", adder(11, 22), 33);

// --- First-class value: pass to higher-order builtin ---
nums = [1, 2, 3, 4];
doubled = arrayMap(nums, function(n) { return nativeAdd(n, n); });
assert("native fn called inside arrayMap closure", doubled[1], 2);
assert("native fn called inside arrayMap closure", doubled[2], 4);
assert("native fn called inside arrayMap closure", doubled[3], 6);
assert("native fn called inside arrayMap closure", doubled[4], 8);

suiteEnd();
</cfscript>
