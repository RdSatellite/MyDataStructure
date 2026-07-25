use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Mutex;

use super::lru::Lru;

pub struct ConcurrentLru<K, V>
where 
    K: Eq + Hash + Clone,
    V: Clone,
{
    inner: Mutex<Lru<K, V>>
}

impl<K, V> ConcurrentLru<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Mutex::new(Lru::new(capacity)),
        }
    }

    pub fn put(&self, key: K, value: V) {
        let mut lru = self.inner.lock().unwrap();
        lru.put(key, value);
    }

    pub fn get(&self, key: &K) -> Option<V>{
        let mut lru = self.inner.lock().unwrap();
        lru.get(key).cloned()
    }

    pub fn len(&self) -> usize {
        let lru = self.inner.lock().unwrap();
        lru.len()
    }

    pub fn is_empty(&self) -> bool {
        let lru = self.inner.lock().unwrap();
        lru.len() == 0
    }

    pub fn capacity(&self) -> usize {
        let lru = self.inner.lock().unwrap();
        lru.capacity()
    }
}