//! Provides various atomic alternatives to language primitives.
//!
//! Every target benilla builds has native atomics of each width, so these are `core`'s.

pub use core::sync::atomic::{
    AtomicBool, AtomicI32, AtomicI64, AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering,
};
