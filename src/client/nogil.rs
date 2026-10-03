use std::future::Future;

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
    let task = runtime.handle().spawn(future);
    runtime
        .handle()
        .block_on(task)
        .map_err(|err| PyRuntimeError::new_err(err.to_string()))?
}
