# cfml-worker JSPI shim

These files are the JavaScript half of the cfml-worker JSPI bridge that lets
`<cfquery datasource="my-d1">` in CFML talk to an async Cloudflare D1
binding through a synchronous-looking call from the wasm VM.

## What's in here

| File | Role |
|---|---|
| `cfml-jspi-shim.mjs` | Reusable JS module — provides `jspiImports()`, `wrapWithPromising()`, `enterRequest()`, `leaveRequest()`. Copy into your worker project (or symlink from `node_modules`). |
| `jspi-template-shim.mjs` | Drop-in `wrangler.toml main` candidate that demonstrates the full wiring. Copy + adjust paths. |

## How it works

JSPI (JavaScript Promise Integration) is a V8 WebAssembly feature, enabled
by default on Cloudflare Workers. The deal:

- A wasm-imported JS function wrapped in `new WebAssembly.Suspending(...)`
  can `await` internally. When wasm calls it, the wasm stack literally
  suspends, the JS event loop resolves the Promise, then wasm resumes.
- For wasm to be *allowed* to call a suspending import, its exported entry
  point must be wrapped with `WebAssembly.promising(...)`.

In cfml-worker:

1. The Rust side declares `extern "C" fn cfml_jspi_d1_query(req_ptr, req_len) -> i64`
   in [`src/jspi.rs`](../src/jspi.rs).
2. This shim supplies the JS implementation, awaits the D1 call inside
   `Suspending`, and packs the JSON response into a wasm-allocated buffer.
3. The Rust [`D1Driver`](../src/d1_driver.rs) parses the JSON response and
   returns a normal CFML query value.

From the CFML programmer's view, `cfquery` blocks until the result is back —
exactly the semantics CFML mandates.

## Quick start

1. Copy `cfml-jspi-shim.mjs` and `jspi-template-shim.mjs` into your worker
   project's `src/` directory.
2. In `wrangler.toml`, point `main` at the template:
   ```toml
   main = "src/jspi-shim.mjs"
   ```
   (Adjust the filename to taste — `jspi-template-shim.mjs` works too.)
3. In your `wrangler.toml`, declare the D1 binding under whatever name you
   want to use in CFML:
   ```toml
   [[d1_databases]]
   binding = "MAIN"
   database_name = "myapp"
   database_id = "..."
   ```
4. In your worker's Rust entry, add the D1 datasource to `WorkerConfig`:
   ```rust
   if let Ok(db) = env.d1("MAIN") {
       config.d1_datasources.push(("main".into(), std::sync::Arc::new(db)));
   }
   ```
   The string `"main"` is what CFML will reference as
   `<cfquery datasource="main">`. The binding lookup is case-insensitive
   (`MAIN`, `Main`, `main` all match the binding).
5. Run your normal build (`worker-build --release` or equivalent). The
   template shim imports the wasm directly from `build/worker/index.wasm`
   — if your build outputs elsewhere, adjust the `wasmModule` import.

## Limitations

- **wasm-bindgen JSPI support is in progress**
  ([rustwasm/wasm-bindgen#3633](https://github.com/rustwasm/wasm-bindgen/issues/3633)).
  We use raw `extern "C"` declarations rather than `#[wasm_bindgen]`. If
  your wasm-bindgen-generated import set conflicts with our `env.*`
  imports, you may need to namespace differently.
- **Errors don't unwind cleanly across suspension**. A rejected D1 promise
  is captured by the JS shim and surfaced as `{success: false, error}` in
  the JSON response. Anything weirder (an exception inside the shim
  itself) returns a null pointer; the Rust side maps both to a
  `CfmlError`.
- **Per-call overhead**. JSPI suspend/resume is real V8 machinery; not
  free, but cheap compared to the latency of any actual DB call.
- **Single in-flight request per isolate**. The shim stores the current
  request env in a module-scoped variable. Workers' isolate model already
  guarantees one request at a time, so this is fine.

## Without a JSPI shim

If `cfml-jspi-shim.mjs` is not wired into your entry, every D1 cfquery
returns a `CfmlError`:

```
cfquery (D1 'main'): host JSPI shim returned null —
check that the JS shim has been wired into wrangler.toml's main entry
```

That's the signal to come back here.
