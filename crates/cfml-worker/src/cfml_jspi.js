/*
 * cfml-worker JSPI snippet — wasm-bindgen-managed.
 *
 * This file is pulled into wasm-bindgen's output `snippets/` directory and
 * imported by the generated `index_bg.js`. It supplies the suspending
 * D1 callback the wasm side declares in `src/jspi.rs`.
 *
 * Three responsibilities:
 *
 *   1. Export `cfml_jspi_d1_query` as a `WebAssembly.Suspending` so calls
 *      from wasm can `await` D1 internally and resume sync-from-wasm.
 *
 *   2. Cache the wasm `memory` object once (handed in by the wasm-bindgen
 *      `start` function) so the suspending callback can build Uint8Array
 *      views over linear memory.
 *
 *   3. Expose `globalThis.__cfmlJspi.setEnv` for the host worker entry
 *      point (the "bootstrap" shim) to register the per-request bindings
 *      env. The suspending callback uses it to look up D1 by datasource
 *      name.
 *
 * The host entry point must additionally wrap the wasm-exported fetch with
 * `WebAssembly.promising(...)`; without that wrap, V8 refuses to call a
 * Suspending import. See `cfml-worker/jspi/README.md`.
 */

let wasmMemory = null;
let activeEnv = null;

/**
 * Called once from a wasm-bindgen `start` function so the snippet knows
 * where wasm linear memory lives. Also installs the public per-request
 * setter on `globalThis` for the host shim to use.
 */
export function __cfml_jspi_set_memory(memory) {
  wasmMemory = memory;
  if (!globalThis.__cfmlJspi) {
    globalThis.__cfmlJspi = {};
  }
  globalThis.__cfmlJspi.setEnv = (env) => {
    activeEnv = env;
  };
  globalThis.__cfmlJspi.clearEnv = () => {
    activeEnv = null;
  };
}

function lookupD1Binding(env, name) {
  // Workers convention is UPPERCASE binding names, but match case-insensitively.
  return (
    env[name] ||
    env[name.toUpperCase()] ||
    env[name.toLowerCase()] ||
    null
  );
}

async function runD1Query(req) {
  if (!activeEnv) {
    return {
      success: false,
      error:
        "cfml-jspi: no active env — host did not call globalThis.__cfmlJspi.setEnv(env) before fetch",
    };
  }
  const binding = lookupD1Binding(activeEnv, req.datasource);
  if (!binding || typeof binding.prepare !== "function") {
    return {
      success: false,
      error: `cfml-jspi: no D1 binding named "${req.datasource}" on env`,
    };
  }
  try {
    let stmt = binding.prepare(req.sql);
    if (Array.isArray(req.params) && req.params.length > 0) {
      stmt = stmt.bind(...req.params);
    }
    const result = await stmt.all();
    return {
      success: result.success !== false,
      results: Array.isArray(result.results) ? result.results : [],
      meta: result.meta || {},
    };
  } catch (e) {
    return { success: false, error: String(e?.message ?? e) };
  }
}

/**
 * Write a JSON response into the caller-provided wasm buffer. Returns the
 * number of bytes written, or `-required_capacity` if `respCap` is too
 * small so the Rust side can retry with a bigger allocation.
 */
function writeResponse(responseObj, respPtr, respCap) {
  const bytes = new TextEncoder().encode(JSON.stringify(responseObj));
  if (bytes.length > respCap) {
    return -bytes.length;
  }
  new Uint8Array(wasmMemory.buffer, respPtr, bytes.length).set(bytes);
  return bytes.length;
}

/**
 * The wasm import — a `WebAssembly.Suspending` so we can `await` D1 inside
 * while looking synchronous from wasm's perspective.
 *
 * Signature (matches `src/jspi.rs::cfml_jspi_d1_query`):
 *
 *   (reqPtr: u32, reqLen: u32, respPtr: u32, respCap: u32) -> i32
 */
export const cfml_jspi_d1_query = new WebAssembly.Suspending(
  async (reqPtr, reqLen, respPtr, respCap) => {
    if (!wasmMemory) {
      // Init never ran — the wasm-bindgen start function failed or this
      // snippet was loaded in a context that did not call into wasm.
      return 0;
    }

    let request;
    try {
      const bytes = new Uint8Array(wasmMemory.buffer, reqPtr, reqLen);
      request = JSON.parse(new TextDecoder().decode(bytes));
    } catch (e) {
      return writeResponse(
        { success: false, error: `cfml-jspi: bad request JSON: ${e.message}` },
        respPtr,
        respCap,
      );
    }

    const response = await runD1Query(request);
    return writeResponse(response, respPtr, respCap);
  },
);
