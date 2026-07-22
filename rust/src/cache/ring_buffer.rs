/// cache/ring_buffer.rs
/// 
/// Abstract Ring Buffer structure.
#[derive(Debug)]
pub struct Slot<T> {
    data: T,
}

impl<T> Slot<T> {
    pub fn new(storage: T) -> Self {
        Self { data: storage }
    }

    /// Return immutable access to inner data
    pub fn inner(&self) -> &T {
        &self.data
    }

    /// Return mutable access to inner data
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.data
    }
}

#[derive(Debug)]
pub struct RingBuffer<T> {
    slots: Vec<Slot<T>>,
}

impl<T> RingBuffer<T> {
    /// Create ring buffer with fixed slot count.
    /// Each slot is initialized by the supplied factory function
    /// 
    /// # Panics
    /// Panics if capacity == 0.
    pub fn new<F>(capacity: usize, mut factory: F) -> Self 
    where 
        F: FnMut() -> T,
    {
        assert!(capacity > 0, "RingBuffer capacity cannot be zero");

        let mut slots = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            slots.push(Slot::new(factory()));
        }

        Self { slots }
    }

    pub fn capacity(&self) -> usize {
        self.slots.len()
    }

    /// Map unbounded logical index to physical slot offset
    #[inline]
    pub fn slot_idx(&self, index: usize) -> usize {
        index % self.capacity()
    }

    /// Compute next physical index (circular)
    #[inline]
    pub fn next_idx(&self, index: usize) -> usize {
        (index + 1) % self.capacity()
    }

    /// Compute previous physical index (circular)
    /// 
    /// # Params
    /// index: Logical index or physical index (they are the same after modded)
    #[inline]
    pub fn prev_idx(&self, index: usize) -> usize {
        (index + self.capacity() - 1) % self.capacity()
    }

    pub fn get_slot(&self, index: usize) -> &Slot<T> {
        &self.slots[self.slot_idx(index)]
    }

    pub fn get_slot_mut(&mut self, index: usize) -> &mut Slot<T> {
        let idx = self.slot_idx(index);
        &mut self.slots[idx]
    }
}
