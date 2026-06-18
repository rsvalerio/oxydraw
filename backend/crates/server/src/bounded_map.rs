//! An LRU-bounded map used to cap unauthenticated in-memory state against unbounded growth
//! (SEC-33). Currently backs the per-room collab scene snapshots ([`crate::rooms`]).

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use tracing::warn;

/// Map bounded by entry count and total payload bytes; inserts past either bound evict
/// least-recently-used entries (reads refresh recency).
///
/// CONC-7: recency is stamped through atomics (`clock` / [`BoundedEntry::last_used`]), so
/// [`get`](Self::get) only needs `&self`. That lets the map live behind an `RwLock` whose
/// readers proceed concurrently — the hot snapshot-load path no longer serializes on a single
/// exclusive lock. Mutations (insert/evict) still take the write lock.
pub(crate) struct BoundedMap<V> {
    entries: HashMap<String, BoundedEntry<V>>,
    total_bytes: usize,
    max_entries: usize,
    max_bytes: usize,
    /// Monotonic counter stamping each access, ordering entries for LRU eviction.
    clock: AtomicU64,
}

struct BoundedEntry<V> {
    value: V,
    bytes: usize,
    last_used: AtomicU64,
}

impl<V> BoundedMap<V> {
    pub(crate) fn new(max_entries: usize, max_bytes: usize) -> Self {
        Self {
            entries: HashMap::new(),
            total_bytes: 0,
            max_entries,
            max_bytes,
            clock: AtomicU64::new(0),
        }
    }

    /// Next tick of the recency clock. `Relaxed` is enough: recency only needs a
    /// consistent total order of stamps, not synchronization with other memory.
    fn tick(&self) -> u64 {
        self.clock.fetch_add(1, Ordering::Relaxed) + 1
    }

    pub(crate) fn insert(&mut self, name: String, value: V, bytes: usize) {
        // Unreachable while the router's body limit stays below `max_bytes`, but don't let
        // a single oversized entry evict the whole map trying to make room for it.
        if bytes > self.max_bytes {
            warn!(%name, bytes, "bounded map: entry exceeds the byte bound, dropping");
            return;
        }
        let entry = BoundedEntry {
            value,
            bytes,
            last_used: AtomicU64::new(self.tick()),
        };
        if let Some(old) = self.entries.insert(name, entry) {
            self.total_bytes -= old.bytes;
        }
        self.total_bytes += bytes;
        while self.entries.len() > self.max_entries || self.total_bytes > self.max_bytes {
            self.evict_lru();
        }
    }

    pub(crate) fn get(&self, name: &str) -> Option<&V> {
        let clock = self.tick();
        self.entries.get(name).map(|e| {
            e.last_used.store(clock, Ordering::Relaxed);
            &e.value
        })
    }

    fn evict_lru(&mut self) {
        // CONC-7 — justified O(n): this `min_by_key` scan is linear in `self.entries.len()`,
        // which is capped at `max_entries` (`SCENE_MAX_ENTRIES = 1024`). Worst case is
        // sustained insert pressure on a full map: each insert past the bound evicts exactly
        // one entry (one in, one out), so it pays one scan of ≤1024 entries — ~1024 relaxed
        // atomic loads plus compares, a few microseconds — while holding the write lock. That
        // is dwarfed by the snapshot payload handling on the same write path, so it is not a
        // contention hotspot at this bound.
        //
        // A sub-O(n) structure (BTreeMap keyed by recency stamp, intrusive LRU list) is
        // deliberately *not* used: it would have to reorder on every `get` to refresh
        // recency, which needs `&mut self` and would force reads to take the exclusive write
        // lock — regressing the concurrent lock-free read path this type exists to provide
        // (see the type-level CONC-7 note). The O(n) eviction is the chosen cost of keeping
        // reads on `&self`. Revisit only if `max_entries` grows by orders of magnitude.
        let oldest = self
            .entries
            .iter()
            .min_by_key(|(_, e)| e.last_used.load(Ordering::Relaxed))
            .map(|(name, _)| name.clone());
        let Some(name) = oldest else { return };
        if let Some(e) = self.entries.remove(&name) {
            self.total_bytes -= e.bytes;
            warn!(%name, bytes = e.bytes, "bounded map: evicted least-recently-used entry");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_map_evicts_lru_past_entry_bound() {
        let mut map = BoundedMap::new(2, 1000);
        map.insert("a".to_string(), 1, 1);
        map.insert("b".to_string(), 2, 1);
        map.get("a"); // refresh "a" so "b" is now the least recently used
        map.insert("c".to_string(), 3, 1);
        assert_eq!(map.get("a"), Some(&1));
        assert_eq!(map.get("b"), None, "LRU entry evicted at the entry bound");
        assert_eq!(map.get("c"), Some(&3));
    }

    #[test]
    fn bounded_map_evicts_past_byte_bound() {
        let mut map = BoundedMap::new(100, 10);
        map.insert("a".to_string(), 1, 6);
        map.insert("b".to_string(), 2, 6);
        assert_eq!(map.get("a"), None, "oldest entry evicted at the byte bound");
        assert_eq!(map.get("b"), Some(&2));
        assert_eq!(map.total_bytes, 6);
    }

    #[test]
    fn bounded_map_overwrite_replaces_byte_charge() {
        let mut map = BoundedMap::new(100, 10);
        map.insert("a".to_string(), 1, 6);
        map.insert("a".to_string(), 2, 4);
        assert_eq!(map.get("a"), Some(&2));
        assert_eq!(map.total_bytes, 4);
    }

    #[test]
    fn bounded_map_drops_entries_larger_than_byte_bound() {
        let mut map = BoundedMap::new(100, 10);
        map.insert("a".to_string(), 1, 6);
        map.insert("huge".to_string(), 2, 11);
        assert_eq!(map.get("huge"), None, "oversized entry not stored");
        assert_eq!(map.get("a"), Some(&1), "existing entries untouched");
    }
}
