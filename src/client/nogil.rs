use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
};

use pin_project_lite::pin_project;
use pyo3::{
    coroutine::CancelHandle,
    exceptions::{PyRuntimeError, asyncio::CancelledError},
    prelude::*,
};
use tokio_util::{sync::CancellationToken, task::AbortOnDropHandle};

use crate::runtime::Runtime;

pin_project! {
    /// A future that allows Python threads to run while it is being polled or executed.
    /// It also handles cancellation and spawns the task in tokio runtime.
    pub struct NoGIL<T> {
        #[pin]
        handle: AbortOnDropHandle<PyResult<T>>,
        cancel: CancelHandle,
    }
}

struct GuardedWaker(Waker);

// ===== impl NoGIL =====

impl<T> NoGIL<T>
where
    T: Send + 'static,
{
    /// Spawn on the selected handle, keeping the runtime alive until the task ends.
    #[inline]
    pub fn new<Fut>(runtime: &Runtime, fut: Fut, cancel: CancelHandle) -> Self
    where
        Fut: Future<Output = PyResult<T>> + Send + 'static,
    {
        let owner = runtime.clone();
        Self {
            handle: AbortOnDropHandle::new(Python::attach(|py| {
                py.detach(|| {
                    runtime.handle().spawn(async move {
                        let _owner = owner;
                        fut.await
                    })
                })
            })),
            cancel,
        }
    }

    /// Also cancel the task when its client is closed.
    #[inline]
    pub fn new_with_token<Fut>(
        runtime: &Runtime,
        fut: Fut,
        cancel: CancelHandle,
        cancel_token: CancellationToken,
    ) -> Self
    where
        Fut: Future<Output = PyResult<T>> + Send + 'static,
    {
        Self::new(
            runtime,
            async move {
                tokio::select! {
                    result = fut => result,
                    _ = cancel_token.cancelled() => Err(CancelledError::new_err("Operation was cancelled: client has been closed")),
                }
            },
            cancel,
        )
    }
}

impl<T> Future for NoGIL<T>
where
    T: Send + 'static,
{
    type Output = PyResult<T>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        // A Python throw must win even when the Tokio task has already finished.
        if let Poll::Ready(exc) = this.cancel.poll_cancelled(cx) {
            this.handle.abort();
            return Poll::Ready(Err(Python::attach(|py| {
                PyErr::from_value(exc.into_bound(py))
            })));
        }

        let waker = cx.waker();
        Python::attach(|py| {
            py.detach(
                || match poll_with_guard(this.handle, &mut Context::from_waker(waker)) {
                    Poll::Ready(Ok(result)) => Poll::Ready(result),
                    Poll::Ready(Err(e)) => Poll::Ready(Err(PyRuntimeError::new_err(e.to_string()))),
                    Poll::Pending => Poll::Pending,
                },
            )
        })
    }
}

/// Run network work on its selected worker; only wait for completion on the caller.
/// The caller must be detached from Python and outside an async Tokio context.
pub fn block_on<F, T>(runtime: &Runtime, future: F) -> PyResult<T>
where
    F: Future<Output = PyResult<T>> + Send + 'static,
    T: Send + 'static,
{
    let owner = runtime.clone();
    let task = runtime.handle().spawn(async move {
        let _owner = owner;
        future.await
    });
    runtime
        .handle()
        .block_on(AbortOnDropHandle::new(task))
        .map_err(|err| PyRuntimeError::new_err(err.to_string()))?
}

/// Protect Python wakers retained by futures that can be woken from Rust threads.
pub fn poll_with_guard<F: Future>(future: Pin<&mut F>, cx: &mut Context<'_>) -> Poll<F::Output> {
    let waker = Waker::from(Arc::new(GuardedWaker(cx.waker().clone())));
    future.poll(&mut Context::from_waker(&waker))
}

// ===== impl GuardedWaker =====

impl Wake for GuardedWaker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        // PyO3's nested attach reuses this attachment. If Python is unavailable,
        // skip the wake instead of invoking its infallible attachment path.
        Python::try_attach(|_| self.0.wake_by_ref());
    }
}
