use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Mutex;

/// An insert-only map for caching the result of functions
pub struct CacheMap<K, V> {
    inner: Mutex<HashMap<K, Box<V>>>,
}

impl<K, V> Default for CacheMap<K, V> {
    fn default() -> Self {
        CacheMap {
            inner: Default::default(),
        }
    }
}

impl<K: Eq + Hash, V> std::iter::FromIterator<(K, V)> for CacheMap<K, V> {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = (K, V)>,
    {
        CacheMap {
            inner: Mutex::new(iter.into_iter().map(|(k, v)| (k, Box::new(v))).collect()),
        }
    }
}

pub struct IntoIter<K, V>(std::collections::hash_map::IntoIter<K, Box<V>>);

impl<K, V> Iterator for IntoIter<K, V> {
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(k, v)| (k, *v))
    }
}

impl<K, V> IntoIterator for CacheMap<K, V> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.inner.into_inner().unwrap().into_iter())
    }
}

impl<K: Eq + Hash, V> CacheMap<K, V> {
    /// Fetch the value associated with the key, or run the provided function to insert one.
    ///
    /// # Example
    ///
    /// ```
    /// use cachemap::CacheMap;
    ///
    /// let m = CacheMap::new();
    ///
    /// let fst = m.cache("key", || 5u32);
    /// let snd = m.cache("key", || 7u32);
    ///
    /// assert_eq!(*fst, *snd);
    /// assert_eq!(*fst, 5u32);
    /// ```
    pub fn cache<F: FnOnce() -> V>(&self, key: K, f: F) -> &V {
        let v = std::ptr::NonNull::from(
            self.inner
                .lock()
                .unwrap()
                .entry(key)
                .or_insert_with(|| Box::new(f()))
                .as_ref(),
        );
        // Safety: We only support adding entries to the hashmap, and as long as a reference is
        // maintained the value will be present.
        unsafe { v.as_ref() }
    }

    /// Fetch the value associated with the key, or insert a default value.
    pub fn cache_default(&self, key: K) -> &V
    where
        V: Default,
    {
        self.cache(key, || Default::default())
    }
}

impl<K, V> CacheMap<K, V> {
    /// Creates a new CacheMap
    pub fn new() -> Self {
        Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_insert() {
        let m = CacheMap::new();

        let a = m.cache("key", || 21u32);
        assert_eq!(21, *a);
    }

    #[test]
    fn double_insert() {
        let m = CacheMap::new();

        let a = m.cache("key", || 5u32);
        let b = m.cache("key", || 7u32);

        assert_eq!(*a, *b);
        assert_eq!(5, *a);
    }

    #[test]
    fn insert_two() {
        let m = CacheMap::new();

        let a = m.cache("a", || 5u32);
        let b = m.cache("b", || 7u32);

        assert_eq!(5, *a);
        assert_eq!(7, *b);

        let c = m.cache("a", || 9u32);
        let d = m.cache("b", || 11u32);

        assert_eq!(*a, *c);
        assert_eq!(*b, *d);

        assert_eq!(5, *a);
        assert_eq!(7, *b);
    }
}
