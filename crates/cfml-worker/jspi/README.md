# cfml-worker JSPI bridge

Lets `<cfquery datasource="...">` and `queryExecute(...)` in CFML talk to an
async Cloudflare D1 binding from inside the synchronous CFML VM, via the V8
JSPI (JavaScript Promise Integration) feature.

## How it works

JSPI is a WebAssembly feature, enabled by default on Cloudflare Workers. The
contract:

- A wasm-imported JS function wrapped in `new WebAssembly.Suspending(...)`
  can `await` internally. When wasm calls it, the wasm stack suspends, the
  JS event loop resolves the Promise, then wasm resumes — looking sync to
  wasm.
- For wasm to be *allowed* to invoke a suspending import, the wasm export
  that triggered the call must be reached through a wrapper produced by
  `WebAssembly.promising(...)`.

In cfml-worker:

1. The Rust side declares the import as a **wasm-bindgen snippet**:
   ```rust
   #[wasm_bindgen(module = "/src/cfml_jspi.js")]
   extern "C" {
       fn cfml_jspi_d1_query(
           req_ptr: u32, req_len: u32,
           resp_ptr: u32, resp_cap: u32,
       ) -> i32;
   }
   ```
   wasm-bindgen copies the JS file into its output `snippets/` directory at
   build time and rewrites the import path so esbuild can resolve it. The
   snippet exports `cfml_jspi_d1_query` as a `WebAssembly.Suspending`, and
   wasm-bindgen passes the value through unchanged for primitive signatures
   — the Suspending object lands directly in the wasm import object.

2. The snippet (`src/cfml_jspi.js`) reads the request JSON out of wasm
   memory, awaits the D1 call, and writes the response JSON back into a
   wasm buffer the Rust side pre-allocated. No alloc/free round-trip
   required.

3. The Rust [`D1Driver`](../src/d1_driver.rs) decodes the response JSON and
   returns a normal CFML query value to the VM.

From the CFML programmer's view, `<cfquery>` blocks until the result is
back — exactly the semantics CFML mandates.

## What the host worker still has to do

The crate-side change handles import resolution and message marshalling.
The **host worker entry point** still owns two responsibilities:

### 1. Wrap the wasm fetch export with `WebAssembly.promising`

JSPI refuses to suspend if the call did not enter through a promising
wrapper. Concretely, your worker's `main` shim must:

```js
const instance = /* the wasm Instance from worker-build */;
const promisingFetch = WebAssembly.promising(instance.exports.fetch);

export default {
  async fetch(request, env, ctx) {
    // ... see step 2 ...
    return await promisingFetch(request, env, ctx);
  },
};
```

The exact plumbing depends on which version of `worker-build` you use and
whether its generated shim exposes the raw wasm exports object. See the
RustCFMLWorker template for a working reference.

### 2. Register the active env before each fetch

The suspending callback needs to look up the D1 binding on `env` by
datasource name. Register it before invoking the wasm fetch:

```js
globalThis.__cfmlJspi.setEnv(env);
try {
  return await promisingFetch(request, env, ctx);
} finally {
  globalThis.__cfmlJspi.clearEnv();
}
```

`__cfmlJspi.setEnv` / `__cfmlJspi.clearEnv` are installed on `globalThis`
by the snippet's wasm-bindgen `start` hook, so they are available as soon
as the wasm module is instantiated.

## Files

| File | Role |
|---|---|
| `../src/jspi.rs` | Rust side — declares the wasm-bindgen extern + the sync wrapper `d1_query_sync()`. |
| `../src/cfml_jspi.js` | The wasm-bindgen snippet — JS implementation of the Suspending import + globalThis env hooks. |

There are no longer any "drop-in" JS files to copy into your worker
project. The snippet ships with the crate and is bundled automatically by
wasm-bindgen.

## Limits & future work

- **Single in-flight env**: `globalThis.__cfmlJspi.setEnv` is a singleton.
  Workers' single-request-per-isolate model makes this safe today, but a
  future refactor may switch to per-request context tracking.
- **No streaming**: `stmt.all()` materialises the full result set before
  returning. For result sets larger than ~64KB the response buffer is
  retried once with a larger allocation; absurdly large queries should be
  paginated by the CFML caller.
- **D1 only for now**: the same Suspending pattern extends to any async
  Workers binding (R2, Queues, fetch service bindings). Each gets its own
  extern + Suspending callback; the snippet is the natural place to add
  them.
