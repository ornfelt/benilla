//! Provides a fundamental executor primitive, built on [`async-executor`].
//!
//! [`async-executor`]: https://crates.io/crates/async-executor

use core::{
    fmt,
    panic::{RefUnwindSafe, UnwindSafe},
};
use derive_more::{Deref, DerefMut};

type ExecutorInner<'a> = async_executor::Executor<'a>;
type LocalExecutorInner<'a> = async_executor::LocalExecutor<'a>;

pub use async_task::FallibleTask;

/// Wrapper around a multi-threading-aware async executor.
/// Spawning will generally require tasks to be `Send` and `Sync` to allow multiple
/// threads to send/receive/advance tasks.
///
/// If you require an executor _without_ the `Send` and `Sync` requirements, consider
/// using [`LocalExecutor`] instead.
#[derive(Deref, DerefMut, Default)]
pub struct Executor<'a>(ExecutorInner<'a>);

/// Wrapper around a single-threaded async executor.
/// Spawning wont generally require tasks to be `Send` and `Sync`, at the cost of
/// this executor itself not being `Send` or `Sync`. This makes it unsuitable for
/// global statics.
///
/// If need to store an executor in a global static, or send across threads,
/// consider using [`Executor`] instead.
#[derive(Deref, DerefMut, Default)]
pub struct LocalExecutor<'a>(LocalExecutorInner<'a>);

impl Executor<'_> {
    /// Construct a new [`Executor`]
    #[expect(clippy::allow_attributes, reason = "This lint may not always trigger.")]
    #[allow(dead_code, reason = "not all feature flags require this function")]
    pub const fn new() -> Self {
        Self(ExecutorInner::new())
    }
}

impl LocalExecutor<'_> {
    /// Construct a new [`LocalExecutor`]
    #[expect(clippy::allow_attributes, reason = "This lint may not always trigger.")]
    #[allow(dead_code, reason = "not all feature flags require this function")]
    pub const fn new() -> Self {
        Self(LocalExecutorInner::new())
    }
}

impl UnwindSafe for Executor<'_> {}

impl RefUnwindSafe for Executor<'_> {}

impl UnwindSafe for LocalExecutor<'_> {}

impl RefUnwindSafe for LocalExecutor<'_> {}

impl fmt::Debug for Executor<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Executor").finish()
    }
}

impl fmt::Debug for LocalExecutor<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LocalExecutor").finish()
    }
}
