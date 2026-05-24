# `.cfconfig.json` — RustCFML Configuration

RustCFML reads an optional `.cfconfig.json` file at startup. The format follows
the [Ortus CFConfig](https://cfconfig.ortusbooks.com/) filename convention and
the BoxLang-style flat, declarative layout, so the same file can be shared with
CommandBox/Lucee and CommandBox/BoxLang projects — RustCFML silently ignores
any keys it doesn't recognise.

All fields are optional. When the file is missing, compiled-in defaults apply.

## File resolution

First match wins.

| Mode | Search order |
|---|---|
| `--serve` | webroot → cwd → directory of the `rustcfml` binary |
| CLI (`rustcfml file.cfm`) | entry file's directory → cwd → binary directory |
| `--build` self-contained binary | external file next to the binary → copy embedded into the VFS at build time → defaults |

CLI flags (`--port`, `--serve <path>`, `--sandbox`) always win over the file.
The config is read once at process startup; restart the server to pick up
changes.

## Environment variable substitution

Every string value supports `${env.VAR:default}` placeholders, expanded once
after parse:

```jsonc
"host":     "${env.DB_HOST:localhost}"     // env var with fallback
"password": "${env.DB_PASSWORD}"           // empty string if unset
```

Unknown namespaces (e.g. `${other.X}`) are left verbatim.

## HTTP protection

In web server mode, requests for `.cfconfig*`, `.env`, `*.lex`, and anything
matching `security.blockedPaths` return **HTTP 404** (not 403, to avoid
confirming the file's existence).

## Example

A realistic production file:

```json
{
  "server": {
    "host": "0.0.0.0",
    "port": 8080
  },
  "runtime": {
    "locale": "en-GB",
    "timezone": "Europe/London",
    "trustedCache": true
  },
  "datasources": {
    "myapp": {
      "driver":   "mysql",
      "host":     "${env.DB_HOST:localhost}",
      "port":     "${env.DB_PORT:3306}",
      "database": "${env.DB_NAME:myapp}",
      "username": "${env.DB_USER:root}",
      "password": "${env.DB_PASS}",
      "default":  true
    }
  },
  "mailServers": [
    {
      "smtp":     "${env.SMTP_HOST}",
      "port":     587,
      "username": "${env.SMTP_USER}",
      "password": "${env.SMTP_PASS}",
      "tls":      true
    }
  ],
  "mappings": {
    "/mylib": "/app/lib"
  },
  "debugging": {
    "enabled": false,
    "errorTemplate": "/errors/500.cfm"
  },
  "security": {
    "disallowedFunctions": ["cfexecute"]
  }
}
```

## Sections and keys

### `server`

| Key | Type | Default | Notes |
|---|---|---|---|
| `host` | string | `127.0.0.1` | Bind address. `0.0.0.0` = all interfaces |
| `port` | int | `8500` | Overridden by `--port` |
| `webroot` | string | `""` | Document root. Overridden by `--serve <path>` |
| `welcomeFiles` | string[] | `["index.cfm", "index.htm", "index.html"]` | Tried in order for directory requests |
| `cfmlExtensions` | string[] | `["cfm", "cfc"]` | Extensions dispatched through the interpreter |
| `maxRequestBodySize` | int (bytes) | `10485760` | `0` = unlimited |
| `maxConcurrentRequests` | int | `0` | `0` = unlimited (reserved; not enforced yet) |
| `requestTimeout` | int (sec) | `0` | `0` = no timeout (reserved; not enforced yet) |

### `runtime`

| Key | Type | Default | Notes |
|---|---|---|---|
| `nullSupport` | bool | `false` | Unset variables return null vs `""` |
| `dotNotationUpperCase` | bool | `true` | Force upper-case struct keys (classic CF) |
| `locale` | string | `""` | IETF BCP 47 (e.g. `en-GB`). Empty = system |
| `timezone` | string | `""` | IANA tz name. Empty = system |
| `whitespaceCompressionEnabled` | bool | `false` | Global `cfsetting enableCFOutputOnly=true` |
| `trustedCache` | bool | `false` | Skip recompile when template mtime unchanged |
| `applicationTimeout` | `"d,h,m,s"` | `"1,0,0,0"` | Application scope timeout |
| `sessionTimeout` | `"d,h,m,s"` | `"0,0,30,0"` | Session scope timeout |
| `clientTimeout` | `"d,h,m,s"` | `"7,0,0,0"` | Client scope timeout |

### `datasources`

Map of name → driver config. The name becomes the value used in
`cfquery datasource="name"` / `queryExecute(..., {datasource: "name"})`.

```jsonc
"datasources": {
  "myDSN": {
    "driver":   "mysql",          // mysql | mariadb | postgresql | postgres | mssql | sqlserver | sqlite
    "host":     "localhost",
    "port":     "3306",
    "database": "mydb",
    "username": "u",
    "password": "p",
    "connectionString": "",       // optional — overrides the synthesised URL
    "default": false              // when true, used when cfquery omits datasource
  }
}
```

`Application.cfc this.datasources` overrides global entries at application scope.

### `mappings`

```jsonc
"mappings": {
  "/mylib": "/var/www/shared/lib"
}
```

Layered underneath `Application.cfc this.mappings` — the app file wins on
conflict.

### `customTagPaths`

```jsonc
"customTagPaths": ["/var/www/tags"]
```

Searched after `Application.cfc this.customTagPaths`.

### `mailServers`

First entry becomes cfmail's default when its tag attributes omit `server`.

```jsonc
"mailServers": [
  {
    "smtp":     "smtp.example.com",
    "port":     587,
    "username": "u",
    "password": "p",
    "tls":      true,
    "ssl":      false,
    "timeout":  30
  }
]
```

### `logging`

| Key | Type | Default | Notes |
|---|---|---|---|
| `level` | string | `"warn"` | `error`/`warn`/`info`/`debug`/`trace`/`off` |
| `loggers.<name>.level` | string | — | Per-logger overrides (e.g. `datasource`) |
| `logsDirectory` | string | `""` | Reserved — currently logs always go to stderr |
| `format` | string | `"text"` | Reserved — JSON sink not yet implemented |

`RUST_LOG` and `--verbose` still take precedence.

### `debugging`

| Key | Type | Default | Notes |
|---|---|---|---|
| `enabled` | bool | `false` | When false, hides error detail from clients (server log keeps it) |
| `errorTemplate` | string | `""` | CFML template rendered for unhandled errors; receives `request._error` |
| `errorStatusCode` | bool | `true` | When false, error responses return 200 |
| `showExecutionTime` | bool | `false` | Reserved |

### `security`

| Key | Type | Default | Notes |
|---|---|---|---|
| `sandbox` | bool | `false` | Same as `--sandbox`: blocks host filesystem writes |
| `disallowedFunctions` | string[] | `[]` | Case-insensitive BIF/user-function names that are refused |
| `disallowedImports` | string[] | `[]` | Regex patterns blocking `createObject("component"\|"rust", ...)` |
| `blockedPaths` | string[] | `["*.cfm.bak","*.cfm~","Application.cfc","*.config.cfm"]` | URL globs returning 404 |
| `csrfEnabled` | bool | `true` | When false, `csrfGenerateToken` / `csrfVerifyToken` error out |
| `secureJSON` | bool | `false` | Prepend `secureJSONPrefix` to `serializeJSON` output |
| `secureJSONPrefix` | string | `"//"` | Hijack-prevention prefix |

### `urlRewriting`

| Key | Type | Default | Notes |
|---|---|---|---|
| `configFile` | string | `"urlrewrite.xml"` | Path to the rewrite rules (relative to webroot or absolute) |
| `enabled` | bool | `true` | Skip rewriting entirely when false |

### `caches` and `sessionStorage`

`sessionStorage` names a cache defined in `caches` that should back the session store.
`caches` is a map of named cache definitions, each with a `provider` and `properties` block.

**Supported providers:**

| `provider` | Description |
|-----------|-------------|
| `"memory"` | In-process store (default — no config needed) |
| `"memcached"` | External Memcached cluster |
| `"cluster"` | Gossip-based multi-node replication via memberlist + Automerge CRDT |

All three providers are built into the stock `rustcfml` binary — there is nothing to enable at build time. Each provider is dormant until a cache definition with the matching `provider` value is referenced as session storage in `.cfconfig.json` (or `this.sessionStorage` in `Application.cfc`).

**Example — Memcached (RustCFML native format):**
```json
{
    "sessionStorage": "mc",
    "caches": {
        "mc": {
            "provider": "memcached",
            "storage": true,
            "properties": {
                "servers": ["localhost:11211"],
                "keyPrefix": "myapp:sess:"
            }
        }
    }
}
```

**Lucee compatibility format** — if you export a `.cfconfig.json` from Lucee with the Memcached extension installed, it uses `class` instead of `provider` and a `custom` map with a space-separated `servers` string. RustCFML accepts this format directly:

```json
{
    "sessionStorage": "sessions",
    "caches": {
        "sessions": {
            "class": "org.lucee.extension.io.cache.memcache.MemCacheRaw",
            "storage": true,
            "custom": {
                "servers": "host1:11211 host2:11211",
                "storage_format": "Binary"
            }
        }
    }
}
```

Both Lucee Memcached class names are recognised:
- `org.lucee.extension.io.cache.memcache.MemCacheRaw` (Lucee 5 / early 6)
- `org.lucee.extension.cache.mc.MemcachedCache` (Lucee 6 current)

**Lucee notes:**
- The `storage: true` flag is required by Lucee for session-eligible caches. RustCFML emits a warning if it is absent but does not refuse.
- Lucee serialises sessions as binary Java objects; RustCFML serialises as JSON. Sessions written by one engine cannot be read by the other — they do not share session data in the same Memcached instance.
- Lucee has `sessionCluster: true/false` (`this.sessionCluster` in Application.cfc) to control whether reads are always pulled from the external store. RustCFML always reads from the store on each request.

**Example — Cluster (single-node config):**
```json
{
    "sessionStorage": "cluster",
    "caches": {
        "cluster": {
            "provider": "cluster",
            "storage": true,
            "properties": {
                "listenAddr": "0.0.0.0:7946",
                "advertiseAddr": "192.168.1.10:7946",
                "seeds": ["node2.internal:7946", "node3.internal:7946"],
                "nodeName": "node1"
            }
        }
    }
}
```

> **`storage: true` is required.** The cache must explicitly opt in to being used as session storage. Lucee enforces this; RustCFML warns if it is missing but uses the cache anyway.

Cluster properties:

| Property | Default | Description |
|----------|---------|-------------|
| `listenAddr` | `0.0.0.0:7946` | TCP `host:port` this node binds for memberlist gossip. Use `0.0.0.0` to bind every interface; restrict to a specific IP for tighter networking. |
| `advertiseAddr` | (empty) | Public address other nodes should reach this one on. Required when `listenAddr` binds `0.0.0.0`; leave empty when `listenAddr` already specifies a routable address. Also used as the default `nodeName`. |
| `seeds` | `[]` | List of peer `host:port` addresses to contact on startup. **Any single reachable seed is enough** to bootstrap — the new node will discover the rest via gossip. An empty list means "I am the first member." |
| `nodeName` | derived | Stable identifier used as the node's id. Defaults to `advertiseAddr`, or `listenAddr-<uuid>` when neither is set. Set this explicitly in production so a node keeps the same identity across restarts. |

### Three-node walkthrough

Three machines, all on the same internal network, all running `rustcfml --serve --port 8500`:

```jsonc
// On node1 (192.168.1.10) — .cfconfig.json
{
    "sessionStorage": "cluster",
    "caches": { "cluster": { "provider": "cluster", "storage": true,
        "properties": {
            "listenAddr":    "0.0.0.0:7946",
            "advertiseAddr": "192.168.1.10:7946",
            "seeds":         [],
            "nodeName":      "node1"
        } } }
}
```
```jsonc
// On node2 (192.168.1.11) — .cfconfig.json
{ "sessionStorage": "cluster",
  "caches": { "cluster": { "provider": "cluster", "storage": true,
    "properties": {
        "listenAddr":    "0.0.0.0:7946",
        "advertiseAddr": "192.168.1.11:7946",
        "seeds":         ["192.168.1.10:7946"],
        "nodeName":      "node2"
    } } } }
```
```jsonc
// On node3 (192.168.1.12) — .cfconfig.json
{ "sessionStorage": "cluster",
  "caches": { "cluster": { "provider": "cluster", "storage": true,
    "properties": {
        "listenAddr":    "0.0.0.0:7946",
        "advertiseAddr": "192.168.1.12:7946",
        "seeds":         ["192.168.1.10:7946", "192.168.1.11:7946"],
        "nodeName":      "node3"
    } } } }
```

Start order: any node can start first. Nodes whose seeds are unreachable at boot log a `partial join` warning, but the cluster heals automatically as the missing peers come up — periodic anti-entropy will pull the latest state in the next push/pull cycle.

Each node logs a single line on success, e.g. `[session/cluster] node 'node2' listening on 0.0.0.0:7946` plus `[session/cluster] joined 1 seed(s) successfully`.

### Firewalls and ports

The cluster uses **one TCP port per node** (the `listenAddr` port — 7946 by default, matching HashiCorp Serf's convention). Open it bidirectionally between every pair of cluster members. No additional UDP ports are needed in this build (the `tcp` feature is the only transport enabled).

Run multiple nodes on one host (e.g. for local testing) by giving each a distinct `listenAddr` port:
```bash
# node A on :7946, node B on :7947
```

### How it works

Each session is held in its own per-process [Automerge](https://automerge.org) document. On `set` / `remove`, the local document records a change and the incremental change bytes are reliably sent to every currently-online cluster member as a [memberlist](https://github.com/al8n/memberlist) user-message. On receive, the change is applied via Automerge's CRDT merge — concurrent writes converge deterministically across the cluster without coordination.

Membership and failure detection come from memberlist (the Rust port of HashiCorp's gossip protocol). On node join, memberlist's TCP push/pull state exchange invokes the cluster store's `local_state` hook on each side, round-tripping the union of all session documents — so a newly-joined node catches up to the cluster's full state immediately, and the same mechanism runs periodically thereafter as anti-entropy against any messages dropped on the live path.

### Sizing

Tested for native rustcfml server deployments up to a few dozen nodes on LAN or WAN. WASM and Cloudflare Workers **cannot** participate — memberlist requires a persistent TCP socket model unavailable in those runtimes.

### Troubleshooting

| Symptom | Likely cause | Fix |
|---------|--------------|-----|
| `[session/cluster] partial join — reached 0 seed(s); error: Connection refused` on every node | None of the seeds were running yet, **or** they aren't actually listening on `listenAddr`, **or** a firewall is blocking the port. | Start at least one seed first, double-check the `host:port` strings, open the port between the nodes. |
| Session set on node A is never visible on node B | Almost always: `nodeName` collision (two nodes share the same name, so memberlist sees them as the same node and ignores one). Less commonly: `advertiseAddr` is set to a value the peer can't actually reach. | Give every node a unique `nodeName`. Verify each `advertiseAddr` resolves and is reachable from every other node. |
| Sessions sometimes appear after a delay rather than immediately | Live `send_reliable` was dropped (network glitch). Anti-entropy will catch it on the next push/pull cycle (a few seconds). | Expected behaviour — the cluster is eventually consistent. If delays exceed ~10 s, investigate network or memberlist tuning. |
| A node's CFML test suite fails when the cluster is configured | Unlikely — the test runner uses CLI mode and never touches `build_session_store`. | If you actually see this, file a bug with the failing suite name. |

**Application.cfc override** — per-app session storage follows Lucee conventions:
```cfml
component {
    this.name            = "MyApp";
    this.sessionManagement = true;
    this.sessionStorage  = "mc";  // references a named cache

    this.cache["mc"] = {
        provider: "memcached",
        properties: { servers: ["localhost:11211"] }
    };
}
```

`this.cache` definitions merge with and override same-named entries from `.cfconfig.json`.
`this.sessionStorage` overrides the server-wide `sessionStorage` for this application.

## Inspecting the resolved config from CFML

The merged config is exposed as a read-only struct on the `server` scope:

```cfml
<cfscript>
writeOutput(server.cfconfig.server.port);
writeOutput(server.cfconfig.runtime.locale);
for (name in server.cfconfig.datasources) {
    writeOutput(name & " -> " & server.cfconfig.datasources[name].driver);
}
</cfscript>
```

Useful for debugging deploys and for templates that want to branch on
environment.

## Precedence summary

```
CLI flag  >  .cfconfig.json  >  compiled-in default
```

At application scope, `Application.cfc this.*` overrides the runtime,
datasource, mapping, and custom-tag-path layers from `.cfconfig.json`.
