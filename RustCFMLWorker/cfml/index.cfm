<cfscript>
    request.pageTitle = "Live demo dashboard";
    request.activeNav = "";
</cfscript>
<cfinclude template="includes/header.cfm">
<cfoutput>

<div class="panel">
    <div class="panel-header">Welcome</div>
    <div class="panel-body">
        <p>This worker runs a CFML interpreter (RustCFML, compiled to WebAssembly)
        on Cloudflare's edge. The pages linked below each exercise a different
        slice of the integration: lazy session storage in Workers KV,
        Durable-Object-backed application scope, and synchronous-from-CFML
        SQL queries against a Cloudflare D1 database via JSPI.</p>
        <p>Edge: <strong>#cgi.cf_ray ?: "(unknown)"#</strong>
        &middot; rendered at <strong>#dateTimeFormat(now(), "yyyy-mm-dd HH:nn:ss")#</strong></p>
    </div>
</div>

<div class="cards">
    <div class="card">
        <h3>/static.cfm <span class="tag">no cookie</span></h3>
        <div class="meta">Lazy session — no write</div>
        <p>A page that never touches the <code>session</code> scope. With
        <code>this.lazySessionCreation = true</code> the engine skips KV
        inserts, skips <code>onSessionStart</code>, and skips
        <code>Set-Cookie</code> entirely.</p>
        <a class="go" href="/static.cfm">Open /static.cfm &rarr;</a>
    </div>
    <div class="card">
        <h3>/session.cfm <span class="tag">greeter</span></h3>
        <div class="meta">Remember-me flow</div>
        <p>Asks for your name on first visit and greets you back on
        subsequent ones. The form submission writes <code>session.greetName</code>,
        which triggers lazy-init: KV record created, <code>onSessionStart</code>
        fires, cookie issued. Anonymous visits stay zero-cost.</p>
        <a class="go" href="/session.cfm">Open /session.cfm &rarr;</a>
    </div>
    <div class="card">
        <h3>/cfml.cfm <span class="tag">language</span></h3>
        <div class="meta">Engine sampler</div>
        <p>Quick rendering of the same examples shown in the wasm
        playground: variables, arrays with higher-order functions, structs,
        closures, fibonacci, fizzbuzz. All running server-side.</p>
        <a class="go" href="/cfml.cfm">Open /cfml.cfm &rarr;</a>
    </div>
</div>

<div class="panel">
    <div class="panel-header">Live application scope (DO-backed)</div>
    <div class="panel-body">
        <p>Application scope is stored in a single Durable Object instance per
        application name. All isolates and regions see the same value, so
        the counter below increments monotonically regardless of which edge
        node serves you.</p>
        <dl class="kv">
            <dt>application.startedAt</dt><dd>#application.startedAt#</dd>
            <dt>application.requestCount</dt><dd>#application.requestCount#</dd>
        </dl>
    </div>
</div>

</cfoutput>
<cfinclude template="includes/footer.cfm">
