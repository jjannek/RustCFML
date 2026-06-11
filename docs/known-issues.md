# Known Issues & Unsupported Behaviour

This document inventories behaviours that RustCFML **does not fully implement**, with
an emphasis on **silent no-ops** — settings or attributes that are accepted without
error but have no effect. Those are the dangerous ones: code relying on them appears
to work but silently doesn't.

Each item is tagged:

- 🔇 **silent** — accepted, no error, no effect (the priority list to make overt)
- 🛑 **errors** — fails loudly with a clear message (safe, just unsupported)
- 🌍 **environment** — unsupported only on a specific target (e.g. wasm)
- 🏗 **by design** — intentional scoping decision (documented for clarity)

Compatibility target is Lucee/BoxLang. Items below are gaps against that target unless
marked *by design*.

> Maintenance: when you implement around a gap or skip an attribute/setting, add it
> here in the same change. See `docs/configuration.md` and `docs/status.md` for the
> positive "what's supported" view.

---

## 1. Application.cfc `this.*` settings — silently ignored 🔇

Read today: `this.name`, `this.mappings`, `this.sessionManagement`, `this.sessionTimeout`,
`this.customTagPaths`, `this.localMode`, `this.sessionStorage`, `this.cache`,
`this.lazySessionCreation`, `this.datasources`, `this.datasource`.

Accepted but **ignored** (no error, no effect):

| Setting | Notes |
|---|---|
| `this.timezone` | Per-app timezone ignored. Only the server/cfconfig `runtime.timezone` is honoured. |
| `this.locale` | Per-app locale ignored. Only cfconfig `runtime.locale` is honoured. |
| `this.applicationTimeout` | Per-app value ignored. cfconfig `runtime.applicationTimeout` IS applied. |
| `this.scriptProtect` | No script-protection filtering of scopes. |
| `this.secureJSON` / `this.secureJSONPrefix` | Per-app value ignored. cfconfig `security.secureJSON*` IS applied (process-global — see §4). |
| `this.nullSupport` / `this.enableNullSupport` | Per-app value ignored. cfconfig `runtime.nullSupport` IS applied. |
| `this.clientManagement`, `this.setClientCookies`, `this.setDomainCookies`, `this.clientStorage` | The **client scope is not implemented** at all. |
| `this.invokeImplicitAccessor` | Ignored. |
| `this.serialization`, `this.javaSettings`, `this.compileExtForCFCDirectory`, `this.blockedExtForFileUpload`, `this.triggerDataMember`, `this.sameFormFieldsAsArray`, `this.searchImplicitScopes`, `this.proxyServer`, `this.smtpServerSettings` | No references in the engine — accepted into the component, never consulted. |

Note: any unrecognised `this.X` is captured into an internal `config` map that is then
never read — so nothing throws, but nothing happens either.

## 2. Application.cfc lifecycle methods — not (fully) invoked 🔇

| Method | Status |
|---|---|
| `onApplicationStart`, `onApplicationEnd`, `onRequestStart`, `onRequest`, `onRequestEnd`, `onSessionStart`, `onSessionEnd` | ✅ invoked |
| `onError` | ⚠️ Partial — only fires when `onApplicationStart` throws, not as a general request-error handler. |
| `onMissingTemplate` | 🔇 Not invoked. (cfconfig front-controller `fallback` exists as an alternative.) |
| `onAbort` | 🔇 Not invoked on `<cfabort>` / `abort()`. |
| `onCFCRequest` | 🔇 Not invoked (no CFC-over-HTTP / remote method dispatch). |

## 3. `.cfconfig.json` keys — accepted but not enforced 🔇

These deserialize without error but have no runtime effect:

| Key | Notes |
|---|---|
| `server.maxConcurrentRequests` | No concurrency limiting. |
| `server.requestTimeout` | No per-request timeout enforcement. |
| `server.http2` | Not wired to the HTTP server. |
| `runtime.trustedCache` | Reserved; bytecode-cache trust is driven by `--production`, not this key. |
| `debugging.showExecutionTime` | No timing output. |
| `datasources[].connectionLimit` / `connectionTimeout` / `idleTimeout` / `timezone` | Pool tuning / per-DS timezone not applied. |
| `mailServers[].timeout` | Carried but not applied during send. |
| `caches[].properties.maxObjects` / `defaultTimeout` / `evictionPolicy` | In-memory cache capacity / TTL / eviction not enforced. |
| `logging.logsDirectory` | 🛑→🔇 Warns at startup ("not yet supported"); logs still go to stderr. |
| `logging.format` | Only `"text"`; other values warn and fall back. |
| `logging.loggers[].appender` | Logger name used; appender ignored. |

## 4. Per-application isolation gaps 🏗/🔇

`.cfconfig.json` is application-level (a file beside `Application.cfc` overlays the
server baseline — see `docs/configuration.md`). But some runtime registries are still
**process-global**, so per-app overrides of these do **not** isolate across apps:

| Area | Status |
|---|---|
| Datasources (`this.datasources` / cfconfig) | ✅ **Per-application** (resolved per request). |
| Security flags — `csrfEnabled`, `secureJSON`, `secureJSONPrefix` | 🔇 **Process-global** (`OnceLock`, set once at startup). Per-app override only changes the readable `server.cfconfig` struct, not enforcement. |
| Default mail server (`mailServers[0]`) | 🔇 **Process-global**. The `cfmail server=` attribute still works per-call. |

Making security flags and the default mail server per-application is a planned
follow-up (mirrors the datasource work).

## 5. Server-level keys are not application-level 🏗

The entire cfconfig `server.*` section (host, welcomeFiles, maxRequestBodySize, …) is a
**server/environment** concern and is intentionally **not** overlaid from a per-app
`.cfconfig.json`. There is deliberately **no `port` key** — the listening port is set
via `--port`; pages read `cgi.server_port`. (This is by design, not a gap.)

## 6. Functions / tags that error loudly when unsupported 🛑

These do **not** silently no-op — they throw a clear message (listed for completeness):

| Feature | Behaviour |
|---|---|
| `evaluate()` | Throws — not implemented (use bracket notation). |
| `structSetMetadata()` | Throws — ordered/case-sensitive struct metadata not supported. |
| `xmlTransform()` | Throws — no XSLT engine. |
| `xmlValidate()` | Throws — no schema-validation engine. |
| `<cfimport>` without `taglib` | Throws — Java/JSP class imports unsupported (custom-tag taglibs work). |
| `<cffile action="...">` outside the supported actions | Throws "not implemented". |
| `<cfthread action="...">` outside run/join/terminate | Throws "not supported". |
| Nested `<cftransaction>` | Throws — nesting unsupported. |

## 7. Partially-ignored parameters 🔇

| Function | Ignored argument(s) | Reason |
|---|---|---|
| `csrfGenerateToken(key, forceNew)` | `key`, `forceNew` | No server-side per-key session token storage. |
| `csrfVerifyToken(token, key)` | `key` | Same. |
| `fileSetAccessMode` / file mode setters | mode | No-op on non-Unix platforms. |
| `fileUpload()` / `fileUploadAll()` | — | Stub: returns `fileWasSaved=false` (needs form-scope wiring). |

## 8. Environment-specific 🌍

| Feature | Restriction |
|---|---|
| `<cfdirectory>` | Not supported on `wasm32` (no filesystem). |
| `<cfzip>` | Not supported on `wasm32`. |
| `<cflock>` | No-op in CLI mode (no server state); enforced in serve mode. |
| `<cfcache>` | No-op today (could emit Cache-Control in serve mode). |

## 9. Query-of-Queries — RustCFML/BoxLang superset 🏗

QoQ (`queryExecute(..., {dbtype:"query"})`) follows BoxLang and accepts SQL that **Lucee's
QoQ rejects**. Same query, *more* accepted — not a wrong-result divergence — but such SQL is
**not portable back to Lucee**:

| Feature | RustCFML | Lucee QoQ |
|---|---|---|
| `LIMIT n [OFFSET m]` | ✅ | ❌ (uses `SELECT TOP n`) |
| `CASE … WHEN … END` (searched + simple) | ✅ | ❌ |
| Scalar subquery in the SELECT list | ✅ | ❌ |
| Derived table `FROM (SELECT …) AS t` | ✅ | ❌ |
| Custom SQL functions (`queryRegisterFunction`) | ✅ | ❌ |

`SELECT TOP n`, `IN (SELECT …)`, all JOIN types, `UNION`, params, `LENGTH()` etc. work on both.
Cross-engine tests live in `tests/qoq/test_qoq_{select,aggregates,joins,subqueries_union}.cfm`
(green on both); superset-only coverage is probe-gated in `test_qoq_rustcfml_ext.cfm` /
`test_qoq_custom_functions.cfm` (skipped where unsupported).

**Correlated subqueries** (a subquery referencing the outer row) are **not** supported — subqueries
are executed once (uncorrelated); this matches typical QoQ usage. Errors loudly if a referenced
table/column is missing.

---

*This list is not exhaustive — it captures gaps identified to date. A periodic audit
sweep (e.g. parallel search for "not supported" / accepted-but-unused config keys /
ignored tag attributes) should refresh it.*
