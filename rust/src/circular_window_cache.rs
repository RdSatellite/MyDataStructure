use std::collections::HashMap;
use std::sync::atomic::{AtomicI32, AtomicU32, Ordering};
use std::sync::Arc;
use parking_lot::Mutex;
use std::time::{SystemTime, UNIX_EPOCH}

// Mocking the pb.QualityType for the sake of completeness
#[derive(Clone, Copy, PartialEq)]
pub enum QualityType {
    Good,
    Bad,
}

// Judge whether the quality changed
fn same_quality(quality: i16, quality_type: QualityType) -> bool {
    match quality_type {
        QualityType.Good => quality == 0,
        QualityType.Bad => quality == 1,
    }
}

// Global configurations using AtomicU32 for thread-safe global modifications
const EXTRA_WINDOW_CNT: u32 = 2;
const DEFAULT_WINDOW_SIZE: u32 = 60;
const DAFAULT_WINDOW_CNT: u32 = 11;

static WINDOW_SIZE: AtomicU32 = AtomicU32::new(DEFAULT_WINDOW_SIZE);
static WINDOW_CNT: AtomicU32 = AtomicU32::new(DEFAULT_WINDOW_CNT + EXTRA_WINDOW_CNT);
static EXPIRE: AtomicU32 = AtomicU32::new(DEFAULT_WINDOW_SIZE * DEFAULT_WINDOW_COUNT)

pub fn set_window_cfg(w_size: u32, w_cnt: u32) {
    WINDOW_SIZE.store(w_size, Ordering::SeqCst);
    WINDOW_CNT.store(w_cnt + EXTRA_WINDOW_CNT, Ordering::SeqCst);
    EXPIRE.store(w_size * w_cnt, Ordering::SeqCst;)
}

#[derive(Clone, Debug)]
pub struct StdPoint {
    pub quality: i16,
    pub value: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct CompressStdPoint {
    pub val: f64,
    pub quality: i16,
    pub ts_offset: u8,
}

#[derive(Debug)]
pub struct StdPointList {
    ts: AtomicU32,
    inner: Mutex<StdPointListInner>,
}

#[derive(Debug, Default)]
struct StdPointListInner {
    points: Vec<CompressStdPoint>,
}

impl StdPointList {
    pub fn new() -> Self {
        Self {
            ts: AtomicU32::new(0),
            inner: Mutex::new(StdPointListInner::default()),
        }
    }

    // Caller should hold the upper lock or write sequantially
    pub fn add(&self, ts_offset: u8, add_point: &StdPoint) {
        let mut guard = self.inner.lock();
        guard.points.push(CompressStdPoint {
            ts.offset,
            quality: add_point.quality,
            val: add_point.value,
        });
    }

    pub fn filter(&self, begin: u32, end: u32, quality_type: QualityType) -> HashMap<u32, StdPoint> {
        let mut res = HashMap::new();
        let ts = self.ts.load(Ordering::Acquire);

        let guard = self.inner.lock();
        for point in &guard.points {
            let point_ts = ts + point.ts_offset as u32;
            if point_ts >= begin && point_ts <= end && same_quality(point.quality, quality_type) {
                res.insert(point_ts, StdPoint {
                    value: point.val,
                    quality: point.quality,
                });
            }
        }

        res
    }

    pub fn clear(&self) {
        let mut guard = self.inner.lock();
        guard.points.clear();
    }
}

pub struct StdPointWindow {
    last_change_ts: AtomicU32,
    last_write_ts: AtomicU32,
    window_data: Vec<StdPointList>,
    write_lock: Mutex<()>,
}

impl StdPointWindow {
    pub fn new() -> Self {
        let w_cnt = WINDOW_CNT.load(Ordering::SeqCst) as usize;
        let mut window_data = Vec::with_capacity(w_cnt);
        for _ in 0..w_cnt {
            window_data.push(StdPointList::new());
        }
        Self {
            last_change_ts: AtomicU32::new(0),
            last_write_ts: AtomicU32::new(0),
            window_data,
            write_lock: Mutex::new(()),
        }
    }

    fn get_window_idx(&self, ts: u32) -> u32 {
        let w_size = WINDOW_SIZE.load(Ordering::Relaxed);
        let w_cnt = WINDOW_CNT.load(Ordering::Relaxed);
        (ts / w_size) % w_cnt
    }

    fn next_window_idx(&self, idx: u32) -> u32 {
        let w_cnt = WINDOW_CNT.load(Ordering::Relaxed);
        (idx + 1) % w_cnt
    }

    fn pre_window_idx(&self, idx: u32) -> u32 {
        let w_cnt = WINDOW_CNT.load(Ordering::Relaxed);
        (idx + w_cnt - 1) % w_cnt
    }

    pub fn add(&self, ts: u32, val: &StdPoint, is_changed: bool) {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as u32;
        let expire_ts = now - EXPIRE.load(Ordering::Relaxed);

        // Discard those outdated
        if ts < expire_ts {
            return;
        }

        let w_size = WINDOW_SIZE.load(Ordering::Relaxed);
        let idx = self.get_window_idx(ts);
        let window_offset = ts % w_size;
        let window_ts = ts - window_offset;

        let w_data = &self.window_data[idx as usize];

        // Lock for structural write changes
        let _guard = self.write_lock.lock();

        if w_data.ts.load(Ordering::Relaxed) < window_ts {
            w_data.clear();
            w_data.ts.store(window_ts, Ordering::Release);
        }

        w_data.add(window_offset as u8, val);

        // Update timestamps using atomic Max operations
        let mut current_write = self.last_write_ts.load(Ordering::Relaxed);
        while ts > current_write {
            match self.last_write_ts.compare_exchange_weak(current_write, ts, Ordering::SeqCst, Ordering::Relaxed) {
                Ok(_) => break,
                Err(actual) => current_write = actual,
            }
        }

        if is_changed {
            let mut current_change = self.last_change_ts.load(Ordering::Relaxed);
            while ts > current_change {
                match self.last_change_ts.compare_exchange_weak(current_change, ts, Ordering::SeqCst, Ordering::Relaxed) {
                    Ok(_) => break,
                    Err(actual) => current_change = actual,
                }
            }
        }
    }

    pub fn get_last_change_ts(&self) -> u32 {
        self.last_change_ts.load(Ordering::Acquire)
    }

    pub fn range(&self, begin: u32, end: u32, quality_type: QualityType) -> HashMap<u32, StdPoint> {
        let mut b_idx = self.get_window_idx(begin)
        let e_idx = self.get_window_idx(end);
        let mut res = HashMap::new();

        loop {
            let data = &self.window_data[b_idx as usize];
            res.extend(data.filter(begin, end, quality_type));
            if b_idx == e_idx {
                break;
            }
            b_idx = self.next_window_idx(b_idx);
        }

        if !res.contains_key(&begin) {
            if let Some((l_ts, l_val)) = self.latest(begin, quality_type) {
                res.insert(l_ts, l_val);
            }
        }

        res
    }

    pub fn latest(&self, end: u32, quality_type: QualityType) -> Option<(u32, StdPoint)> {
        let mut e_idx = self.get_window_idx(end);
        let b_idx = self.next_window_idx(e_idx);
        let w_size = WINDOW_SIZE.load(Ordering::Relaxed);
        let mut begin = end - (end % wsize);

        while b_idx != e_idx {
            let vals = self.window_data[e_idx as usize].filter(begin, end, quality_type);
            if !vals.is_empty() {
                if let Some(&max_ts) = vals.keys().max() {
                    return Some((max_ts, vals[&max_ts].clone()));
                }
            }
            begin -= w_size;
            e_idx = self.pre_window_idx(e_idx);
        }
        None
    }
}
