# WebAssembly

[← Back to README](../README.md)

RustCFML compiles to WebAssembly via `wasm-bindgen`, so the same engine runs in the browser and on Cloudflare Workers.

## Browser

```bash
cargo install wasm-pack
wasm-pack build crates/wasm --target web
```

```javascript
import init, { CfmlEngine } from './pkg/rustcfml_wasm.js';
await init();
const output = CfmlEngine.new().execute('writeOutput("Hello from WASM!");');
```

The [interactive demo](https://rustcfml.github.io/RustCFML/demo/) is this WASM build running entirely in the browser.

## Cloudflare Workers

RustCFML runs on Cloudflare Workers at the edge. Database access uses [Hyperdrive](https://developers.cloudflare.com/hyperdrive/) bindings (PostgreSQL via `postgres.js`, MySQL via `mysql2`), and application/session state can use KV, R2, or Durable Objects. The Worker host integration lives in a separate repo:

- **[RustCFML-Cloudflare-worker](https://github.com/RustCFML/RustCFML-Cloudflare-worker)**

See **[Database](database.md#postgresql-on-cloudflare-workers)** for how `queryExecute` behaves against Hyperdrive datasources.

## Notes & limits

- The `wasm32-unknown-unknown` target does **not** build the host database drivers (the `postgres`/`mysql` crates use tokio transports that don't compile to wasm) — datasource access on Workers goes through Hyperdrive instead.
- **S3 is not yet available** in the WASM/Worker build — the AWS SDK uses tokio/hyper transport that doesn't compile on `wasm32-unknown-unknown`. For Workers, use the native R2 binding via the Worker host config. A `fetch()`-backed S3 transport for the WASM target is on the roadmap. See **[Object Storage](s3.md#wasm--cloudflare-workers)**.
