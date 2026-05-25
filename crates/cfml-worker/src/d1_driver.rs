//! Cloudflare D1 driver for the cfml-stdlib dynamic-driver registry.
//!
//! # Status: v1 stub
//!
//! D1's `worker::d1::D1Database` API is async (every prepare/bind/run/all
//! returns a `Future`). The `cfml_stdlib::DynamicDbDriver` contract is sync
//! (`fn execute(...) -> CfmlResult`). On Cloudflare Workers there is **no**
//! way to block sync Rust on a JS Promise: the runtime is a single-threaded
//! event loop and `wasm_bindgen_futures` can only *schedule* futures, never
//! drive one to completion from inside a sync stack frame.
//!
//! Making `<cfquery datasource="d1-name">` work end-to-end therefore
//! requires either (a) extending the dynamic-driver trait with an `async fn
//! execute_async` and routing the VM's cfquery intercept to that async path,
//! or (b) refactoring the VM into a state machine that can suspend on a
//! pending DB call. Both are tracked for v2.
//!
//! For v1 the driver registers cleanly but returns a descriptive error if
//! CFML actually executes a query against it. This keeps the registration
//! plumbing in place (so the v2 swap is a one-file change) and gives the
//! user a clear message rather than a silent empty result.

#![cfg(target_arch = "wasm32")]

use cfml_common::dynamic::CfmlValue;
use cfml_common::vm::{CfmlError, CfmlResult};
use cfml_stdlib::DynamicDbDriver;
use std::sync::Arc;
use worker::d1::D1Database;

pub struct D1Driver {
    name: String,
    #[allow(dead_code)]
    db: Arc<D1Database>,
}

impl D1Driver {
    pub fn new(name: impl Into<String>, db: Arc<D1Database>) -> Self {
        Self {
            name: name.into(),
            db,
        }
    }
}

impl DynamicDbDriver for D1Driver {
    fn execute(&self, _sql: &str, _params: &CfmlValue, _return_type: &str) -> CfmlResult {
        Err(CfmlError::runtime(format!(
            "cfquery against D1 datasource '{}' is not supported in cfml-worker v1 — \
             the D1 binding is async-only and the current cfquery dispatch is sync. \
             Track the async cfquery refactor for v2.",
            self.name
        )))
    }
}
