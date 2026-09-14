package lru

import "sync"

type ConcurrentLRU struct {
	inner *LRU
	lock  sync.Mutex
}

func NewConcurrentLRU(capacity int) *ConcurrentLRU {
	return &ConcurrentLRU{
		inner: NewLRU(capacity),
		lock:  sync.Mutex{},
	}
}

func (lru *ConcurrentLRU) Set(k int, v int) {
	lru.lock.Lock()
	defer lru.lock.Unlock()

	lru.inner.Set(k, v)
}

func (lru *ConcurrentLRU) Get(k int) (int, bool) {
	lru.lock.Lock()
	defer lru.lock.Unlock()

	return lru.inner.Get(k)
}
