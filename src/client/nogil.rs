//! Waits for the blocking API. Network work runs on the client's [`Runtime`] while the
//! caller waits detached from Python, so other threads keep the interpreter.

use std::{
    future::{Future, poll_fn},
    pin::pin,
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
    thread::{self, Thread},
};

use futures_util::FutureExt;
use pyo3::{exceptions::PyRuntimeError, prelude::*};

use super::body::Pulls;
use crate::runtime::Runtime;

/// Run network work on its selected worker; only wait for completion on the caller.
/// The caller must be detached from Python and outside an async Tokio context.
/// Its runtime borrow keeps the workers alive until the join completes.
pub fn block_on<F, T>(runtime: &Runtime, future: F) -> PyResult<T>
where
    F: Future<Output = PyResult<T>> + Send + 'static,
    T: Send + 'static,
{
    runtime
        .handle()
        .block_on(runtime.handle().spawn(future))
        .map_err(|err| PyRuntimeError::new_err(err.to_string()))?
}

/// Poll `future` on the caller instead of spawning it: return still attached when it is
/// ready at once, as a buffered body usually is, and otherwise wait detached.
/// The caller must be outside an async Tokio context.
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

/// Like [`block_on`] from an attached caller, which also serves the request body's `pulls`.
///
/// The first pull is served right after sending, before releasing the GIL, and later ones
/// while the request runs, each stopping once it completes. The wait parks outside the
/// runtime, so the iterator may itself make blocking requests.
pub fn block_on_feeding<F, T>(
    py: Python<'_>,
    runtime: &Runtime,
    future: F,
    pulls: Option<Pulls>,
) -> PyResult<T>
where
    F: Future<Output = PyResult<T>> + Send + 'static,
    T: Send + 'static,
{
    let Some(mut pulls) = pulls else {
        return py.detach(|| block_on(runtime, future));
    };

    let mut task = runtime.handle().spawn(future);
    if pulls
        .try_recv()
        .is_ok_and(|pull| pull.serve(py, || task.is_finished()))
    {
        // The body is fully queued and asks for nothing more; watching `pulls` would only
        // wake this thread when the body drops them.
        return py
            .detach(|| runtime.handle().block_on(task))
            .map_err(|err| PyRuntimeError::new_err(err.to_string()))?;
    }

    py.detach(|| {
        let output = park_on(poll_fn(|cx| {
            loop {
                if let Poll::Ready(output) = task.poll_unpin(cx) {
                    return Poll::Ready(output);
                }
                let Poll::Ready(Some(pull)) = pulls.poll_recv(cx) else {
                    return Poll::Pending;
                };
                Python::attach(|py| pull.serve(py, || task.is_finished()));
            }
        }));
        // Dropping the receiver could strand a pull sent at the same moment, and the body
        // would wait on it forever. Draining after `close` drops every pull unserved, so the
        // body moves to the blocking pool.
        pulls.close();
        park_on(async { while pulls.recv().await.is_some() {} });
        output
    })
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
