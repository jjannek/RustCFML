<cfscript>
    request.pageTitle = "Session greeter — remember-me demo";
    request.activeNav = "session";

    // Form action: explicit "forget me" wipes the session. Any other
    // POST with a `name` field starts (or refreshes) the greeting.
    if (structKeyExists(form, "forget")) {
        sessionInvalidate();
    } else if (structKeyExists(form, "name") && len(trim(form.name))) {
        session.greetName = trim(form.name);
        session.greetedAt = dateTimeFormat(now(), "yyyy-mm-dd HH:nn:ss");
    }

    // Bump visit count only once we know the user — anonymous visits
    // stay zero-cost (no KV write, no cookie).
    if (structKeyExists(session, "greetName")) {
        session.visits = (session.visits ?: 0) + 1;
    }
</cfscript>
<cfinclude template="includes/header.cfm">
<cfoutput>

<div class="panel">
    <div class="panel-header">What this page does</div>
    <div class="panel-body">
        <p>A tiny "remember-me" flow. On the first visit you'll see a
        form asking for your name. Submitting it writes
        <code>session.greetName</code>, which triggers lazy-init: the
        engine inserts the session record into KV, fires
        <code>onSessionStart</code>, and sends back a <code>Set-Cookie</code>
        header. From then on, every visit hydrates the session from KV
        and greets you back.</p>
        <p>If you never submit the form, nothing is persisted, no cookie
        is issued, and the next request is just as cheap as the first.</p>
    </div>
</div>

<cfif structKeyExists(session, "greetName")>
    <div class="panel">
        <div class="panel-header">Welcome back</div>
        <div class="panel-body">
            <p class="hero">&##x1F44B; Hello again, #encodeForHTML(session.greetName)#!</p>
            <dl class="kv">
                <dt>session.greetName</dt><dd>#encodeForHTML(session.greetName)#</dd>
                <dt>session.greetedAt</dt><dd>#session.greetedAt#</dd>
                <dt>session.createdAt</dt><dd>#session.createdAt#</dd>
                <dt>session.visits</dt><dd>#session.visits#</dd>
            </dl>
            <form method="POST" class="forget-form">
                <input type="hidden" name="forget" value="1">
                <button type="submit" class="btn btn-quiet">Forget me</button>
            </form>
        </div>
    </div>
<cfelse>
    <div class="panel">
        <div class="panel-header">Who are you?</div>
        <div class="panel-body">
            <p>No <code>greetName</code> in session yet — looks like this
            is your first visit (or you cleared the cookie). Enter a name
            and we'll remember you.</p>
            <form method="POST" class="greet-form">
                <input type="text" name="name" placeholder="Your name" required class="text-input">
                <button type="submit" class="btn btn-primary">Greet me</button>
            </form>
        </div>
    </div>
<cfif structKeyExists(cookie, "CFID")>
    <div class="panel">
        <div class="panel-header">Cookie present, no session record</div>
        <div class="panel-body">
            <p>You're carrying a <code>CFID</code> cookie but the engine
            couldn't find a matching record in KV — either it expired
            (the cron tidies them every 30 minutes past timeout) or you
            cleared it client-side. Submitting the form will reuse the
            existing cookie value.</p>
        </div>
    </div>
</cfif>
</cfif>

<div class="panel">
    <div class="panel-header">Cookie + scope state right now</div>
    <div class="panel-body">
        <dl class="kv">
            <dt>cookie.CFID</dt><dd>#cookie.CFID ?: "(none)"#</dd>
            <dt>cgi.request_method</dt><dd>#cgi.request_method#</dd>
            <dt>form keys</dt><dd>#structKeyList(form)#</dd>
        </dl>
    </div>
</div>

</cfoutput>
<cfinclude template="includes/footer.cfm">
