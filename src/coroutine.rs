//! Awaitables that drive Rust futures from Python coroutines.
//!
//! `awaitable` implements the coroutine protocol, `scope` adds `async with` to request
//! coroutines, and `asyncio` holds what is specific to asyncio event loops.
//!
//! A [`Coroutine`] polls its future on the event loop thread, and [`spawn`] moves
//! the work to Tokio. A wake marks the coroutine ready and queues it on the [`Port`]
//! of the awaiting loop, ringing a socket the loop watches only when the queue was
//! empty, so waking Tokio threads never take the GIL. A loop that cannot watch the
//! socket is woken with `call_soon_threadsafe`, which does attach the waking thread.
//! The loop thread then resolves every queued asyncio future in one batch.

mod asyncio;
mod awaitable;
mod scope;

use std::{
    any::TypeId,
    future::{Future, poll_fn},
    mem,
    task::Poll,
};

use pyo3::{IntoPyObjectExt, PyTypeInfo, intern, prelude::*, sync::PyOnceLock};

pub(crate) use self::asyncio::running_loop;
pub use self::awaitable::Coroutine;
use self::{asyncio::Port, scope::Scope};
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
    local(py, qualname, runtime.clone().run_task(fut))
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
    T: for<'a> IntoPyObject<'a> + 'static,
{
    Bound::new(py, coroutine(qualname, fut))
}

/// A result type whose native `__aenter__` returns the object itself, so `async with` on a
/// [`managed`] coroutine enters it without awaiting `__aenter__`.
pub trait EntersSelf: PyTypeInfo {
    /// The native `__aenter__`, recorded by [`record_enters_self`].
    fn native_aenter() -> &'static PyOnceLock<Py<PyAny>>;
}

/// Record `T`'s native `__aenter__` at module init, before user code can replace it.
pub fn record_enters_self<T: EntersSelf>(py: Python<'_>) -> PyResult<()> {
    let aenter = T::type_object(py).getattr(intern!(py, "__aenter__"))?;
    let _ = T::native_aenter().set(py, aenter.unbind());
    Ok(())
}

/// Like [`local`], but `async with` may enter the coroutine directly, as `async with await`
/// would: its result must be an async context manager, whose `__aenter__` is awaited too.
#[inline]
pub fn managed<'py, F, T>(
    py: Python<'py>,
    qualname: &'static str,
    fut: F,
) -> PyResult<Bound<'py, Coroutine>>
where
    F: Future<Output = PyResult<T>> + Send + 'static,
    T: EntersSelf + for<'a> IntoPyObject<'a> + 'static,
{
    let mut coroutine = coroutine(qualname, fut);
    // The result is always exactly a `T`, so it enters itself unless user code replaced
    // `T.__aenter__`.
    coroutine.scope = Scope::Ready(|py| {
        T::native_aenter().get(py).is_some_and(|native| {
            T::type_object(py)
                .getattr(intern!(py, "__aenter__"))
                .is_ok_and(|aenter| aenter.is(native))
        })
    });
    Bound::new(py, coroutine)
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

/// A coroutine awaiting `fut` and converting its output to a Python object. Like a sync
/// method, one with no result returns `None`, where `()` would convert to an empty tuple.
fn coroutine<F, T>(qualname: &'static str, fut: F) -> Coroutine
where
    F: Future<Output = PyResult<T>> + Send + 'static,
    T: for<'a> IntoPyObject<'a> + 'static,
{
    Coroutine::new(qualname, async move {
        let value = fut.await?;
        Python::attach(|py| {
            if TypeId::of::<T>() == TypeId::of::<()>() {
                return Ok(py.None());
            }
            value.into_py_any(py)
        })
    })
}
