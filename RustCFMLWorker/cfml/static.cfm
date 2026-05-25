<cfscript>
    request.pageTitle = "Static page — no session touch";
    request.activeNav = "static";
</cfscript>
<cfinclude template="includes/header.cfm">
<cfoutput>

<div class="panel">
    <div class="panel-header">What this page does</div>
    <div class="panel-body">
        <p>Nothing involving the <code>session</code> scope.
        The CFML code on this page renders the timestamp and a counter
        from <code>application</code> scope — that's it.</p>
        <p>Because Application.cfc opts into
        <code>this.lazySessionCreation = true</code>, the engine does
        <em>not</em> create a session record in KV, does <em>not</em> fire
        <code>onSessionStart</code>, and does <em>not</em> emit a
        <code>Set-Cookie</code> header on the response. Pages with this
        shape cost zero KV operations.</p>
    </div>
</div>

<div class="panel">
    <div class="panel-header">Server-resolved values</div>
    <div class="panel-body">
        <dl class="kv">
            <dt>now()</dt><dd>#dateTimeFormat(now(), "yyyy-mm-dd HH:nn:ss")#</dd>
            <dt>application.requestCount</dt><dd>#application.requestCount#</dd>
            <dt>cgi.cf_ray</dt><dd>#cgi.cf_ray ?: ""#</dd>
            <dt>cgi.request_method</dt><dd>#cgi.request_method#</dd>
        </dl>
        <p>Inspect the response headers (e.g. <code>curl -i</code>) — you'll
        see no <code>Set-Cookie</code> line.</p>
    </div>
</div>

</cfoutput>
<cfinclude template="includes/footer.cfm">
