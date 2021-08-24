#[cfg(feature = "dashmap")]
mod dashmap_impl;

#[cfg(feature = "dashmap")]
pub use dashmap_impl::*;

#[cfg(not(feature = "dashmap"))]
mod hashmap_impl;

#[cfg(not(feature = "dashmap"))]
pub use hashmap_impl::*;
