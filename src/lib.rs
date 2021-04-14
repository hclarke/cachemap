use dashmap::DashMap;
use std::hash::Hash;
use std::sync::Arc;
use std::marker::PhantomData;
use std::ops::Deref;

/// An insert-only map for caching the result of functions
pub struct CacheMap<K:Hash+Eq,V:?Sized> {
	inner: DashMap<K,Arc<V>>,
}

/// A handle that can be converted to a &T or an Arc<T>
pub struct ArcRef<'a,T:?Sized> {
	// this pointer never gets dereferenced, but it has to be T, so that Ref is the right size for wide pointers
	#[allow(dead_code)]
	fake_ptr: *const T, 
	phantom: PhantomData<&'a T>,
}

impl<'a,T:?Sized> Clone for ArcRef<'a, T> {
	fn clone(&self) -> Self {
		*self
	}
}
impl<'a,T:?Sized> Copy for ArcRef<'a, T> {}


impl<T:?Sized> Deref for ArcRef<'_,T> {
	type Target = Arc<T>;
	fn deref(&self) -> &Self::Target {
		unsafe { std::mem::transmute(self) }
	}
}

impl<'a,T:?Sized> ArcRef<'a,T> {
	/// Converts the ArcRef into an Arc<T>
	pub fn to_arc(self) -> Arc<T> {
		self.deref().clone()
	}

	/// Converts the ArcRef into a &T
	pub fn as_ref(self) -> &'a T {
		let ptr = &**self as *const T;
		unsafe { &*ptr }
	}

}

impl<K:Hash+Eq,V> CacheMap<K,V> {
	/// Fetch the value associated with the key, or run the profvided function to insert one.
	///
	/// #Example
	///
	/// ```
	/// use cachemap::CacheMap;
	///
    /// let m = CacheMap::new();
    ///
    /// let fst = m.cache("key", || 5u32).as_ref();
    /// let snd = m.cache("key", || 7u32).as_ref();
    /// 
	/// assert_eq!(*fst, *snd);
	/// assert_eq!(*fst, 5u32);
	/// ```
	pub fn cache<F:FnOnce()->V>(&self, key: K, f:F) -> ArcRef<'_, V> {
		self.cache_arc(key, || Arc::new(f()))
	}
}

impl<K:Hash+Eq,V:?Sized> CacheMap<K,V> {
	/// Creates a new CacheMap
	pub fn new() -> Self {
		CacheMap {
			inner: DashMap::new(),
		}
	}

	/// Fetch the value associated with the key, or run the profvided function to insert one.
	/// With this version, the function returns an Arc<V>, whch allows caching unsized types.
	///
	/// #Example
	///
	/// ```
	/// use cachemap::CacheMap;
	///
	/// let m: CacheMap<_, [usize]> = CacheMap::new();
	///
	/// let a = m.cache_arc("a", || {
	///		let a = &[1,2,3][..];
	///     a.into()
	/// }).as_ref();
	///
	/// let b = m.cache_arc("b", || {
	///		let b = &[9,9][..];
	///     b.into()
	/// }).as_ref();
	///
	/// assert_eq!(a, &[1,2,3]);
	/// assert_eq!(b, &[9,9]);
	/// ```
	pub fn cache_arc<F:FnOnce()->Arc<V>>(&self, key: K, f:F) -> ArcRef<'_, V> {
		let val = self.inner.entry(key).or_insert_with(f);
		let arc: &Arc<V> = &*val;
		let arc_ref: &ArcRef<'_,V> = unsafe {
			std::mem::transmute(arc)
		};
		*arc_ref
	}
}



#[cfg(test)]
mod tests {
	use super::*;

	#[test]
    fn single_insert() {
        let m = CacheMap::new();

        let a = m.cache("key", || 21u32).as_ref();
        assert_eq!(21, *a);
    }

    #[test]
    fn double_insert() {
        let m = CacheMap::new();

        let a = m.cache("key", || 5u32).as_ref();
        let b = m.cache("key", || 7u32).as_ref();

        assert_eq!(*a, *b);
        assert_eq!(5, *a);
    }

    #[test]
    fn insert_two() {

        let m = CacheMap::new();

        let a = m.cache("a", || 5u32).as_ref();
        let b = m.cache("b", || 7u32).as_ref();

        assert_eq!(5, *a);
        assert_eq!(7, *b);


        let c = m.cache("a", || 9u32).as_ref();
        let d = m.cache("b", || 11u32).as_ref();

        assert_eq!(*a, *c);
        assert_eq!(*b, *d);

        assert_eq!(5, *a);
        assert_eq!(7, *b);
    }

    #[test]
    fn use_after_drop() {

    	#[derive(Clone)]
    	struct Foo(usize);
    	impl Drop for Foo {
    		fn drop(&mut self) {
    			assert_eq!(33, self.0);
    		}
    	}

    	{
	    	let mut arc = {
	        	let m = CacheMap::new();
	        	let a = m.cache("key", || Foo(99)).to_arc();
	        	assert_eq!(99, (*a).0);
	        	a
	        };

	        Arc::make_mut(&mut arc).0 = 33;
	    }

	    assert!(true);

    }


}
