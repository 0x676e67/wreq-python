use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use pin_project_lite::pin_project;
use pyo3::{
    coroutine::CancelHandle,
    exceptions::{PyRuntimeError, asyncio::CancelledError},
    prelude::*,
};
use tokio_util::{sync::CancellationToken, task::AbortOnDropHandle};

pin_project! {
    /// A future that allows Python threads to run while it is being polled or executed.
    /// It also handles cancellation and spawns the task in tokio runtime.
    pub struct NoGIL<T> {
        #[pin]
        handle: AbortOnDropHandle<PyResult<T>>,
        cancel: CancelHandle,
    }
}

impl<T> NoGIL<T>
where
    T: Send + 'static,
{
    /// Create [`NoGIL`] from a future
    #[inline]
    pub fn new<Fut>(fut: Fut, cancel: CancelHandle) -> Self
    where
        Fut: Future<Output = PyResult<T>> + Send + 'static,
    {
        Self {
            handle: AbortOnDropHandle::new(pyo3_async_runtimes::tokio::get_runtime().spawn(fut)),
            cancel,
        }
    }

    /// Create [`NoGIL`] from a future and a cancellation token
    #[inline]
    pub fn new_with_token<Fut>(
        fut: Fut,
        cancel: CancelHandle,
        cancel_token: CancellationToken,
    ) -> Self
    where
        Fut: Future<Output = PyResult<T>> + Send + 'static,
    {
        Self::new(
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
            py.detach(|| match this.handle.poll(&mut Context::from_waker(waker)) {
                Poll::Ready(Ok(result)) => Poll::Ready(result),
                Poll::Ready(Err(e)) => Poll::Ready(Err(PyRuntimeError::new_err(e.to_string()))),
                Poll::Pending => Poll::Pending,
            })
        })
    }
}
