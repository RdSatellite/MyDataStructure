package lru

type node struct {
	k    int
	v    int
	Next *node
	Last *node
}

type LRU struct {
	l        *node
	m        map[int]*node
	capacity int
}

func NewLRU(capacity int) *LRU {
	dummy := &node{}
	dummy.Next = dummy
	dummy.Last = dummy

	return &LRU{
		l:        dummy,
		m:        make(map[int]*node),
		capacity: capacity,
	}
}

// --- internal (unsafe!) --- ///

func (lru *LRU) remove(n *node) {
	n.Next.Last = n.Last
	n.Last.Next = n.Next

	delete(lru.m, n.k)
}

func (lru *LRU) add(n *node) {
	k, dummy := n.k, lru.l

	n.Next = dummy.Next
	dummy.Next.Last = n
	n.Last = dummy
	dummy.Next = n

	lru.m[k] = n

	if len(lru.m) > lru.capacity {
		toDelete := lru.l.Last
		lru.remove(toDelete)
	}
}

// --- apis --- //

func (lru *LRU) Len() int {
	return len(lru.m)
}

func (lru *LRU) Set(k int, v int) {
	if existed, ok := lru.m[k]; ok {
		lru.remove(existed)
	}

	n := &node{
		k: k,
		v: v,
	}

	lru.add(n)
}

func (lru *LRU) Get(k int) (int, bool) {
	existed, ok := lru.m[k]

	if !ok {
		return -1, false
	}

	// Push to front
	lru.remove(existed)
	lru.add(existed)

	return existed.v, true
}
