// concurrent/shard_map.rs
//
// Concurrent safe shard map

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::RwLock;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct ShardMap<K, V> 
where
    K: 'static
{
    shard_cnt: u64,
    hash_fun: Box<dyn (Fn(&K)->u64) + Send + Sync>,
    
    // Shared resources
    shards: Vec<RwLock<HashMap<K, V>>>,
    size: AtomicUsize,
}

impl<K, V> ShardMap<K, V>
where 
    K: Eq + Hash,
{
    pub fn new<F>(shard_cnt: u64, hash_fun: F) -> Self
    where
        F: (Fn(&K)->u64) + Send + Sync + 'static,
    {
        let mut shards = Vec::with_capacity(shard_cnt as usize);
        for _ in 0..shard_cnt {
            shards.push(RwLock::new(HashMap::new()));
        }

        Self {
            shard_cnt,
            hash_fun: Box::new(hash_fun),
            shards,
            size: AtomicUsize::new(0)
        }
    }

    pub fn with_default_hash(shard_cnt: u64) -> Self {
        Self::new(shard_cnt, default_hash)
    }

    fn get_shard_idx(&self, key: &K) -> usize {
        let hash = (self.hash_fun)(key);
        (hash % self.shard_cnt) as usize
    }

    fn get_shard(&self, key: &K) -> &RwLock<HashMap<K, V>> {
        &self.shards[self.get_shard_idx(key)]
    }

    pub fn get(&self, key: &K) -> Option<V> 
    where
        V: Clone
    {
        let guard = self.get_shard(key).read().unwrap();
        guard.get(key).cloned()
    }

    pub fn get_many(&self, keys: &[&K]) -> HashMap<K, V> 
    where 
        K: Clone,
        V: Clone,
    {
        let mut res = HashMap::with_capacity(keys.len());
        
        for &key in keys {
            let guard = self.get_shard(key).read().unwrap();
            if let Some(value) = guard.get(key) {
                res.insert((*key).clone(), value.clone());
            }
        }

        res
    }

    // Set and return the new value, else return the existed one. 
    pub fn setnx(&self, key: K, value: V) -> V 
    where
        K: Clone,
        V: Clone,
    {
        let mut guard = self.get_shard(&key).write().unwrap();

        match guard.entry(key) {
            Entry::Occupied(entry) => {
                entry.get().clone()
            }
            Entry::Vacant(entry) => {
                entry.insert(value.clone());
                self.size.fetch_add(1, Ordering::Relaxed);
                value
            }
        }
    }

    // Fetch all existing keys
    pub fn keys(&self) -> Vec<K> 
    where 
        K: Clone,
    {
        let capacity = self.len();
        let mut res = Vec::with_capacity(capacity);
        
        for shard in &self.shards {
            let guard = shard.read().unwrap();
            for key in guard.keys() {
                res.push(key.clone())
            }
        }

        res
    }

    pub fn len(&self) -> usize {
        self.size.load(Ordering::Relaxed)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

// --- Hash --- //

pub fn default_hash<T: Hash + ?Sized>(key: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}
