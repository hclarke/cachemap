use std::hash::Hash;

#[cfg(not(feature = "abi_stable"))]
mod basic_impl {
    pub type BoxImpl<T> = Box<T>;
    pub type HashMapImpl<K, V> = std::collections::HashMap<K, V>;
    pub type MutexImpl<T> = std::sync::Mutex<T>;
    pub type MutexGuardImpl<'a, T> = std::sync::MutexGuard<'a, T>;
    pub type IterImpl<'a, K, V> = std::collections::hash_map::Iter<'a, K, V>;
    pub type IntoIterImpl<K, V> = std::collections::hash_map::IntoIter<K, V>;

    pub fn box_into_inner_impl<T>(b: BoxImpl<T>) -> T {
        *b
    }

    pub fn mutex_lock_impl<'a, T>(m: &'a MutexImpl<T>) -> MutexGuardImpl<'a, T> {
        m.lock().unwrap()
    }

    pub fn mutex_into_inner_impl<T>(m: MutexImpl<T>) -> T {
        m.into_inner().unwrap()
    }
}

#[cfg(not(feature = "abi_stable"))]
use basic_impl::*;

#[cfg(feature = "abi_stable")]
mod abi_stable_impl {
    use abi_stable::{
        external_types::RMutex,
        std_types::{RBox, RHashMap},
    };
    pub type BoxImpl<T> = RBox<T>;
    pub type HashMapImpl<K, V> = RHashMap<K, V>;
    pub type MutexImpl<T> = RMutex<T>;
    pub type MutexGuardImpl<'a, T> =
        abi_stable::external_types::parking_lot::mutex::RMutexGuard<'a, T>;
    pub type IterImpl<'a, K, V> = abi_stable::std_types::map::Iter<'a, K, V>;
    pub type IntoIterImpl<K, V> = abi_stable::std_types::map::IntoIter<K, V>;

    pub fn box_into_inner_impl<T>(b: BoxImpl<T>) -> T {
        RBox::into_inner(b)
    }

    pub fn mutex_lock_impl<'a, T>(m: &'a MutexImpl<T>) -> MutexGuardImpl<'a, T> {
        m.lock()
    }

    pub fn mutex_into_inner_impl<T>(m: MutexImpl<T>) -> T {
        m.into_inner()
    }
}

#[cfg(feature = "abi_stable")]
use abi_stable_impl::*;

/// An insert-only map for caching the result of functions
#[cfg_attr(feature = "abi_stable", derive(abi_stable::StableAbi))]
#[cfg_attr(feature = "abi_stable", repr(C))]
pub struct CacheMap<K, V> {
    inner: MutexImpl<HashMapImpl<K, BoxImpl<V>>>,
}

impl<K: Eq + Hash, V> Default for CacheMap<K, V> {
    fn default() -> Self {
        CacheMap {
            inner: MutexImpl::new(Default::default()),
        }
    }
}

impl<K: Eq + Hash, V> std::iter::FromIterator<(K, V)> for CacheMap<K, V> {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = (K, V)>,
    {
        CacheMap {
            inner: MutexImpl::new(
                iter.into_iter()
                    .map(|(k, v)| (k, BoxImpl::new(v)))
                    .collect(),
            ),
        }
    }
}

pub struct IntoIter<K, V>(IntoIterImpl<K, BoxImpl<V>>);

impl<K, V> Iterator for IntoIter<K, V> {
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|t| (t.0, box_into_inner_impl(t.1)))
    }
}

impl<K, V> IntoIterator for CacheMap<K, V> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(mutex_into_inner_impl(self.inner).into_iter())
    }
}

pub struct Iter<'a, K, V> {
    iter: IterImpl<'a, K, BoxImpl<V>>,
    _guard: MutexGuardImpl<'a, HashMapImpl<K, BoxImpl<V>>>,
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|t| (t.0, t.1.as_ref()))
    }
}

impl<'a, K, V> IntoIterator for &'a CacheMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        let guard = mutex_lock_impl(&self.inner);
        let iter = unsafe {
            std::mem::transmute::<IterImpl<K, BoxImpl<V>>, IterImpl<K, BoxImpl<V>>>(guard.iter())
        };
        Iter {
            iter,
            _guard: guard,
        }
    }
}

impl<K: Eq + Hash, V> CacheMap<K, V> {
    /// Fetch the value associated with the key, or run the provided function to insert one.
    ///
    /// # Example
    ///
    /// ```
    /// use cachemap2::CacheMap;
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
            mutex_lock_impl(&self.inner)
                .entry(key)
                .or_insert_with(|| BoxImpl::new(f()))
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

    /// Return whether the map contains the given key.
    pub fn contains_key<Q: ?Sized>(&self, key: &Q) -> bool
    where
        K: std::borrow::Borrow<Q>,
        Q: Hash + Eq,
    {
        mutex_lock_impl(&self.inner).contains_key(key)
    }

    /// Return an iterator over the map.
    ///
    /// This iterator will lock the underlying map until it is dropped.
    pub fn iter(&self) -> Iter<K, V> {
        self.into_iter()
    }
}

impl<K: Eq + Hash, V> CacheMap<K, V> {
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
    fn contains_key() {
        let m = CacheMap::new();

        m.cache("key", || 21u32);
        assert!(m.contains_key("key"));
        assert!(!m.contains_key("other"));
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

    #[test]
    fn iter() {
        use std::collections::HashMap;
        use std::iter::FromIterator;
        let m = CacheMap::new();
        m.cache("a", || 5u32);
        m.cache("b", || 7u32);

        let mut expected = HashMap::<&'static str, u32>::from_iter([("a", 5u32), ("b", 7u32)]);

        for (k, v) in &m {
            assert!(expected.remove(k).expect("unexpected key") == *v);
        }

        assert!(expected.is_empty());
    }
}
