//! Provides various atomic alternatives to language primitives.
//!
//! Every target benilla builds has native atomics of each width, so these are `core`'s.

pub use core::sync::atomic::{
    AtomicBool, AtomicI16, AtomicI32, AtomicI64, AtomicI8, AtomicIsize, AtomicPtr, AtomicU16,
    AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering,
};
