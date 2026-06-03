<cfscript>
suiteBegin("cfthread Concurrency");
</cfscript>

<!--- Multiple threads run and each writes its OWN key into the shared request
      scope; join-all then proves every body ran and its write is visible.
      Distinct keys avoid relying on cflock (a no-op in CLI mode). --->
<cfthread name="cw1"><cfset request.cft_r1 = 11></cfthread>
<cfthread name="cw2"><cfset request.cft_r2 = 22></cfthread>
<cfthread name="cw3"><cfset request.cft_r3 = 33></cfthread>
<cfthread action="join" timeout="10000"/>

<cfscript>
assert(
	"three threads all wrote the shared request scope",
	request.cft_r1 & "," & request.cft_r2 & "," & request.cft_r3,
	"11,22,33"
);
assert("cfthread.cw1 completed", cfthread.cw1.status, "COMPLETED");
assert("cfthread.cw2 completed", cfthread.cw2.status, "COMPLETED");
assert("cfthread.cw3 completed", cfthread.cw3.status, "COMPLETED");
</cfscript>

<!--- request scope is shared live: a thread's write is visible to the parent
      after join (CFML request scope crosses thread boundaries). --->
<cfthread name="rsh"><cfset request.cft_shared = "from-thread"></cfthread>
<cfthread action="join" name="rsh" timeout="5000"/>

<cfscript>
assert("request scope shared across threads", request.cft_shared, "from-thread");
</cfscript>

<!--- attributes passed to cfthread are bound from the parent context at spawn
      and exposed as the thread's `attributes` scope. --->
<cfset greeting = "hello">
<cfthread name="att" who="world" extra="#greeting#">
	<cfset thread.msg = attributes.who & "/" & attributes.extra>
</cfthread>
<cfthread action="join" name="att" timeout="5000"/>

<cfscript>
assert("thread attributes literal + interpolated", cfthread.att.msg, "world/hello");
</cfscript>

<!--- thread scope values surface as cfthread.NAME.* --->
<cfthread name="ts">
	<cfset thread.label = "done">
	<cfset thread.count = 42>
</cfthread>
<cfthread action="join" name="ts" timeout="5000"/>

<cfscript>
assert("thread scope string surfaced", cfthread.ts.label, "done");
assert("thread scope number surfaced", cfthread.ts.count, 42);
</cfscript>

<!--- an error inside a thread becomes status TERMINATED with a captured message,
      and does NOT abort the parent request. --->
<cfthread name="boom"><cfthrow message="kaboom"></cfthread>
<cfthread action="join" name="boom" timeout="5000"/>

<cfscript>
assertTrue("thread error captured", len(cfthread.boom.error) > 0);
assert("thread error status is TERMINATED", cfthread.boom.status, "TERMINATED");
assert("parent survives a thread error", 1 + 1, 2);
</cfscript>

<!--- threadJoin() script BIF: joins by name and surfaces the thread scope --->
<cfthread name="bif1"><cfset thread.v = 7></cfthread>
<cfscript>
threadJoin("bif1", 5000);
assert("threadJoin() BIF surfaces thread scope", cfthread.bif1.v, 7);
assert("threadJoin() BIF status", cfthread.bif1.status, "COMPLETED");
</cfscript>

<!--- threadTerminate() script BIF is callable and harmless on a finished thread.
      (Terminating a CPU-bound loop is timing-dependent and not deterministic
      across engines, so it isn't asserted here; cooperative cancellation of a
      running loop is covered by a local smoke test.) --->
<cfthread name="bif2"><cfset thread.done = true></cfthread>
<cfscript>
threadJoin("bif2", 5000);
threadTerminate("bif2");
assert("threadTerminate() BIF callable, completed thread unchanged", cfthread.bif2.status, "COMPLETED");
</cfscript>

<!--- join with no name joins all outstanding threads --->
<cfthread name="ja1"><cfset request.cft_ja1 = "a"></cfthread>
<cfthread name="ja2"><cfset request.cft_ja2 = "b"></cfthread>
<cfthread action="join" timeout="10000"/>

<cfscript>
assert("join-all completes every outstanding thread", request.cft_ja1 & request.cft_ja2, "ab");
suiteEnd();
</cfscript>
