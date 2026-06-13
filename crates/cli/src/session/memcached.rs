//! Memcached-backed session store.
//!
//! Enabled by the `memcached` Cargo feature. Each session is serialised to
//! JSON and stored under the key `{key_prefix}{session_id}` with a TTL equal
//! to the session's `timeout_secs`.

#[cfg(feature = "memcached")]
mod inner {
    use cfml_common::dynamic::CfmlValue;
    use cfml_vm::{SessionData, session_store::SessionStore};
    use indexmap::IndexMap;
    use memcache::Client;

    pub struct MemcachedStore {
        client: Client,
        key_prefix: String,
    }

    impl MemcachedStore {
        /// Connect to a Memcached cluster.
        ///
        /// `servers` should be bare `host:port` strings; the `memcache://`
        /// scheme is added automatically.
        pub fn new(servers: &[String], key_prefix: &str) -> Result<Self, memcache::MemcacheError> {
            let urls: Vec<String> = servers
                .iter()
                .map(|s| {
                    if s.starts_with("memcache://") {
                        s.clone()
                    } else {
                        format!("memcache://{}", s)
                    }
                })
                .collect();
            let client = memcache::connect(urls)?;
            Ok(Self {
                client,
                key_prefix: key_prefix.to_string(),
            })
        }

        fn key(&self, id: &str) -> String {
            format!("{}{}", self.key_prefix, id)
        }
    }

    impl SessionStore for MemcachedStore {
        fn get(&self, id: &str) -> Option<SessionData> {
            let raw: Option<String> = self.client.get(&self.key(id)).ok().flatten();
            raw.and_then(|s| serde_json::from_str(&s).ok())
        }

        fn set(&self, id: &str, data: SessionData) {
            if let Ok(json) = serde_json::to_string(&data) {
                let ttl = data.timeout_secs as u32;
                let _ = self.client.set(&self.key(id), json, ttl);
            }
        }

        fn remove(&self, id: &str) {
            let _ = self.client.delete(&self.key(id));
        }

        fn rotate(&self, old_id: &str, new_id: &str) {
            if let Some(data) = self.get(old_id) {
                self.set(new_id, data);
                self.remove(old_id);
            }
        }

        fn take_expired(
            &self,
            _now_secs: u64,
        ) -> Vec<(String, String, IndexMap<String, CfmlValue>)> {
            // Memcached enforces TTL natively — nothing to drain here, and
            // onSessionEnd is consequently never delivered for this store.
            Vec::new()
        }
    }
}

#[cfg(feature = "memcached")]
pub use inner::MemcachedStore;
