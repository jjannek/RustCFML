//! Cloudflare Workers scheduled (cron) handler — expired-session sweep.
//!
//! KV TTL alone will eventually evict expired session blobs, but the
//! eviction is opaque and the keys linger long enough to bloat list
//! costs in busy namespaces. This handler does the same job
//! deterministically:
//!
//! 1. Lists all KV keys in the session namespace.
//! 2. For each, deserializes the session blob and checks
//!    `now - last_accessed_secs > timeout_secs`.
//! 3. Deletes expired entries from KV.
//!
//! `onSessionEnd` is **intentionally not invoked**. The lifecycle hook
//! requires loading Application.cfc, instantiating a VM, and running
//! user code per expired session — costly, and rarely needed for the
//! Cloudflare deployment model. If you need cleanup semantics, do them
//! at `onSessionStart` (idempotently) or via your own scheduled CFML
//! page on a separate cron.
//!
//! Cadence is configured by the host project's `wrangler.toml` cron
//! expression. Every 30 minutes is a sensible default; tighten it if
//! your session timeouts are short.

#![cfg(target_arch = "wasm32")]

use crate::kv_stores::KvBackedSessionStore;
use crate::WorkerConfig;
use worker::*;

/// Entry point a host project's `#[event(scheduled)]` delegates to:
///
/// ```ignore
/// #[event(scheduled)]
/// pub async fn scheduled(event: ScheduledEvent, env: Env, ctx: ScheduleContext) {
///     let config = build_config(&env);
///     let _ = cfml_worker::handle_scheduled(event, env, ctx, &config).await;
/// }
/// ```
pub async fn handle_scheduled(
    _event: ScheduledEvent,
    _env: Env,
    _ctx: ScheduleContext,
    config: &WorkerConfig,
) -> Result<()> {
    let Some(kv_sessions) = config.kv_sessions.as_ref() else {
        // No KV session store wired up — nothing to sweep.
        return Ok(());
    };

    let session_store = KvBackedSessionStore::new(kv_sessions.clone());
    let now_secs = (Date::now().as_millis() / 1000) as u64;

    // sweep_expired already deletes the KV entries; we discard the
    // returned (id, data) pairs because we don't fire onSessionEnd.
    let _ = session_store.sweep_expired(now_secs).await?;
    Ok(())
}
