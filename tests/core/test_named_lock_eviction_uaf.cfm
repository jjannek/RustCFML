<cfscript>
suiteBegin("Concurrency: an actively-held named lock is never evicted out from under its live guard");

// Regression for a use-after-free that SIGSEGV'd serve mode (found running
// Preside-CMS's PresideObjectServiceTest, which acquires >1024 distinct named
// locks via per-object locking).
//
// The named-lock registry (server_state.named_locks) is capped at 1024 entries;
// once exceeded, acquiring a NEW name evicts every "idle" entry. The old eviction
// heuristic treated a lock as idle when its Arc strong_count was 1 — but the
// cflock acquisition path stored ONLY the lifetime-extended ('static) RwLock
// guard in held_locks, NOT a clone of the backing Arc. So an actively-held lock
// also had strong_count 1, got evicted, its RwLock allocation was freed, and the
// dangling 'static guard crashed the process in RwLock::unlock_queue when the
// block finally released it.
//
// Repro: hold an OUTER lock, then while holding it acquire >1024 distinct INNER
// names so an eviction sweep runs mid-block. Pre-fix the outer lock was evicted
// and exiting its block dereferenced freed memory (SIGSEGV). Post-fix the held
// lock keeps a retained Arc (strong_count >= 2), so it survives the sweep.
//
// In CLI mode named locks are a no-op (no server_state), so this is harmless
// there; the teeth are in serve-mode validation (cold + warm), where a regression
// crashes the whole runner rather than failing a single assertion.

trace = "START ";
err = "";
try {
    lock name="uafOuterLock" timeout="10" type="exclusive" {
        trace &= "OUTER_IN ";
        // Acquire well past the 1024-entry cap so an eviction sweep fires while
        // the outer lock is still held.
        for (i = 1; i <= 1300; i++) {
            lock name="uafInner_#i#" timeout="10" type="exclusive" {
                // no-op: acquire + release, growing the registry
            }
        }
        trace &= "OUTER_STILL_HELD ";
    }
    trace &= "OUTER_RELEASED";
} catch (any e) {
    err = e.message;
}

// If we reached here at all in serve mode, the process did not SIGSEGV.
assertTrue("outer lock body ran", findNoCase("OUTER_IN", trace) GT 0);
assertTrue("outer lock still valid after >1024 inner acquisitions (no eviction UAF)",
    findNoCase("OUTER_STILL_HELD", trace) GT 0);
assertTrue("outer lock released cleanly (no crash on guard drop)",
    findNoCase("OUTER_RELEASED", trace) GT 0);
assert("no lock error during the run", err, "");

suiteEnd();
</cfscript>
