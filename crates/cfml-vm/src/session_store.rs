//! Pluggable session storage backend.

use cfml_common::dynamic::CfmlValue;
use indexmap::IndexMap;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::SessionData;

/// Trait for session storage backends.
///
/// All methods receive a session ID string. Implementations must be
/// `Send + Sync` so they can be shared across Tokio worker threads.
pub trait SessionStore: Send + Sync + 'static {
    /// Retrieve a session by ID. Returns `None` if the session does not exist.
    fn get(&self, id: &str) -> Option<SessionData>;
    /// Insert or overwrite a session.
    fn set(&self, id: &str, data: SessionData);
    /// Remove a session.
    fn remove(&self, id: &str);
    /// Atomically move session data from `old_id` to `new_id`.
    fn rotate(&self, old_id: &str, new_id: &str);
    /// Returns `true` if the session exists.
    fn contains(&self, id: &str) -> bool {
        self.get(id).is_some()
    }
    /// Drain all sessions whose `last_accessed_secs` age exceeds their
    /// `timeout_secs`, returning their IDs and variable maps so callers can
    /// invoke `onSessionEnd`.
    fn take_expired(&self, now_secs: u64) -> Vec<(String, IndexMap<String, CfmlValue>)>;
}

// ─────────────────────────────────────────────
// MemoryStore — in-process default
// ─────────────────────────────────────────────

/// In-process session store backed by a `Mutex<HashMap>`.
/// Identical behaviour to the original `ServerState.sessions` field.
#[derive(Clone)]
pub struct MemoryStore {
    inner: Arc<Mutex<HashMap<String, SessionData>>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionStore for MemoryStore {
    fn get(&self, id: &str) -> Option<SessionData> {
        self.inner.lock().ok()?.get(id).cloned()
    }

    fn set(&self, id: &str, data: SessionData) {
        if let Ok(mut m) = self.inner.lock() {
            m.insert(id.to_string(), data);
        }
    }

    fn remove(&self, id: &str) {
        if let Ok(mut m) = self.inner.lock() {
            m.remove(id);
        }
    }

    fn rotate(&self, old_id: &str, new_id: &str) {
        if let Ok(mut m) = self.inner.lock() {
            if let Some(data) = m.remove(old_id) {
                m.insert(new_id.to_string(), data);
            }
        }
    }

    fn contains(&self, id: &str) -> bool {
        self.inner.lock().ok().map_or(false, |m| m.contains_key(id))
    }

    fn take_expired(&self, now_secs: u64) -> Vec<(String, IndexMap<String, CfmlValue>)> {
        if let Ok(mut m) = self.inner.lock() {
            let expired: Vec<String> = m
                .iter()
                .filter(|(_, s)| now_secs.saturating_sub(s.last_accessed_secs) > s.timeout_secs)
                .map(|(k, _)| k.clone())
                .collect();
            expired
                .into_iter()
                .filter_map(|id| m.remove(&id).map(|s| (id, s.variables)))
                .collect()
        } else {
            Vec::new()
        }
    }
}
