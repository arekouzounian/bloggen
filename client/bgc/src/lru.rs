//! LRU cache implementation
//! Doubly linked list, arena alloc
//! on evict swap to last index in arena so that we don't get internal
//! fragmentation
use std::cmp::Eq;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

#[derive(Clone, Debug)]
struct CacheEntry<K, V>
where
    K: Clone + Debug,
    K: Clone + Debug,
{
    key: K,
    value: V,
    next: Option<usize>,
    prev: Option<usize>,
}

// Current implementation mutates even on reads because of LRU reordering.
// Another approach which would allow immutable concurrent reads would be to
// avoid updating LRU on reads, and only update on writes. This of course
// would no longer be a true LRU cache, but a LFW (Least Frequently Written)
// cache. Since we expect writes to happen pretty frequently, though, it might
// be preferable.
// Do we expect cache sizes to become a problem?

/// Thread unsafe LRU cache implementation.
pub struct LruCache<K, V>
where
    K: Eq + Hash + Clone + Debug,
    V: Clone + Debug,
{
    index: HashMap<K, usize>,           // key -> arena index
    entry_arena: Vec<CacheEntry<K, V>>, // cache line arena
    head: Option<usize>,
    tail: Option<usize>,
}

pub struct LruCacheIterator<'a, K, V>
where
    K: Eq + Hash + Clone + Debug,
    V: Clone + Debug,
{
    index: usize,
    lru_cache: &'a LruCache<K, V>,
}

impl<'a, K, V> Iterator for LruCacheIterator<'a, K, V>
where
    K: Eq + Hash + Clone + Debug,
    V: Clone + Debug,
{
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.lru_cache.entry_arena.get(self.index);
        self.index += 1;
        next.map(|cache_entry| (&cache_entry.key, &cache_entry.value))
    }
}

impl<K, V> LruCache<K, V>
where
    K: Eq + Hash + Clone + Debug,
    V: Clone + Debug,
{
    pub fn new(capacity: usize) -> Self {
        Self {
            index: HashMap::with_capacity(capacity),
            entry_arena: Vec::with_capacity(capacity),
            head: None,
            tail: None,
        }
    }

    pub fn len(&self) -> usize {
        self.entry_arena.len()
    }

    pub fn capacity(&self) -> usize {
        self.entry_arena.capacity()
    }

    fn put_to_front(&mut self, new_ind: usize) {
        match self.head {
            Some(head_ind) => {
                if head_ind == new_ind {
                    return;
                }
            }
            None => return,
        }

        let new_node = self.entry_arena.get(new_ind).unwrap();

        // update immediately left and immediately right pointers
        let prev_ind = new_node.prev.unwrap();
        let next_ind = new_node.next;
        self.entry_arena[prev_ind].next = next_ind;

        if let Some(next_ind) = next_ind {
            self.entry_arena[next_ind].prev = Some(prev_ind);
        } else {
            // update tail
            self.tail = Some(prev_ind);
        }

        let head_ind = self.head.unwrap();
        self.entry_arena[head_ind].prev = Some(new_ind);
        self.entry_arena[new_ind].next = Some(head_ind);
        self.entry_arena[new_ind].prev = None; // new head has no predecessor
        self.head = Some(new_ind);
    }

    fn evict(&mut self) -> Option<CacheEntry<K, V>> {
        if self.tail.is_none() {
            return None;
        }

        let old_tail_ind = self.tail.unwrap();
        let tail_ref = &self.entry_arena[old_tail_ind];
        self.index.remove(&tail_ref.key);

        if tail_ref.prev.is_none() {
            // tail is head — only one element
            self.head = None;
            self.tail = None;
            self.entry_arena.clear();
            self.index.clear();
            return None;
        }

        // Update tail: the predecessor becomes the new tail with no next
        let new_tail_ind = tail_ref.prev.unwrap();
        self.tail = Some(new_tail_ind);
        self.entry_arena[new_tail_ind].next = None;

        let last_ind = self.entry_arena.len() - 1;

        if old_tail_ind != last_ind {
            // Move the last arena entry into the evicted slot so the arena stays compact
            self.entry_arena[old_tail_ind] = self.entry_arena[last_ind].clone();

            // Re-wire the moved entry's neighbors to its new position
            if let Some(prev_ind) = self.entry_arena[old_tail_ind].prev {
                self.entry_arena[prev_ind].next = Some(old_tail_ind);
            }
            if let Some(next_ind) = self.entry_arena[old_tail_ind].next {
                self.entry_arena[next_ind].prev = Some(old_tail_ind);
            }

            // Fix the index so the moved key points to its new slot
            let moved_key = self.entry_arena[old_tail_ind].key.clone();
            self.index.insert(moved_key, old_tail_ind);

            // Fix head/tail pointers if they referred to the moved slot
            if self.head == Some(last_ind) {
                self.head = Some(old_tail_ind);
            }
            if self.tail == Some(last_ind) {
                self.tail = Some(old_tail_ind);
            }
        }

        self.entry_arena.pop()
    }

    /// If called on an existing key, updates value
    /// Returns a list of all entries that have been evicted
    pub fn insert(&mut self, key: K, value: V) -> Vec<(K, V)> {
        let node_ind: usize;
        let mut evicted_entries = Vec::new();

        if let Some(ind) = self.index.get(&key) {
            self.entry_arena[*ind].value = value;
            node_ind = *ind;
        } else {
            while self.entry_arena.len() >= self.entry_arena.capacity() {
                if let Some(cache_entry) = self.evict() {
                    evicted_entries.push((cache_entry.key, cache_entry.value));
                } else {
                    break;
                }
            }

            self.entry_arena.push(CacheEntry {
                key: key.clone(),
                value: value,
                next: None,
                prev: None,
            });
            node_ind = self.entry_arena.len() - 1;
            self.index.insert(key, node_ind);
        }

        // update to front
        if self.head.is_none() {
            self.head = Some(node_ind);
            self.tail = Some(node_ind);
        } else {
            let tail_ind = self.tail.unwrap();
            self.entry_arena[tail_ind].next = Some(node_ind);
            self.entry_arena[node_ind].prev = Some(tail_ind);
            self.tail = Some(node_ind);
            self.put_to_front(node_ind);
        }

        evicted_entries
    }

    pub fn update(&mut self, key: &K, value: V) {
        if let Some(ind) = self.index.get(key) {
            self.entry_arena[*ind].value = value;
            self.put_to_front(*ind);
        }
    }

    /// Gets but doesn't push to front
    fn get_no_update(&self, key: &K) -> Option<&V> {
        self.index
            .get(key)
            .map(|&ind| self.entry_arena.get(ind).map(|entry| &entry.value))
            .flatten()
    }

    fn get_mut_no_update(&mut self, key: &K) -> Option<&mut V> {
        self.index
            .get(key)
            .map(|&ind| self.entry_arena.get_mut(ind).map(|entry| &mut entry.value))
            .flatten()
    }

    pub fn get<'a, 'b>(&'a mut self, key: &'b K) -> Option<&'a V> {
        let node_index = *self.index.get(key)?;
        self.put_to_front(node_index);
        self.get_no_update(key)
    }

    pub fn get_mut<'a, 'b>(&'a mut self, key: &'b K) -> Option<&'a mut V> {
        let node_index = *self.index.get(key)?;
        self.put_to_front(node_index);
        self.get_mut_no_update(key)
    }

    pub fn contains_key(&mut self, key: &K) -> bool {
        self.index.contains_key(key)
    }

    // for debugging
    pub fn dump(&self) {
        println!("{:?}", self.head);
        println!("{:?}", self.tail);

        let mut curr = self.head;
        while let Some(ind) = curr {
            let item = &self.entry_arena[ind];
            println!("{:?}", item);
            curr = item.next;
        }
    }
}

impl<'a, K, V> std::iter::IntoIterator for &'a LruCache<K, V>
where
    K: Hash + Eq + Clone + Debug,
    V: Clone + Debug,
{
    type Item = (&'a K, &'a V);
    type IntoIter = LruCacheIterator<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        LruCacheIterator {
            index: 0,
            lru_cache: self,
        }
    }
}

mod lru_cache_tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn insert_works() {
        const CAPACITY: usize = 64;
        let mut cache = LruCache::new(CAPACITY);
        for i in 0..CAPACITY {
            cache.insert(i, i);
        }

        cache.dump();

        for i in 0..CAPACITY {
            assert!(cache.get(&i).is_some())
        }
    }

    #[test]
    fn check_evicts() {
        const CAPACITY: usize = 1;
        const ITERATIONS: usize = 100;
        let mut cache = LruCache::new(CAPACITY);

        for i in 0..ITERATIONS {
            cache.insert(i, i);
        }

        assert!(cache.get(&(ITERATIONS - 1)).is_some());
        assert!(cache.len() == CAPACITY);
    }

    #[test]
    fn len_and_capacity() {
        const CAPACITY: usize = 10;
        let mut cache = LruCache::new(CAPACITY);
        assert_eq!(cache.len(), 0);
        assert_eq!(cache.capacity(), CAPACITY);

        cache.insert(1, "a");
        cache.insert(2, "b");
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.capacity(), CAPACITY);
    }

    #[test]
    fn insert_duplicate_updates_value() {
        let mut cache = LruCache::new(4);
        cache.insert("key", 1);
        cache.insert("key", 2);

        // Should still be only 1 entry
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get(&"key"), Some(&2));
    }

    #[test]
    fn contains_key_works() {
        let mut cache = LruCache::new(4);
        cache.insert("hello", 42);

        assert!(cache.contains_key(&"hello"));
        assert!(!cache.contains_key(&"world"));
    }

    #[test]
    fn update_modifies_value() {
        let mut cache = LruCache::new(4);
        cache.insert("key", 1);
        cache.update(&"key", 99);

        assert_eq!(cache.get(&"key"), Some(&99));
    }

    #[test]
    fn get_promotes_to_front_preventing_eviction() {
        // With capacity 2: insert A, insert B (A is now LRU tail).
        // Access A to promote it to head. Insert C — B should be evicted, not A.
        let mut cache = LruCache::new(2);
        cache.insert("a", 1);
        cache.insert("b", 2);

        // Access "a" so it becomes the most-recently-used
        let _ = cache.get(&"a");

        // Insert "c" — should evict "b" (LRU), keeping "a"
        cache.insert("c", 3);

        assert!(cache.contains_key(&"a"), "a should not be evicted");
        assert!(!cache.contains_key(&"b"), "b should have been evicted");
        assert!(cache.contains_key(&"c"), "c should be present");
    }

    #[test]
    fn iterator_covers_all_items() {
        let mut cache = LruCache::new(4);
        cache.insert(1, "one");
        cache.insert(2, "two");
        cache.insert(3, "three");

        let keys: Vec<i32> = (&cache).into_iter().map(|(k, _)| *k).collect();
        assert_eq!(keys.len(), 3);
        assert!(keys.contains(&1));
        assert!(keys.contains(&2));
        assert!(keys.contains(&3));
    }
}
