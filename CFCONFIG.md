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

Accepted by the parser for forward compatibility with shared Lucee/BoxLang
configs, but **not applied at runtime** — RustCFML uses an in-process cache
and has no alternative session backend.

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
