/*
 * cfml-jspi-shim.mjs — JS half of the cfml-worker JSPI bridge.
 *
 * The wasm side declares a sync extern `cfml_jspi_d1_query(req_ptr, req_len)
 * -> i64`. This shim provides the JS implementation, wrapped with
 * `WebAssembly.Suspending` so it can `await` D1 internally while *looking*
 * synchronous to wasm.
 *
 * Three things must hook up for D1 cfquery to work end-to-end:
 *
 *   1. The wasm module is instantiated with the import object returned by
 *      `jspiImports()` MERGED into whatever wasm-bindgen produces.
 *
 *   2. The wasm-exported fetch handler is wrapped with
 *      `WebAssembly.promising(...)` (via `wrapWithPromising`) so the
 *      runtime knows it may suspend.
 *
 *   3. At the start of every request, the host calls `enterRequest(env,
 *      memory, exports)` to register the active environment. The suspending
 *      callback uses it to look up D1 bindings by datasource name. Call
 *      `leaveRequest()` after the response is built.
 *
 * See `jspi-template-shim.mjs` in this directory for a working reference
 * entry-point that puts it all together.
 */

let currentRequest = null;

/**
 * Register the active request's env + wasm-memory views. Called by the host
 * shim's fetch handler at the start of each request.
 *
 * The current implementation supports a single in-flight request per
 * isolate, which is the Workers model (a single request body executes
 * synchronously from wasm's perspective).
 */
export function enterRequest(env, memory, exports) {
  currentRequest = { env, memory, exports };
}

export function leaveRequest() {
  currentRequest = null;
}

function lookupD1Binding(env, datasourceName) {
  // Tried in order: exact name, UPPERCASE, lowercase. Workers convention is
  // UPPERCASE binding names but case is not enforced.
  return (
    env[datasourceName] ||
    env[datasourceName.toUpperCase()] ||
    env[datasourceName.toLowerCase()] ||
    null
  );
}

async function runD1Query(req) {
  const env = currentRequest?.env;
  if (!env) {
    return {
      success: false,
      error: "cfml-jspi-shim: no active request env — enterRequest() was not called",
    };
  }
  const binding = lookupD1Binding(env, req.datasource);
  if (!binding || typeof binding.prepare !== "function") {
    return {
      success: false,
      error: `cfml-jspi-shim: no D1 binding named "${req.datasource}" on env`,
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
    return {
      success: false,
      error: String(e?.message ?? e),
    };
  }
}

/**
 * Build the importObject fragment to merge into your wasm's instantiation
 * call. The values are `WebAssembly.Suspending` wrappers — they can only be
 * called from inside a `WebAssembly.promising` export.
 */
export function jspiImports() {
  return {
    env: {
      cfml_jspi_d1_query: new WebAssembly.Suspending(async (reqPtr, reqLen) => {
        const r = currentRequest;
        if (!r) return 0n;

        // Read request JSON out of wasm memory.
        const reqBytes = new Uint8Array(
          r.memory.buffer,
          Number(reqPtr),
          Number(reqLen),
        );
        let request;
        try {
          request = JSON.parse(new TextDecoder().decode(reqBytes));
        } catch (e) {
          return packResponse(r.exports, r.memory, {
            success: false,
            error: `cfml-jspi-shim: invalid request JSON: ${e.message}`,
          });
        }

        const response = await runD1Query(request);
        return packResponse(r.exports, r.memory, response);
      }),
    },
  };
}

function packResponse(exports, memory, responseObj) {
  const bytes = new TextEncoder().encode(JSON.stringify(responseObj));
  const ptr = exports.cfml_jspi_alloc(bytes.length);
  if (!ptr) return 0n;
  const view = new Uint8Array(memory.buffer, ptr, bytes.length);
  view.set(bytes);
  // Pack as i64: (ptr << 32) | len. Use BigInt because i64 isn't a native JS
  // number type. The wasm side decodes via simple shift+mask.
  return (BigInt(ptr) << 32n) | BigInt(bytes.length);
}

/**
 * Wrap any wasm-exported function that may transitively call a Suspending
 * import. Returns a function that returns a Promise.
 */
export function wrapWithPromising(wasmFunction) {
  return WebAssembly.promising(wasmFunction);
}
