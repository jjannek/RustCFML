use std::collections::HashMap;

pub struct TileCache<K, V> where K: Eq + std::hash::Hash {
    entries: HashMap<K, V>,
}

impl<K, V> TileCache<K, V> where K: Eq + std::hash::Hash {
    pub fn new(capacity: usize) -> Self {
        TileCache {
            entries: HashMap::with_capacity(capacity),
        }
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.entries.get(key)
    }

    pub fn insert(&mut self, key: K, value: V) {
        self.entries.insert(key, value);
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.entries.remove(key)
    }
}
