# Deployment

[← Back to README](../README.md)

RustCFML deploys as a single artifact in several shapes. Two cross-cutting capabilities — **production mode** (warm caching) and the **sandbox / virtual filesystem** — apply to both web and CLI deployments and are documented at the end of this page.

## Web application

Run the built-in server behind your reverse proxy. Enable production caching for deployment:

```bash
rustcfml --serve ./mywebroot --production
```

See **[Web Server](web-server.md)** for serve-mode details, `Application.cfc` lifecycle, sessions, and URL rewriting.

## Docker

*Coming soon* — an optimised, minimal container image for running RustCFML web applications. Until then, a standalone web-application binary (below) copied into a `scratch`/`distroless` base works well, since RustCFML has no runtime dependencies.

## CLI tools

Build a command-line tool from a CFML app. Arguments are available via the `cli` scope, which works like CFML's `arguments` scope — named keys for flags, 1-based numeric keys for positional args.

```bash
rustcfml --build ./myapp -o greet --mode cli --entry main.cfm
```

**myapp/main.cfm:**

```cfml
<cfscript>
name = cli.name ?: "World";
writeOutput("Hello, #name#!" & chr(10));

// Positional args: cli[1], cli[2], ...
for (i = 1; i <= structCount(cli); i++) {
    if (isNumeric(i) && structKeyExists(cli, i))
        writeOutput("  arg #i#: #cli[i]#" & chr(10));
}
</cfscript>
```

```bash
./greet                     # Hello, World!
./greet --name Alex         # Hello, Alex!
./greet foo bar             # positional: cli[1]="foo", cli[2]="bar"
```

## Self-contained web binaries

Package a web application as a single binary with an embedded HTTP server — no runtime dependencies, no source files to deploy.

```bash
rustcfml --build ./webapp -o myserver --mode serve
```

```bash
./myserver                          # Foreground on port 8500
./myserver --port 3000              # Custom port
./myserver start --port 3000        # Daemonize (background)
./myserver status                   # Check if running
./myserver stop                     # Graceful shutdown
```

### Binary sizes

| Build | Size |
|---|---|
| Release binary (no app) | ~13 MB |
| + small web app | ~13 MB |
| + large app (100+ files) | ~13–15 MB |

No JRE, no runtime, no dependencies. Compare: Lucee/BoxLang require a 200+ MB JRE.

### Native (Rust) modules

Self-contained binaries can include user-authored Rust code that surfaces as first-class CFML built-ins and classes. See **[Native Modules](native-modules.md)**.

## Cloudflare Workers

Run RustCFML at the edge by compiling to WebAssembly. The Worker integration (Hyperdrive datasources, KV/R2/Durable Objects, session storage) lives in a separate repo:

- **[RustCFML-Cloudflare-worker](https://github.com/RustCFML/RustCFML-Cloudflare-worker)**

See **[WebAssembly](wasm.md)** for the WASM target generally.

## Production mode (web and CLI)

By default the server re-validates files on each request: it walks up from the page directory to find `Application.cfc`, stats the resolved file, and stats every cached bytecode entry to detect source changes. This keeps the dev loop hot — edit a file and refresh.

Passing `--production` (or setting `RUSTCFML_PRODUCTION=1`) enables three in-memory caches that persist for the server's lifetime:

- **Application.cfc path resolution** — the first request walks the directory tree; subsequent requests hit a hashmap. Negative results (no `Application.cfc` anywhere in the chain) are cached too.
- **URL → file resolution** — `is_file` stats from request routing are memoized.
- **Bytecode cache trust** — the per-hit `mtime` check on every compiled file is skipped.

Net effect: requests pay zero filesystem IO once the cache is warm — typically a 3–4× throughput gain on an app with `Application.cfc` + cfincludes. Files added or modified on disk are not picked up until the server is restarted. See **[Performance](performance.md)** for measured numbers.

## Sandbox / virtual filesystem (web and CLI)

Self-contained binaries can run in **sandbox mode**, which completely isolates the application from the host filesystem. Sandbox mode also enables production caching automatically, since the embedded VFS is immutable at runtime.

```bash
./myserver --sandbox                # No host filesystem access
./myserver --sandbox --port 3000    # Sandbox + custom port
```

In sandbox mode:

- **Embedded files are readable** — `fileRead()`, `fileExists()`, `directoryList()`, `expandPath()`, and `include` all work against the embedded virtual filesystem. Your application can read its own bundled config files, templates, and assets normally.
- **Host filesystem is invisible** — `fileExists("/etc/passwd")` returns `false`; `fileRead()` on any host path returns "file not found". The application cannot discover or read files outside the embedded archive.
- **All writes are blocked** — `fileWrite()`, `fileAppend()`, `fileDelete()`, `directoryCreate()`, and other write operations throw *"filesystem writes are disabled in sandbox mode"*.

Even if application code is compromised (e.g. via a code-injection vulnerability), the attacker cannot read sensitive host files, write persistent backdoors/web shells, or modify host files. The embedded virtual filesystem is **read-only and non-persistent** — any state the application needs to persist should use external services (databases, APIs).
