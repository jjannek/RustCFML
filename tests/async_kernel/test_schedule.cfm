<cfscript>
suiteBegin("_schedule (one-shot)");

// ---- delay=0: schedule fires immediately and resolves like runAsync.
f0 = _schedule(function() { return "now"; }, 0);
assert("_schedule(delay=0) returns value", f0.get(), "now");
assertTrue("schedule future isDone", f0.isDone());

// ---- delay>0: fires after the delay; we measure the elapsed wallclock to
// confirm we actually waited.
start = getTickCount();
fDelay = _schedule(function() { return getTickCount(); }, 80);
fireTick = fDelay.get();
elapsed = getTickCount() - start;
assertTrue("delayed schedule waited >= 50ms", elapsed gte 50);
assertTrue("delayed schedule produced a numeric tick", isNumeric(fireTick));

// ---- Struct options form
fOpts = _schedule(function() { return 7; }, { delayMs: 0 });
assert("_schedule({delayMs:0}) returns value", fOpts.get(), 7);

// ---- cancel before fire: cooperative cancel during the sleep window
fCancelMe = _schedule(function() { return "should not run"; }, 1000);
didCancel = fCancelMe.cancel();
assertTrue("schedule cancel returns true", didCancel);
// Give the relay a beat to see the flag and post TERMINATED.
res = fCancelMe.get(2000);
assert("cancelled schedule -> status TERMINATED", fCancelMe.status(), "TERMINATED");

writeOutput("[async_kernel] _schedule tests OK" & chr(10));
suiteEnd();
</cfscript>
