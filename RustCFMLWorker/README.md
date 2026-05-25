# rustcfml-worker

Reference Cloudflare Workers host wiring for [`cfml-worker`](../crates/cfml-worker).

Demonstrates:

- `<cfquery>` against Cloudflare D1 (sync-from-wasm via JSPI).
- KV-backed session storage.
- Durable Object–backed application scope with strong consistency.
- Cron-driven KV tidy-up that deletes expired session blobs on a
  configurable schedule (`onSessionEnd` is **not** supported in the
  Cloudflare host — see notes below).

## Layout

| Path | Purpose |
|---|---|
| `src/lib.rs` | Worker entry — `#[event(fetch)]`, `#[event(scheduled)]`, `#[durable_object] ApplicationScopeDO`. |
| `build.rs` | Walks `cfml/` at build time and emits a static `CFML_FILES` table. |
| `cfml/Application.cfc` | Sample app demonstrating `onApplicationStart`, `onSessionStart`, `onSessionEnd`. |
| `cfml/index.cfm` | Sample page reading from session + application scope. |
| `wrangler.toml` | Bindings + cron trigger. Edit the `<paste-id-here>` placeholders. |

## Setup

1. Install `wrangler` and `worker-build`:
   ```bash
   npm i -g wrangler
   cargo install worker-build
   ```
2. Provision KV namespaces:
   ```bash
   wrangler kv namespace create SESSIONS   # paste the returned id into wrangler.toml
   wrangler kv namespace create APP
   ```
3. (Optional) Provision a D1 database:
   ```bash
   wrangler d1 create rustcfml-demo
   ```
   Uncomment the `[[d1_databases]]` block in `wrangler.toml` and paste the id.
4. Deploy:
   ```bash
   wrangler deploy
   ```

## Session tidy-up cadence

The `[triggers] crons` entry in `wrangler.toml` controls how often the
worker sweeps expired session blobs from KV. The default is
`*/30 * * * *` (every 30 minutes). Tighten or loosen it freely — the
only knock-on effect is timeliness of cleanup vs. KV `list` cost.

**`onSessionEnd` is deliberately not implemented.** Firing it from the
scheduled handler would require loading Application.cfc and spinning up
a VM per expired session, which is heavy and rarely needed in a
serverless deployment. If your app needs cleanup semantics:

- Make `onSessionStart` idempotent and recover from cold state there, or
- Write a CFML page that does the cleanup and hit it from a separate
  cron (e.g. via the `[triggers]` mechanism pointing at a fetch URL).

## Verifying DO-backed application scope

After deploy, hit the worker from two different regions in quick
succession (e.g. `curl --resolve` against two PoPs). `application.requestCount`
should stay monotonically increasing — that's strong consistency the
KV-only path can't guarantee.

## Notes

- This crate is **not** a workspace member because the host is wasm32-only;
  `cargo build` from the repo root will not pick it up. Build it from
  this directory.
- The `[build] command = "worker-build --release"` line handles wasm-bindgen
  output, JSPI wiring (`WebAssembly.promising`), and the JSPI snippet copy
  from `cfml-worker/src/cfml_jspi.js`. No hand-rolled bootstrap mjs needed.
- For multi-app deployments, append every `this.name` to `config.app_names`
  in `src/lib.rs`. Each application gets its own DO instance via
  `idFromName(<app_name>)`.
