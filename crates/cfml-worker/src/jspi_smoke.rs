//! JSPI smoke test — a sync wasm export that calls the Hyperdrive Suspending
//! import on a synthetic query. Lives outside `handle_fetch` so the host
//! can invoke it via a `WebAssembly.promising` wrapper from a clean
//! contiguous wasm stack — i.e., without going through the
//! `wasm-bindgen-futures`-driven `#[event(fetch)]` machinery that breaks
//! JSPI's contiguous-stack requirement.
//!
//! Wire it up:
//!
//!   1. Host's post-build patch wraps `wasm.cfml_worker_jspi_smoke` in
//!      `WebAssembly.promising` and stashes the result on
//!      `globalThis.__cfmlJspi.smoke`.
//!   2. The Worker JS entrypoint, on receiving a request for a designated
//!      smoke path, calls `globalThis.__cfmlJspi.smoke(datasource_ptr,
//!      datasource_len, sql_ptr, sql_len)` instead of dispatching to the
//!      regular fetch handler, awaits the returned Promise, and serialises
//!      the response.
//!
//! The result is written back into a thread-local so the host can pull it
//! out post-suspend without us needing to fight the wasm-bindgen ABI for
//! arbitrary return values.

#![cfg(target_arch = "wasm32")]

use std::cell::RefCell;

thread_local! {
    static SMOKE_RESULT: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Synchronous body of the JSPI smoke test. No args, hardcoded query —
/// keeps the wasm signature trivial so we don't need `__wbindgen_malloc`
/// (which wasm-opt strips when it's otherwise unreferenced) for marshalling
/// strings across the JS boundary. The host re-exports this behind a
/// `#[wasm_bindgen]` function; the post-build patch wraps the resulting
/// wasm export in `WebAssembly.promising` and exposes it on
/// `globalThis.__cfmlJspi.smoke`.
pub fn cfml_worker_jspi_smoke() {
    let request = r#"{"datasource":"HYPERDRIVE_PG","sql":"SELECT 1 AS one, 'jspi-smoke' AS who","params":[]}"#;

    let result = match crate::jspi::hyperdrive_query_sync(request) {
        Ok(s) => s,
        Err(e) => format!(
            "{{\"success\":false,\"error\":\"smoke: hyperdrive_query_sync errored: {}\"}}",
            e.message.replace('"', "\\\"")
        ),
    };

    SMOKE_RESULT.with(|cell| *cell.borrow_mut() = Some(result));
}

/// Pull the smoke-test result out of the thread-local. Returns the JSON
/// string; the host re-exports under `#[wasm_bindgen]` for JS access.
pub fn cfml_worker_jspi_smoke_take() -> String {
    SMOKE_RESULT
        .with(|cell| cell.borrow_mut().take())
        .unwrap_or_else(|| "{\"success\":false,\"error\":\"smoke: no result captured\"}".to_string())
}
