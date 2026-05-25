/*
 * jspi-template-shim.mjs — reference worker entry that wires JSPI into a
 * worker-rs / worker-build wasm build.
 *
 * Copy this into your worker project, adjust the `WASM_URL` import, and
 * point `wrangler.toml` at it via `main = "src/jspi-shim.mjs"`. The
 * `[build]` section that runs `worker-build` stays as-is — we consume the
 * `.wasm` it emits, but supply our own JS shim.
 *
 * If you instead want to keep worker-build's generated shim and *augment*
 * it, see `patch-worker-build-shim.mjs` in the same directory.
 *
 * Why this exists: Cloudflare D1 is async-only, but `<cfquery>` and
 * `queryExecute()` dispatch synchronously inside the CFML VM. JSPI bridges
 * the two — the wasm stack suspends on a Suspending import, the JS event
 * loop awaits the D1 promise, then wasm resumes. From CFML's perspective
 * cfquery just blocks the way it does on a native engine.
 */

import { jspiImports, wrapWithPromising, enterRequest, leaveRequest } from "./cfml-jspi-shim.mjs";

// worker-build emits the wasm under build/worker/index.wasm. If you use a
// different output path, adjust this import.
import wasmModule from "../build/worker/index.wasm";

// Lazily instantiate. We share one instance per isolate.
let instancePromise = null;

async function getInstance() {
  if (!instancePromise) {
    instancePromise = WebAssembly.instantiate(wasmModule, {
      // Merge our JSPI imports with whatever wasm-bindgen / worker-rs needs.
      // If worker-build's shim provides additional imports, replicate them
      // here. The minimum required set for cfml-worker + worker-rs is
      // empty besides the JSPI hook, but your build may produce others.
      ...jspiImports(),
    });
  }
  return instancePromise;
}

export default {
  async fetch(request, env, ctx) {
    const instance = await getInstance();
    const exports = instance.exports;
    const memory = exports.memory;

    enterRequest(env, memory, exports);
    try {
      // worker-rs exports its fetch handler under the name `fetch` after
      // wasm-bindgen mangling. Wrap with promising so suspending imports
      // are legal inside this call.
      const fetchExport = exports.fetch ?? exports.handler ?? exports.default;
      if (!fetchExport) {
        return new Response(
          "cfml-jspi-shim: could not find wasm-exported fetch handler",
          { status: 500 },
        );
      }
      const promisedFetch = wrapWithPromising(fetchExport);
      return await promisedFetch(request, env, ctx);
    } finally {
      leaveRequest();
    }
  },
};
