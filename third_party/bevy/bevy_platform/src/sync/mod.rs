//! Provides various synchronization alternatives to language primitives.
//!
//! These are `std::sync`'s items, the ones the build names.

pub use alloc::sync::Arc;
pub use std::sync::{
    LazyLock, Mutex, MutexGuard, OnceLock, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard,
};

pub mod atomic;
