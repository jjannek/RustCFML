<cfscript>
// NativeObject must survive being shared across cfthread boundaries.
// Skipped unless RUSTCFML_NATIVE_SMOKE_TEST=1.
smokeFlag = "";
try { smokeFlag = createObject("java", "java.lang.System").getenv("RUSTCFML_NATIVE_SMOKE_TEST"); } catch (any e) {}
if (isNull(smokeFlag)) smokeFlag = "";
if (smokeFlag != "1") {
    suiteBegin("Native classes across threads (skipped — set RUSTCFML_NATIVE_SMOKE_TEST=1 to run)");
    suiteEnd();
    return;
}

suiteBegin("NativeObject across cfthread");

// A single Counter mutated concurrently by N threads. Because the RwLock
// inside the NativeObject serialises writes, the final count must equal
// the total number of increments — no torn writes, no lost updates.
shared = createObject("rust", "Counter");
threadCount = 4;
bumpsPerThread = 25;

for (i = 1; i <= threadCount; i++) {
    threadName = "bump_" & i;
    thread name="#threadName#" action="run" {
        for (j = 1; j <= bumpsPerThread; j++) {
            shared.increment();
        }
    }
}

// Join all spawned threads before checking the final value.
for (i = 1; i <= threadCount; i++) {
    threadName = "bump_" & i;
    thread name="#threadName#" action="join";
}

assert(
    "concurrent increments are not lost (RwLock-serialised)",
    shared.get(),
    threadCount * bumpsPerThread
);

suiteEnd();
</cfscript>
