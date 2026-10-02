//! Provides various synchronization alternatives to language primitives.
//!
//! Currently missing from this module are the following items:
//! * `Condvar`
//! * `WaitTimeoutResult`
//! * `mpsc`
//!
//! Otherwise, this is a drop-in replacement for `std::sync`.

pub use alloc::sync::{Arc, Weak};
pub use std::sync::{
    Barrier, BarrierWaitResult, LazyLock, LockResult, Mutex, MutexGuard, Once, OnceLock,
    OnceState, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard, TryLockError,
    TryLockResult,
};

pub mod atomic;
