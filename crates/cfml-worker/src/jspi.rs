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
//! This module declares the **wasm-side half** of that contract:
//!
//! - `cfml_jspi_d1_query`: a raw extern that the JS shim must register as a
//!   `new WebAssembly.Suspending(async (req_ptr, req_len) => { ... })`. The
//!   shim reads the JSON request from linear memory, awaits the D1 call,
//!   allocates a response buffer via `cfml_jspi_alloc`, writes the response
//!   JSON, and returns a packed `(ptr << 32) | len` `i64`.
//!
//! - `cfml_jspi_alloc` / `cfml_jspi_free`: wasm-exported helpers the JS
//!   shim uses to hand variable-sized responses back without us needing a
//!   ring-buffer or two-phase protocol.
//!
//! The full host-side wiring (the suspending import, `WebAssembly.promising`
//! around the fetch export) lives in a sibling `jspi-shim.mjs` ship.

#![cfg(target_arch = "wasm32")]

use cfml_common::vm::CfmlError;

unsafe extern "C" {
    /// Suspending import — looks sync, internally awaits a Promise on the
    /// JS side. The JS shim is responsible for wrapping this with
    /// `new WebAssembly.Suspending(...)` at instantiation time.
    ///
    /// Input: a UTF-8 JSON request `{datasource, sql, params}` at
    /// `req_ptr`..`req_ptr + req_len` in wasm linear memory.
    ///
    /// Output: packed `(response_ptr as i64) << 32 | (response_len as i64)`.
    /// The response bytes live in wasm memory allocated via
    /// `cfml_jspi_alloc` and must be freed via `cfml_jspi_free` after the
    /// caller has copied/parsed them.
    ///
    /// A zero return value (both ptr and len = 0) signals a host-side
    /// failure that couldn't be encoded as a JSON error response (e.g. the
    /// shim is missing or panicked). Callers should surface this as a
    /// generic runtime error.
    pub(crate) fn cfml_jspi_d1_query(req_ptr: *const u8, req_len: usize) -> i64;
}

/// Allocate `size` bytes inside wasm linear memory and return a pointer the
/// JS shim can write into. Memory is leaked from Rust's perspective until
/// `cfml_jspi_free` is called with the same `ptr` + `size`.
///
/// The JS shim **must** use this to hand back response bytes — naive use of
/// `wasm.memory` write at an arbitrary offset would clobber Rust-owned
/// allocations.
#[no_mangle]
pub extern "C" fn cfml_jspi_alloc(size: usize) -> *mut u8 {
    let mut v: Vec<u8> = Vec::with_capacity(size);
    let ptr = v.as_mut_ptr();
    // SAFETY: pair with `cfml_jspi_free`. We hand the capacity out as the
    // length the caller intends to write, so reconstruction in free is
    // (ptr, len=0, cap=size).
    std::mem::forget(v);
    ptr
}

/// Free a buffer previously returned by `cfml_jspi_alloc`. `size` must match
/// the size originally passed to `cfml_jspi_alloc` so the global allocator
/// can return the slab.
#[no_mangle]
pub extern "C" fn cfml_jspi_free(ptr: *mut u8, size: usize) {
    if ptr.is_null() || size == 0 {
        return;
    }
    // SAFETY: ptr was obtained from Vec::<u8>::with_capacity(size) and
    // mem::forget'd. Reconstructing the Vec with the same capacity and len=0
    // lets Drop run the deallocation.
    unsafe {
        let _ = Vec::<u8>::from_raw_parts(ptr, 0, size);
    }
}

/// Invoke the suspending import, reclaim the response buffer, return its
/// JSON contents as a `String`.
pub(crate) fn d1_query_sync(request_json: &str) -> Result<String, CfmlError> {
    let req_bytes = request_json.as_bytes();
    // SAFETY: the extern is JSPI-suspending; behaviour is defined provided
    // the JS shim is registered as `WebAssembly.Suspending`. From the wasm
    // side this is an ordinary blocking call.
    let packed: i64 =
        unsafe { cfml_jspi_d1_query(req_bytes.as_ptr(), req_bytes.len()) };

    if packed == 0 {
        return Err(CfmlError::runtime(
            "cfquery (D1): host JSPI shim returned null — \
             check that the JS shim has been wired into wrangler.toml's main entry"
                .to_string(),
        ));
    }

    let ptr = ((packed as u64) >> 32) as u32 as usize as *mut u8;
    let len = ((packed as u64) & 0xFFFF_FFFF) as usize;

    if ptr.is_null() || len == 0 {
        return Err(CfmlError::runtime(
            "cfquery (D1): host JSPI shim returned an empty response".to_string(),
        ));
    }

    // SAFETY: ptr/len describe wasm-memory bytes the JS shim wrote into via
    // `cfml_jspi_alloc`. We read them, then immediately free the buffer.
    let body = unsafe {
        let slice = std::slice::from_raw_parts(ptr, len);
        // Copy into an owned String — the underlying buffer is freed below
        // and we must not keep references to it.
        let s = std::str::from_utf8(slice)
            .map_err(|e| {
                CfmlError::runtime(format!(
                    "cfquery (D1): host returned non-UTF-8 response: {}",
                    e
                ))
            })?
            .to_string();
        cfml_jspi_free(ptr, len);
        s
    };

    Ok(body)
}
