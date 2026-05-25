<cfscript>
    request.pageTitle = "D1 + JSPI — current status & blocker";
    request.activeNav = "db";
</cfscript>
<cfinclude template="includes/header.cfm">
<cfoutput>

<div class="panel">
    <div class="panel-header">Goal</div>
    <div class="panel-body">
        <p>Make <code>&lt;cfquery datasource="main"&gt;...&lt;/cfquery&gt;</code>
        in CFML execute synchronously against Cloudflare D1, so that
        existing ColdFusion code runs unmodified at the edge. D1's API
        is Promise-based; the CFML VM is synchronous. JSPI (JavaScript
        Promise Integration) is the standards-track bridge.</p>
    </div>
</div>

<div class="panel">
    <div class="panel-header">What works</div>
    <div class="panel-body">
        <p>The build succeeds and the worker deploys. Direct execution
        against D1 from outside the worker is fine (the dashboard
        shows ~3 queries from <code>wrangler d1 execute</code> seed
        commands). The CFML wiring compiles cleanly; the dynamic
        driver registers, <code>queryExecute</code> dispatches, the JSON
        request reaches the JS Suspending shim's argument-decode step.</p>
    </div>
</div>

<div class="panel">
    <div class="panel-header">What hangs</div>
    <div class="panel-body">
        <p>When CFML invokes <code>&lt;cfquery&gt;</code>, the wasm stack
        reaches the JSPI <code>WebAssembly.Suspending</code> import and
        suspends. It never resumes — the Cloudflare runtime cancels the
        request after ~25 seconds with "code had hung".</p>
    </div>
</div>

<div class="panel">
    <div class="panel-header">Investigation 1 — wasm-bindgen JS adapter (fixed)</div>
    <div class="panel-body">
        <p>wasm-bindgen wraps every snippet-imported function in a JS
        adapter that does <code>&gt;&gt;&gt; 0</code> conversions on the
        arguments:</p>
        <pre class="code">__wbg_cfml_jspi_d1_query_HASH: function(t,e,n,i) {
    return Q(t&gt;&gt;&gt;0, e&gt;&gt;&gt;0, n&gt;&gt;&gt;0, i&gt;&gt;&gt;0)
}</pre>
        <p>where <code>Q = new WebAssembly.Suspending(async ...)</code>.
        With the adapter in place, the wasm imports table receives a
        regular JS function, not the Suspending object itself. JSPI
        requires the Suspending to be installed directly so the runtime
        can recognise the call as suspendable.</p>
        <p>Fixed by post-build patch (<code>jspi-patch.mjs</code>): the
        adapter is replaced with a direct reference to <code>Q</code>.
        <a href="https://github.com/nilslice/workers-zig">workers-zig</a>
        bypasses wasm-bindgen entirely for the same reason.</p>
    </div>
</div>

<div class="panel">
    <div class="panel-header">Investigation 2 — promising wrap on exports (fixed)</div>
    <div class="panel-body">
        <p>The wasm export reached from the JS shim must itself be the
        result of <code>WebAssembly.promising(...)</code>. <code>worker-build</code> 0.8.x
        does not do this — it wires the raw wasm export into a normal JS
        wrapper. The patch hoists a promising wrapper at module init
        and rewrites the wrapper to <code>await</code> it.</p>
    </div>
</div>

<div class="panel">
    <div class="panel-header">Investigation 3 — wasm-bindgen-futures executor (the real blocker)</div>
    <div class="panel-body">
        <p>With both fixes above the JSPI import is reachable, but the
        request still hangs. The reason is architectural:</p>
        <p><code>##[event(fetch)]</code> generates an <em>async</em> Rust
        handler. <code>wasm-bindgen-futures</code> converts that into a
        wasm function that returns a JS Promise <em>immediately</em>:</p>
        <pre class="code">JS event loop microtask
  → wasm-bindgen-futures executor poll()
    → handle_fetch poll()
      → vm.execute_with_lifecycle()
        → D1Driver::execute()
          → cfml_jspi_d1_query()  ← Suspending import here</pre>
        <p>JSPI requires the wasm stack from the Suspending import all
        the way back to a <code>WebAssembly.promising</code> wrap to be a
        single contiguous wasm execution. With
        <code>wasm-bindgen-futures</code>, the chain is broken at every
        async await — the actual work runs inside microtasks <em>outside</em>
        the promising context. When the Suspending import suspends, the
        runtime has nowhere to suspend <em>to</em>: there is no enclosing
        promising frame on the wasm stack. The request never resumes.</p>
    </div>
</div>

<div class="panel">
    <div class="panel-header">Possible fixes (none trivial)</div>
    <div class="panel-body">
        <p><strong>1. Sync wasm entry-point.</strong> Replace
        <code>##[event(fetch)]</code> with a hand-rolled wasm export that
        takes the request synchronously and uses JSPI for every I/O
        call. No <code>wasm-bindgen-futures</code> on the request path.
        All Workers async APIs (KV.get, R2.get, D1, DO fetch, env.var
        lookups, etc.) become JSPI Suspending imports. Substantial
        rewrite of <code>cfml-worker</code>; would also affect the
        <code>handle_scheduled</code> entry.</p>
        <p><strong>2. Pre-resolve queries.</strong> Scan the CFML at
        request start for <code>cfquery</code> blocks and resolve them
        async, then hand a memo-table to the VM. Doesn't generalise —
        prepared statements bound to runtime values can't be statically
        extracted.</p>
        <p><strong>3. Async cfquery in the VM.</strong> Re-architect the
        CFML interpreter to yield from sync to async at I/O calls.
        Massive engine change.</p>
        <p><strong>4. cfhttp to a sidecar Worker.</strong> Run a separate
        Worker that exposes <code>/api/d1?sql=...</code> as a JSON
        endpoint. CFML uses <code>&lt;cfhttp&gt;</code> to call it. No
        JSPI needed but pays per-request HTTP latency.</p>
    </div>
</div>

<div class="panel">
    <div class="panel-header">What I'd ship next</div>
    <div class="panel-body">
        <p>Option 1 (sync wasm entry) is the right long-term answer —
        it unlocks JSPI for KV, R2, DO, Queues, the whole Workers API
        surface from synchronous CFML. The work is bounded: a custom
        <code>##[export_name = "fetch"]</code> wasm export plus
        Suspending imports for each Workers binding type. The KV-backed
        session prime + the DO-backed application prime would migrate
        from <code>.await</code> to the Suspending equivalents.</p>
        <p>Option 4 (cfhttp sidecar) is the right short-term workaround
        if a working demo matters more than architectural cleanliness.</p>
    </div>
</div>

</cfoutput>
<cfinclude template="includes/footer.cfm">
