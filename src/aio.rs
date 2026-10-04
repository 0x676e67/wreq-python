//! Awaitables that drive Rust futures from asyncio tasks.
//!
//! A [`Coroutine`] polls its future on the event loop thread, and [`spawn`] moves
//! the work to Tokio. A wake marks the coroutine ready and queues it on the [`Port`]
//! of the awaiting loop, ringing a socket the loop watches only when the queue was
//! empty, so waking Tokio threads never take the GIL. A loop that cannot watch the
//! socket is woken with `call_soon_threadsafe`, which does attach the waking thread.
//! The loop thread then resolves every queued asyncio future in one batch.

mod coroutine;
mod port;

use std::{
    future::{Future, poll_fn},
    mem,
    task::Poll,
};

use pyo3::{IntoPyObjectExt, exceptions::PyRuntimeError, prelude::*};
use tokio_util::task::AbortOnDropHandle;

pub use self::coroutine::Coroutine;
use self::port::Port;
use crate::runtime::Runtime;

/// Run `fut` on the runtime once the coroutine named `qualname` is first awaited.
#[inline]
pub fn spawn<'py, F, T>(
    py: Python<'py>,
    qualname: &'static str,
    runtime: &Runtime,
    fut: F,
) -> PyResult<Bound<'py, Coroutine>>
where
    F: Future<Output = PyResult<T>> + Send + 'static,
    T: for<'a> IntoPyObject<'a> + Send + 'static,
{
    local(py, qualname, run(runtime.clone(), fut))
}

/// Poll `fut` on the event loop thread, attached to Python, when awaited.
#[inline]
pub fn local<'py, F, T>(
    py: Python<'py>,
    qualname: &'static str,
    fut: F,
) -> PyResult<Bound<'py, Coroutine>>
where
    F: Future<Output = PyResult<T>> + Send + 'static,
    T: for<'a> IntoPyObject<'a>,
{
    Bound::new(py, coroutine(qualname, fut))
}

/// Like [`local`], but `async with` may enter the coroutine directly, entering the
/// async context manager it returns without a separate `await`.
#[inline]
pub fn managed<'py, F, T>(
    py: Python<'py>,
    qualname: &'static str,
    fut: F,
) -> PyResult<Bound<'py, Coroutine>>
where
    F: Future<Output = PyResult<T>> + Send + 'static,
    T: for<'a> IntoPyObject<'a>,
{
    Bound::new(py, coroutine(qualname, fut).managed())
}

/// A coroutine that returns `value` without suspending.
#[inline]
pub fn ready<'py, T>(
    qualname: &'static str,
    value: Bound<'py, T>,
) -> PyResult<Bound<'py, Coroutine>> {
    let py = value.py();
    let value = value.into_any().unbind();
    Bound::new(py, Coroutine::new(qualname, async move { Ok(value) }))
}

/// Yield to the event loop once, like `asyncio.sleep(0)`.
#[inline]
pub async fn yield_now() {
    let mut yielded = false;
    poll_fn(|cx| {
        if mem::replace(&mut yielded, true) {
            return Poll::Ready(());
        }
        // A wake during the poll makes the coroutine yield without a future.
        cx.waker().wake_by_ref();
        Poll::Pending
    })
    .await;
}

/// A coroutine awaiting `fut` and converting its output to a Python object.
fn coroutine<F, T>(qualname: &'static str, fut: F) -> Coroutine
where
    F: Future<Output = PyResult<T>> + Send + 'static,
    T: for<'a> IntoPyObject<'a>,
{
    Coroutine::new(qualname, async move {
        let value = fut.await?;
        Python::attach(|py| value.into_py_any(py))
    })
}

/// Spawn `fut` on the runtime and wait for it; dropping the wait aborts the task.
/// The task keeps the runtime alive until it ends.
#[inline]
pub async fn run<F, T>(runtime: Runtime, fut: F) -> PyResult<T>
where
    F: Future<Output = PyResult<T>> + Send + 'static,
    T: Send + 'static,
{
    let task = runtime.handle().clone().spawn(async move {
        let _owner = runtime;
        fut.await
    });
    AbortOnDropHandle::new(task)
        .await
        .map_err(|err| PyRuntimeError::new_err(err.to_string()))?
}
