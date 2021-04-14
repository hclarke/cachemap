# CacheMap

CacheMap is a data structure for concurrently caching values.

the `cache` function will look up a value in the map, or generate and store a new one using the provided function

## Example

```
use cachemap::CacheMap;
	
let m = CacheMap::new();

let fst = m.cache("key", || 5u32).as_ref();
let snd = m.cache("key", || 7u32).as_ref();

assert_eq!(*fst, *snd);
assert_eq!(*fst, 5u32);
```

## Features 🌞

- can cache values concurrently (using `&CacheMap<K,V>` rather than `&mut CacheMap<K,V>`)
- returned references use the map's lifetime, so clients can avoid smart pointers
- clients can optionally get `Arc<V>` pointers, in case values need to outlive the map
- values can be addes as `Arc<V>`, allowing unsized values, and re-using `Arc<V>`s from elsewhere

## MisFeatures 💧

> A cache with a bad policy is another name for a memory leak

this map provides only one way to remove things from the cache: drop the entire map.
