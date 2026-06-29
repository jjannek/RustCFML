# Debugging

RustCFML ships the Adobe/Lucee-familiar **debug output footer** — the panel
appended to a page showing where a request spent its time: the queries it ran
(with bound parameters), every template it executed, exceptions, log/trace
entries, and the request scopes. It is modelled on Lucee 6/7's data model, so it
feels native and existing Lucee debug habits carry over.

The footer is **off by default** and gated so it never leaks to ordinary
visitors. It is built on an internal observability hook bus — the same
foundation later profiling/tracing layers build on — that costs nothing when no
debugger is attached.

> Built with the `observability` Cargo feature, which is on by default for the
> native binary and off for the WebAssembly/Worker build.

## Quick start

Enable it for local development by adding a `debugging` block to your
[`.cfconfig.json`](configuration.md):

```jsonc
{
  "debugging": {
    "enabled": true
  }
}
```

With just `enabled: true`, the footer renders for requests from `127.0.0.1` /
`::1` (the default `showFromIPs` whitelist). Hit any `.cfm` page and scroll past
the content — the panel is appended below it.

## Activation — the four gates

The footer renders only when **all four** of these pass (evaluated
cheapest/most-secure first; a request that fails any gate collects nothing and
allocates nothing):

1. **Enabled** — `debugging.enabled` is `true`.
2. **Viewer allowed** — the client IP is in `debugging.showFromIPs` **OR** the
   URL trigger matches (see below). This is the security gate: debug output
   leaks SQL, scope contents and file paths, so it is localhost-only by default
   and is enforced **identically in production**.
3. **Not suppressed by the page** — `<cfsetting showDebugOutput="false">` turns
   the footer off for that page (it can only turn it *off*, never bypass gates
   1–2). Non-HTML responses (JSON, binary, redirects) auto-suppress.
4. **Renderable** — there is an HTML/text response body to append to. Auto-render
   happens on web requests only; CLI runs still collect the data (so the BIFs
   below work) but don't get a panel appended to stdout.

### Running debug on a live site

Because the IP whitelist is honoured in production, the first-class way to debug
a live site is to leave `enabled: true` and restrict `showFromIPs` to your
office/VPN/ops addresses — every other visitor gets a normal page and never sees
the panel or any timing. Optionally add a secret URL trigger as a second path in.

### The URL trigger (a RustCFML enhancement)

Lucee core matches by IP only; RustCFML adds a **fully configurable** URL trigger
— both the variable *name* and its required *value* — which enables
security-by-obscurity:

```jsonc
"urlTrigger": {
  "enabled": true,
  "param": "myhiddenvar",   // the URL/form variable NAME (default "debug")
  "value": "s3cr3t-9f2a"    // required value (default "true"); set an unguessable secret
}
```

Then `?myhiddenvar=s3cr3t-9f2a` unlocks the footer for that request. An empty
`value` means presence-only (any value) — and is **refused in production mode**,
so a bare `?debug` can never expose a live site.

Behind a reverse proxy, set `trustForwardedFor` so the gate resolves the real
client IP rather than the proxy's (see the config reference below).

## What the footer shows

| Section | Contents |
|---|---|
| **Queries** | Each `queryExecute` / `<cfquery>`: name, execution time, recordcount, datasource, issuing `template:line`, the SQL, and the **bound parameters** (value + cfsqltype). |
| **Execution Time** | The request total, split into **Application** and **Query** time. |
| **Templates** | Every template executed — the requested page, each `<cfinclude>`, `Application.cfc` lifecycle methods, and CFC method calls — aggregated per file with total / app / query / count / avg. |
| **Exceptions** | Exceptions raised during the request (including ones caught by `try`/`catch`), with type, message and tag context. |
| **Trace / Log** | `writeLog` / `<cflog>` and `trace` / `<cftrace>` entries. |
| **Generic data** | App- and framework-injected panels (see `debugAdd` below). |
| **Scopes** | The configured request scopes (`cgi`, `url`, `form`, … — never `variables`/`local`). |

## Templates

Five built-in templates, selected with `debugging.template`:

- `modern` *(default)* — the rich HTML panel.
- `classic` / `simple` — plainer HTML tables.
- `comment` — an HTML `<!-- … -->` block (visible only in view-source), handy
  when a visible panel would disturb the layout.
- `none` — collect the data (so the BIFs work) but render no panel.

## BIFs

Available whenever gates 1–2 pass:

- **`getDebugData()`** → a struct with the sections above (`queries`, `pages`,
  `exceptions`, `traces`, `genericData`, `scopes`, `total`, …). Times are in
  microseconds. Use it to build a custom/AJAX debug view or feed your own tooling.
- **`isDebugMode()`** → boolean; `true` when the footer is active this request.
- **`debugAdd(category, name, value)`** or **`debugAdd(category, struct)`** →
  append rows to the **Generic data** section. The supported channel for app code
  and frameworks to inject their own debug panel.

```cfscript
if ( isDebugMode() ) {
    debugAdd( "MyApp", { controller: "users.index", cacheHit: false } );
}
```

`<cfsetting showDebugOutput="false">` suppresses the footer for the current page.

## Configuration reference

The `debugging` block in `.cfconfig.json` (Lucee-compatible; unknown keys are
ignored):

```jsonc
{
  "debugging": {
    "enabled": false,                          // master switch
    "showFromIPs": ["127.0.0.1", "::1"],       // the security gate — exact IPs allowed to see the footer
    "trustForwardedFor": false,                // reverse-proxy client-IP resolution:
                                               //   false  = use the socket peer (default)
                                               //   true   = trust X-Forwarded-For / X-Real-IP (foot-gun; only
                                               //            safe if your edge overwrites the header on ingress)
    "urlTrigger": {                            // RustCFML enhancement (Lucee matches by IP only)
      "enabled": true,
      "param": "debug",                        // the URL/form variable NAME — rename to hide it
      "value": "true"                          // required value; "" = presence-only (refused in production)
    },
    "template": "modern",                      // modern | classic | simple | comment | none
    "highlightMs": 250,                        // queries slower than this (ms) are highlighted red
    "maxRecords": 10,                          // rolling cap per section
    "fields": {                                // section toggles
      "database": true,
      "exception": true,
      "tracing": true,
      "timer": true,
      "dump": true,
      "scopes": ["cgi", "url", "form"]         // which scopes to dump (never variables/local)
    }
  }
}
```

## Notes & limitations

- The footer is a web-page artifact and auto-renders on web requests only; in
  CLI runs the data is still collected and reachable via `getDebugData()`.
- Per-template **Load** (compile/startup) time is not yet broken out separately —
  it folds into Application time.
- This footer is the first layer of a larger observability roadmap (a
  threshold-gated sampling profiler, OpenTelemetry traces + metrics, and a DAP
  step debugger) built on the same hook bus. Those layers are designed but not
  yet shipped.
