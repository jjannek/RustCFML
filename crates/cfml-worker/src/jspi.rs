//! Sync-looking bridge to async Cloudflare Worker APIs via JSPI
//! (JavaScript Promise Integration).
//!
//! Cloudflare Workers exposes every I/O API (D1, KV, R2, fetch) as
//! Promise-returning JS calls. JSPI is a V8 WebAssembly feature that lets a
//! wasm-imported JS function be marked `WebAssembly.Suspending`: when wasm
//! invokes it, the wasm stack literally suspends, the JS event loop drives
//! the Promise to completion, and wasm resumes — appearing fully synchronous
//! to the caller.
//!
//! ## Why this is a wasm-bindgen snippet, not a raw `extern "C"`
//!
//! An earlier revision declared `cfml_jspi_d1_query` in a plain `extern "C"`
//! block. The Rust toolchain emits that as a wasm import on the conventional
//! `"env"` module, which works fine for `WebAssembly.instantiate` but blows
//! up modern `worker-build` + `esbuild`: wasm-bindgen lifts every wasm import
//! into a JS `import` statement, and there is no JS module called `"env"` for
//! esbuild to resolve.
//!
//! Routing the import through a wasm-bindgen **snippet** sidesteps this:
//! wasm-bindgen copies `cfml_jspi.js` into its output `snippets/` directory
//! and emits an import path esbuild can actually resolve. For
//! primitive-typed externs (u32 / i32) wasm-bindgen does not add a marshalling
//! wrapper — the snippet's exported value (which can be a
//! `WebAssembly.Suspending` object) is what wasm calls directly.
//!
//! ## Wire protocol
//!
//! Rust → JS (suspending import):
//! ```text
//! cfml_jspi_d1_query(req_ptr, req_len, resp_ptr, resp_cap) -> i32
//! ```
//! - `req_ptr` / `req_len`: UTF-8 JSON `{datasource, sql, params}` in wasm
//!   linear memory.
//! - `resp_ptr` / `resp_cap`: caller-allocated wasm buffer the JS side
//!   writes the response JSON into.
//! - Return: positive = bytes written; negative = `-required_capacity`
//!   (caller should retry with a larger buffer); zero = host shim is not
//!   installed.
//!
//! Rust pre-allocating the response buffer means we no longer need
//! `cfml_jspi_alloc` / `cfml_jspi_free` wasm exports — the host shim never
//! has to round-trip through Rust to acquire memory.
//!
//! ## Init handshake
//!
//! The Suspending callback needs `wasm.memory` to read/write the buffers.
//! `wasm-bindgen`'s `start` hook calls `__cfml_jspi_set_memory(...)` once at
//! instantiation, handing the wasm memory object to the snippet. The host
//! worker entry point (`jspi-bootstrap.mjs` shipped with the worker
//! template) separately calls `globalThis.__cfmlJspi.setEnv(env)` before
//! each fetch so the suspending callback knows which D1 binding to use.

#![cfg(target_arch = "wasm32")]

use cfml_common::vm::CfmlError;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "/src/cfml_jspi.js")]
extern "C" {
    /// Hand the wasm `memory` export to the snippet so it can build Uint8Array
    /// views over linear memory inside the suspending callback.
    fn __cfml_jspi_set_memory(memory: JsValue);

    /// Suspending import — registered as `new WebAssembly.Suspending(async …)`
    /// in `cfml_jspi.js`. From wasm's perspective this is a normal sync call
    /// returning an `i32` byte-count.
    fn cfml_jspi_d1_query(
        req_ptr: u32,
        req_len: u32,
        resp_ptr: u32,
        resp_cap: u32,
    ) -> i32;
}

#[wasm_bindgen(start)]
fn __cfml_worker_jspi_start() {
    __cfml_jspi_set_memory(wasm_bindgen::memory());
}

/// Initial response-buffer capacity. Anything larger triggers a retry with
/// a freshly-sized buffer; small enough that the common case (a few hundred
/// bytes of JSON) fits in one allocation.
const INITIAL_RESPONSE_CAP: usize = 64 * 1024;

/// Invoke the suspending import, returning the response JSON. Retries once
/// with a larger buffer if the first call signals overflow.
pub(crate) fn d1_query_sync(request_json: &str) -> Result<String, CfmlError> {
    let req_bytes = request_json.as_bytes();
    let mut buf: Vec<u8> = vec![0u8; INITIAL_RESPONSE_CAP];

    let written = cfml_jspi_d1_query(
        req_bytes.as_ptr() as u32,
        req_bytes.len() as u32,
        buf.as_mut_ptr() as u32,
        buf.len() as u32,
    );

    if written == 0 {
        return Err(CfmlError::runtime(
            "cfquery (D1): host JSPI shim returned null — \
             check that the worker entry point imports cfml-jspi-bootstrap"
                .to_string(),
        ));
    }

    let written = if written < 0 {
        let required = (-written) as usize;
        buf = vec![0u8; required];
        let retry = cfml_jspi_d1_query(
            req_bytes.as_ptr() as u32,
            req_bytes.len() as u32,
            buf.as_mut_ptr() as u32,
            buf.len() as u32,
        );
        if retry <= 0 {
            return Err(CfmlError::runtime(
                "cfquery (D1): retry with larger response buffer also failed"
                    .to_string(),
            ));
        }
        retry as usize
    } else {
        written as usize
    };

    buf.truncate(written);
    String::from_utf8(buf).map_err(|e| {
        CfmlError::runtime(format!(
            "cfquery (D1): host JSPI shim returned non-UTF-8 response: {}",
            e
        ))
    })
}
