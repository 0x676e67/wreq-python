//! Waits for the blocking API. Network work runs on the client's [`Runtime`] while the
//! caller waits detached from Python, so other threads keep the interpreter.

use std::{future::Future, pin::pin};

use futures_util::FutureExt;
use pyo3::{exceptions::PyRuntimeError, prelude::*};

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
