//! Async kernel — Layer 0.
//!
//! Native runtime support for the `coldbox.system.async.*` port (WireBox/
//! ColdBox dependency). Three primitives:
//!
//! - `runAsync(closure)` — VM intercept; spawns the closure via the cfthread
//!   spawner, wraps the resulting `ThreadHandle` in a `FutureNative`, returns
//!   it as a `CfmlValue::NativeObject`. Inline-runs and returns a resolved
//!   `FutureNative` on wasm / when `real-threads` is off.
//!
//! - `_schedule(closure, delayMs[, everyMs|spacedMs])` — VM intercept; spawns
//!   a worker that sleeps `delayMs` then runs the closure (one-shot, fixed-
//!   rate, or fixed-delay-after-completion). Returns a `FutureNative` ticket
//!   whose `cancel()` flips the cancel flag.
//!
//! - `Future` — `impl CfmlNative` holding the `ThreadHandle` + cached result.
//!   Method dispatch goes through `call_member_function` in `lib.rs`.
//!
//! Critical: `CfmlNative::call_method` has no `&mut VM`. The native object's
//! `RwLock` is held in write mode for the entire call. So `get()` must take
//! ownership of its channel (`Option::take`), not block on shared locked
//! state — a second concurrent method call on the same Future would deadlock
//! otherwise.
//!
//! Anything that *runs* a CFML closure (composing futures, firing
//! continuations) cannot live here — those must be intercepted BIFs with
//! `&mut VM`. The CFML async port composes via `runAsync(() => cb(prev.get()))`
//! instead.

use crate::{ThreadHandle, ThreadResult};
use cfml_common::dynamic::{CfmlNative, CfmlValue};
use cfml_common::vm::{CfmlError, CfmlResult};
use indexmap::IndexMap;
use std::sync::Mutex;

/// A Future wrapping one spawned async task. Either holds a live
/// `ThreadHandle` (waiting/running) or a cached `ThreadResult` (resolved).
///
/// Identity is by `Arc` pointer-equality (the surrounding `Arc<RwLock<…>>`),
/// matching how other NativeObjects compare in the VM. The handle is
/// `Option::take`n on first `get()` so the receiver's ownership leaves the
/// locked struct before we block on it — avoids the documented re-entrancy
/// deadlock.
pub struct FutureNative {
    /// `ThreadHandle` holds an `mpsc::Receiver` which is `!Sync`; we wrap in
    /// a `Mutex` so `FutureNative` satisfies `CfmlNative: Sync`. In practice
    /// the surrounding `Arc<RwLock<dyn CfmlNative>>` already serializes
    /// method calls, so this inner lock is essentially uncontended.
    handle: Mutex<Option<ThreadHandle>>,
    result: Option<ThreadResult>,
    /// `True` when the task was inline-run (wasm / `real-threads` off) and
    /// the result was injected at construction. `cancel()` is a no-op then.
    inline_resolved: bool,
}

impl std::fmt::Debug for FutureNative {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FutureNative")
            .field("done", &self.result.is_some())
            .field(
                "status",
                &self.result.as_ref().map(|r| r.status.clone()).unwrap_or_default(),
            )
            .finish()
    }
}

impl FutureNative {
    /// Wrap a freshly-spawned `ThreadHandle` (the live, async path).
    pub fn from_handle(handle: ThreadHandle) -> Self {
        Self {
            handle: Mutex::new(Some(handle)),
            result: None,
            inline_resolved: false,
        }
    }

    /// Pre-resolved future (inline-run path: wasm / `real-threads` off).
    pub fn resolved(result: ThreadResult) -> Self {
        Self {
            handle: Mutex::new(None),
            result: Some(result),
            inline_resolved: true,
        }
    }

    /// Block until the underlying task completes (or `timeout_ms` elapses;
    /// 0 = forever). Returns:
    /// - `Ok(value)` — completed normally. The thread body's *return value*
    ///   is the future value. We can't recover that from `ThreadResult`
    ///   (which only carries status/output/error/thread_vars), so v1 returns
    ///   `thread.result` when the body set it, else the `thread` scope as a
    ///   struct, else Null.
    /// - `Err(...)` — the body threw; the error message is preserved.
    ///
    /// On timeout: leaves the handle in place and returns Null without
    /// error (matches `threadJoin` semantics — caller checks `isDone()`).
    fn await_result(&mut self, timeout_ms: i64) -> CfmlResult {
        if self.result.is_none() {
            // Take the handle out of the Mutex slot so we don't hold the
            // inner lock across the blocking recv. If we time out we put it
            // back; on completion it stays None.
            let mut taken: Option<ThreadHandle> = {
                let mut slot = self.handle.lock().unwrap();
                slot.take()
            };
            if let Some(mut handle) = taken.take() {
                let recv = if timeout_ms > 0 {
                    handle
                        .rx
                        .recv_timeout(std::time::Duration::from_millis(timeout_ms as u64))
                        .ok()
                } else {
                    handle.rx.recv().ok()
                };
                match recv {
                    Some(res) => {
                        if let Some(j) = handle.join.take() {
                            let _ = j.join();
                        }
                        self.result = Some(res);
                    }
                    None => {
                        // Timeout — restore the handle and return Null.
                        let mut slot = self.handle.lock().unwrap();
                        *slot = Some(handle);
                        return Ok(CfmlValue::Null);
                    }
                }
            }
        }
        let r = self.result.as_ref().unwrap();
        if !r.error.is_empty() {
            return Err(CfmlError::runtime(r.error.clone()));
        }
        // The closure's return value is the future value. Fall back to
        // `thread.result` (set inside the body), then the whole `thread`
        // scope, then Null — matches the convention CFML users expect.
        if let Some(v) = &r.return_value {
            if !matches!(v, CfmlValue::Null) {
                return Ok(v.clone());
            }
        }
        if let Some(v) = r.thread_vars.get("result") {
            return Ok(v.clone());
        }
        if !r.thread_vars.is_empty() {
            return Ok(CfmlValue::strukt(r.thread_vars.clone()));
        }
        Ok(CfmlValue::Null)
    }

    fn is_done(&mut self) -> bool {
        if self.result.is_some() {
            return true;
        }
        // Non-blockingly drain the channel: if the body has already
        // published, cache the result so subsequent get()/isDone calls are
        // O(1). Without this, `isDone()` would lie until someone called
        // `get()` and `anyOf`/poll-style loops would spin forever.
        let taken: Option<ThreadHandle> = {
            let mut slot = self.handle.lock().unwrap();
            slot.take()
        };
        if let Some(mut handle) = taken {
            match handle.rx.try_recv() {
                Ok(res) => {
                    if let Some(j) = handle.join.take() {
                        let _ = j.join();
                    }
                    self.result = Some(res);
                    return true;
                }
                Err(_) => {
                    // Not ready — put the handle back.
                    let mut slot = self.handle.lock().unwrap();
                    *slot = Some(handle);
                    return false;
                }
            }
        }
        false
    }

    fn is_cancelled(&self) -> bool {
        if let Some(r) = &self.result {
            return r.status == "TERMINATED";
        }
        let slot = self.handle.lock().unwrap();
        match &*slot {
            Some(h) => h.cancel.load(std::sync::atomic::Ordering::Relaxed),
            None => false,
        }
    }

    fn error_message(&self) -> String {
        self.result
            .as_ref()
            .map(|r| r.error.clone())
            .unwrap_or_default()
    }

    fn status_str(&self) -> String {
        if let Some(r) = &self.result {
            return r.status.clone();
        }
        let slot = self.handle.lock().unwrap();
        if slot.is_some() {
            "RUNNING".to_string()
        } else {
            "UNKNOWN".to_string()
        }
    }
}

impl CfmlNative for FutureNative {
    fn class_name(&self) -> &str {
        "Future"
    }

    fn call_method(&mut self, name: &str, args: Vec<CfmlValue>) -> CfmlResult {
        match name.to_ascii_lowercase().as_str() {
            "get" => {
                let timeout = args
                    .get(0)
                    .map(|v| v.as_string().parse::<i64>().unwrap_or(0))
                    .unwrap_or(0);
                self.await_result(timeout)
            }
            "isdone" => Ok(CfmlValue::Bool(self.is_done())),
            "iscancelled" | "iscanceled" => Ok(CfmlValue::Bool(self.is_cancelled())),
            "cancel" => {
                if self.inline_resolved || self.result.is_some() {
                    return Ok(CfmlValue::Bool(false));
                }
                let slot = self.handle.lock().unwrap();
                if let Some(h) = &*slot {
                    h.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
                    return Ok(CfmlValue::Bool(true));
                }
                Ok(CfmlValue::Bool(false))
            }
            "error" => Ok(CfmlValue::string(self.error_message())),
            "status" => Ok(CfmlValue::string(self.status_str())),
            other => Err(CfmlError::runtime(format!(
                "Future has no method [{}]",
                other
            ))),
        }
    }

    fn get_property(&self, name: &str) -> Option<CfmlValue> {
        // Allow `future.done` / `future.status` / `future.error` as property
        // reads — same shape WireBox uses elsewhere.
        match name.to_ascii_lowercase().as_str() {
            // Property read is &self only; report the cached state without
            // polling. Callers wanting authoritative "done" should invoke
            // the isDone() method (which can poll).
            "done" => Some(CfmlValue::Bool(self.result.is_some())),
            "status" => Some(CfmlValue::string(self.status_str())),
            "error" => Some(CfmlValue::string(self.error_message())),
            _ => None,
        }
    }
}

/// Read a numeric option from a CFML struct (case-insensitive). Returns
/// `None` when the key is absent or unparseable.
pub fn struct_get_i64(s: &IndexMap<String, CfmlValue>, key: &str) -> Option<i64> {
    s.iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .and_then(|(_, v)| match v {
            CfmlValue::Int(i) => Some(*i),
            CfmlValue::Double(d) => Some(*d as i64),
            CfmlValue::Bool(b) => Some(if *b { 1 } else { 0 }),
            other => other.as_string().parse::<i64>().ok(),
        })
}
