/// lru.rs
/// 
/// LRU structure in Rust

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::atomic::{self, AtomicUsize};

type Index = usize;

struct Node<K, V> {
    key: K,
    value: V,
    prev: Option<Index>,
    next: Option<Index>,
}

pub struct Lru<K, V> 
where 
    K: Eq + Hash,
{
    capacity: usize,
    len: AtomicUsize,

    head: Option<Index>,
    tail: Option<Index>,

    map: HashMap<K, Index>,

    arena: Vec<Node<K, V>>,
    free: Vec<usize>, // Stack
}

impl<K, V> Lru<K, V> 
where
    K: Eq + Hash + Clone,
{
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "Capacity must be greater than zero");

        return Self {
            capacity,
            len: AtomicUsize::new(0),
            head: None,
            tail: None,
            map: HashMap::with_capacity(capacity),
            arena: Vec::with_capacity(capacity),
            free: Vec::new(),
            
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn len(&self) -> usize {
        self.len.load(atomic::Ordering::Relaxed)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn put(&mut self, key: K, value: V) {
        if let Some(&idx) = self.map.get(&key) {
            self.detach(idx);
            self.arena[idx].value = value;
            self.insert_head(idx);
            return;
        }

        if self.len() == self.capacity {
            if let Some(tail_idx) = self.tail {
                self.detach(tail_idx);

                let evicted_key = &self.arena[tail_idx].key;
                self.map.remove(evicted_key);

                self.free.push(tail_idx);
                self.len.fetch_sub(1, atomic::Ordering::Relaxed);
            }
        }

        if let Some(idx) = self.free.pop() {
            self.arena[idx] = Node {
                key: key.clone(),
                value,
                prev: None,
                next: None,
            };
            self.map.insert(key, idx);
            self.insert_head(idx);
        } else {
            let idx = self.arena.len();
            self.arena.push(Node {
                key: key.clone(),
                value,
                prev: None,
                next: None,
            });
            self.map.insert(key, idx);
            self.insert_head(idx);
        }

        self.len.fetch_add(1, atomic::Ordering::Relaxed);
    }

    pub fn get(&mut self, key: &K) -> Option<&V> {
        let idx = *self.map.get(key)?;

        self.detach(idx);
        self.insert_head(idx);

        Some(&self.arena[idx].value)
    }

    fn insert_head(&mut self, idx: Index) {
        self.arena[idx].prev = None;
        self.arena[idx].next = self.head;

        if let Some(head_idx) = self.head {
            self.arena[head_idx].prev = Some(idx);
        } else {
            self.tail = Some(idx);
        }

        self.head = Some(idx);
    }

    fn detach(&mut self, idx: Index) {
        let prev = self.arena[idx].prev;
        let next = self.arena[idx].next;

        match prev {
            Some(p) => self.arena[p].next = next,
            None => self.head = next,
        }

        match next {
            Some(n) => self.arena[n].prev = prev,
            None => self.tail = prev,
        }

        self.arena[idx].prev = None;
        self.arena[idx].next = None;
    }
}