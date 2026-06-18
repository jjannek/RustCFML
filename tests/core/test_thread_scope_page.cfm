<cfscript>
suiteBegin("thread soft-scope at page level");

// `thread` is a soft scope: writable/readable even outside a cfthread (TestBox's
// BDDRunner stashes thread.testResults/target/suiteStats at page scope). A real
// variable named `thread` still wins (java.lang.Thread currentThread() pattern).

function usesThreadScope() {
    thread.testResults = "RESULTS";
    thread.target = { name = "t" };
    thread.target.name = "renamed";
    return thread.testResults & "|" & thread.target.name;
}
assert("page-level thread.x write + read round-trips", usesThreadScope(), "RESULTS|renamed");

// A real local variable named `thread` must shadow the soft scope.
function threadAsVar() {
    thread = "i am a value";
    return thread;
}
assert("a local var named thread wins over the soft scope", threadAsVar(), "i am a value");

suiteEnd();
</cfscript>
