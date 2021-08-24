# CacheMap

CacheMap is a data structure for concurrently caching values.

the `cache` function will look up a value in the map, or generate and store a new one using the provided function

## Example

```
use cachemap::CacheMap;
	
let m = CacheMap::new();

let fst = m.cache("key", || 5u32);
let snd = m.cache("key", || 7u32);

assert_eq!(*fst, *snd);
assert_eq!(*fst, 5u32);
```

## Features 🌞

- can cache values concurrently (using `&CacheMap<K,V>` rather than `&mut CacheMap<K,V>`)
- returned references use the map's lifetime, so clients can avoid smart pointers
- clients can optionally enable the `dashmap` feature, which uses `dashmap`
  internally and allows:
  - getting `Arc<V>` pointers, in case values need to outlive the map
  - adding `Arc<V>` directly, allowing unsized values, and re-using `Arc<V>`s from elsewhere
