<cfscript>
suiteBegin("lock / try-finally exit semantics");

// Regression: try/finally (and the `lock {}` desugaring, which is
// `try { body } finally { __cflock_end() }`) mishandled non-normal exits.
//   1. An exception thrown inside a lock/try-finally was SWALLOWED (execution
//      fell through past the lock instead of propagating).
//   2. A `return` inside a lock skipped the finally, so the lock was never
//      released -> the next acquire of the same lock deadlocked (cflock timeout).
// Both surfaced booting WireBox (Singleton.getFromScope locks, builds, and
// `return`s inside the lock; autowire throwing inside the lock was masked).

// --- 1. exception thrown inside a lock propagates (with type preserved) ---
function throwInLock(){
	lock name="tlfs1" type="exclusive" timeout="5" {
		throw( type = "CustomBoom", message = "kaboom" );
	}
	return "SHOULD-NOT-REACH";
}
caught = "";
caughtType = "";
try {
	throwInLock();
} catch ( any e ) {
	caught = e.message;
	caughtType = e.type;
}
assert("exception propagates out of lock (message)", caught, "kaboom");
assert("exception type preserved across frame", caughtType, "CustomBoom");

// --- 2. rethrow inside a lock propagates ---
function rethrowInLock(){
	lock name="tlfs2" type="exclusive" timeout="5" {
		try {
			throw( type = "Inner", message = "inner" );
		} catch ( any e ) {
			rethrow;
		}
	}
	return "SHOULD-NOT-REACH";
}
caught2 = "";
try { rethrowInLock(); } catch ( any e ) { caught2 = e.type; }
assert("rethrow propagates out of lock", caught2, "Inner");

// --- 3. return inside a lock releases the lock (no deadlock on re-acquire) ---
function acquireAndReturn(){
	lock name="tlfs_shared" type="exclusive" timeout="3" {
		return "got-it";
	}
	return "fell-through";
}
first = acquireAndReturn();
second = acquireAndReturn();   // would time out if the first leaked the lock
assert("return-in-lock returns the in-lock value", first, "got-it");
assert("lock released on return (re-acquire succeeds)", second, "got-it");

// --- 4. plain try/finally: finally runs on a normal return ---
ran = { finally = false };
function returnWithFinally( flag ){
	try {
		return "early";
	} finally {
		arguments.flag.finally = true;
	}
}
rv = returnWithFinally( ran );
assert("finally runs on return (value)", rv, "early");
assertTrue("finally body executed on return", ran.finally);

// --- 5. try/catch nested INSIDE a finally, with a rethrow in the outer catch ---
// Regression (Preside TaskManagerService.runTask): a `rethrow` (or `return`)
// emits its enclosing finally inline at compile time. When that finally body
// itself contains a `try {} catch {}` (whose own catch has no finally), the
// inner construct read the SAME finally off the compiler's finally_stack and
// re-emitted it — recursing until the native stack overflowed AT COMPILE TIME
// (a hard process abort while loading the .cfc). Fixed by popping the finally
// being emitted inline. AND: the inner try's throw-and-swallow must not change
// which exception the outer `rethrow` re-raises (it clobbered last_exception);
// fixed with SaveException/RestoreException around the inline finally.
function rethrowWithTryInFinally(){
	try {
		throw( type = "Outer", message = "boom" );
	} catch ( any e ) {
		rethrow;
	} finally {
		try {
			throw( type = "Inner", message = "inner" );
		} catch ( any e2 ) {
			// swallowed — must NOT become the propagated exception
		}
	}
}
reThrew = "";
try {
	rethrowWithTryInFinally();
} catch ( any e ) {
	reThrew = e.message;
}
assert("rethrow re-raises the OUTER exception, not the finally's swallowed inner", reThrew, "boom");

suiteEnd();
</cfscript>
