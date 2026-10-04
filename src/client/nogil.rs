use std::{
    future::{Future, poll_fn},
    pin::pin,
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
    thread::{self, Thread},
};

use futures_util::FutureExt;
use pyo3::{exceptions::PyRuntimeError, prelude::*};
use tokio::sync::mpsc::UnboundedReceiver;

use super::body::Pull;
use crate::runtime::Runtime;

/// Run network work on its selected worker; only wait for completion on the caller.
/// The caller must be detached from Python and outside an async Tokio context.
/// Its runtime borrow keeps the workers alive until the join completes.
pub fn block_on<F, T>(runtime: &Runtime, future: F) -> PyResult<T>
where
    F: Future<Output = PyResult<T>> + Send + 'static,
    T: Send + 'static,
{
    let task = runtime.handle().spawn(future);
    runtime
        .handle()
        .block_on(task)
        .map_err(|err| PyRuntimeError::new_err(err.to_string()))?
}

/// Run `future` on the caller, returning without releasing the GIL when it completes at
/// once and otherwise waiting detached. The caller must be outside an async Tokio context.
pub fn run<F, T>(py: Python<'_>, runtime: &Runtime, future: F) -> PyResult<T>
where
    F: Future<Output = PyResult<T>> + Send,
    T: Send,
{
    let handle = runtime.handle();
    let mut future = pin!(future);
    let ready = {
        let _runtime = handle.enter();
        future.as_mut().now_or_never()
    };
    match ready {
        Some(output) => output,
        None => py.detach(|| handle.block_on(future)),
    }
}

/// Like [`block_on`], serving the request body's `pulls` on the caller while it waits.
/// Waiting stays outside the runtime, so a pulled iterator may itself block on requests.
pub fn block_on_feeding<F, T>(
    runtime: &Runtime,
    future: F,
    mut pulls: UnboundedReceiver<Pull>,
) -> PyResult<T>
where
    F: Future<Output = PyResult<T>> + Send + 'static,
    T: Send + 'static,
{
    let mut task = runtime.handle().spawn(future);
    park_on(poll_fn(|cx| {
        while let Poll::Ready(Some(pull)) = pulls.poll_recv(cx) {
            pull.serve();
        }
        task.poll_unpin(cx)
    }))
    .map_err(|err| PyRuntimeError::new_err(err.to_string()))?
}

/// Poll `future` on the calling thread, parking it between wakes.
fn park_on<F: Future>(future: F) -> F::Output {
    struct Unpark(Thread);

    impl Wake for Unpark {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }

        fn wake_by_ref(self: &Arc<Self>) {
            self.0.unpark();
        }
    }

    let waker = Waker::from(Arc::new(Unpark(thread::current())));
    let mut cx = Context::from_waker(&waker);
    let mut future = pin!(future);
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut cx) {
            return output;
        }
        thread::park();
    }
}
