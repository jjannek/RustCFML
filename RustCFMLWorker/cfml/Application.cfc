component {
    this.name              = "rustcfml-worker-demo";
    this.sessionManagement = true;
    // Lazy session creation: no session record + no Set-Cookie until
    // CFML actually writes to `session.X`. Eliminates per-request KV
    // overhead for static-style pages.
    this.lazySessionCreation = true;
    this.sessionTimeout    = createTimeSpan(0, 0, 5, 0); // 5 minutes
    this.applicationTimeout = createTimeSpan(1, 0, 0, 0);

    function onApplicationStart() {
        application.startedAt = now();
        application.requestCount = 0;
        return true;
    }

    function onSessionStart() {
        session.createdAt = now();
        session.hits = 0;
    }

    // Note: onSessionEnd is NOT supported in the Cloudflare deployment.
    // The scheduled handler only tidies up expired session blobs in KV;
    // it does not load Application.cfc or invoke lifecycle methods.

    function onRequest(targetPage) {
        application.requestCount = (application.requestCount ?: 0) + 1;
        include "#targetPage#";
    }
}
