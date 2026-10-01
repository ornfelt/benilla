use alloc::fmt;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

/// Wraps `async_executor::Task`, a spawned future.
///
/// Tasks are also futures themselves and yield the output of the spawned future.
///
/// When a task is dropped, it gets canceled and won't be polled again.
///
/// Tasks that panic get immediately canceled. Awaiting a canceled task also causes a panic.
#[must_use = "Tasks are canceled when dropped, use `.detach()` to run them in the background."]
pub struct Task<T>(async_task::Task<T>);

impl<T> Task<T> {
    /// Creates a new task from a given `async_executor::Task`
    pub(crate) fn new(task: async_task::Task<T>) -> Self {
        Self(task)
    }
}

impl<T> Task<T> {
    /// Detaches the task to let it keep running in the background.
    pub fn detach(self) {
        self.0.detach();
    }

    /// Returns `true` if the current task is finished.
    ///
    /// Unlike poll, it doesn't resolve the final value, it just checks if the task has finished.
    /// Note that in a multithreaded environment, this task can be finished immediately after calling this function.
    pub fn is_finished(&self) -> bool {
        // Defer to the `async_task` implementation.
        self.0.is_finished()
    }
}

impl<T> Future for Task<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // `async_task` has `Task` implement `Future`, so we just poll it.
        Pin::new(&mut self.0).poll(cx)
    }
}

// All variants of Task<T> are expected to implement Unpin
impl<T> Unpin for Task<T> {}

// Derive doesn't work for macro types, so we have to implement this manually.
impl<T> fmt::Debug for Task<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use crate::Task;

    #[test]
    fn task_is_sync() {
        fn is_sync<T: Sync>() {}
        is_sync::<Task<()>>();
    }

    #[test]
    fn task_is_send() {
        fn is_send<T: Send>() {}
        is_send::<Task<()>>();
    }
}
