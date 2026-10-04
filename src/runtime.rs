use std::{
    sync::{Arc, OnceLock},
    thread,
    time::Duration,
};

use pingora_runtime::{BlockingPoolOpts, Runtime as PingoraRuntime, RuntimeBuilder};
use pyo3::{
    exceptions::{PyRuntimeError, PyValueError},
    prelude::*,
};
use tokio::runtime::Handle;

/// Shared Tokio runtime, released after its clients and active work are dropped.
#[derive(Clone)]
#[pyclass(frozen, skip_from_py_object)]
pub struct Runtime {
    /// Shared owner; `None` only once `Drop` has taken it to shut the runtime down.
    inner: Option<Arc<PingoraRuntime>>,
    /// The worker this copy runs on; without work stealing its client's tasks stay there.
    handle: Handle,
}

impl Runtime {
    /// Borrow the selected worker's handle without changing the selection.
    pub fn handle(&self) -> &Handle {
        &self.handle
    }

    /// Share the runtime and select a worker for a new client.
    pub fn select(&self) -> PyResult<Self> {
        let inner = self
            .inner
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("Runtime is unavailable"))?;
        Ok(Self {
            inner: Some(inner.clone()),
            handle: inner.get_handle().clone(),
        })
    }
}

impl From<PingoraRuntime> for Runtime {
    fn from(runtime: PingoraRuntime) -> Self {
        // No-steal workers are lazy upstream. Start them before sharing the runtime.
        let handle = runtime.get_handle().clone();
        Self {
            inner: Some(Arc::new(runtime)),
            handle,
        }
    }
}

impl FromPyObject<'_, '_> for Runtime {
    type Error = PyErr;

    fn extract(value: Borrowed<'_, '_, PyAny>) -> PyResult<Self> {
        Ok(value.extract::<PyRef<'_, Self>>()?.clone())
    }
}

#[pymethods]
impl Runtime {
    /// Create and start the runtime's workers.
    /// Without work stealing, each client stays on one worker (not CPU-pinned).
    /// Workers default to CPU parallelism and names to the package name.
    /// Thread counts must be positive; thread_keep_alive is a nonnegative timedelta.
    #[new]
    #[pyo3(signature = (
        *,
        workers = None,
        work_steal = true,
        thread_name = None,
        max_blocking_threads = None,
        thread_keep_alive = None,
    ))]
    fn new(
        py: Python<'_>,
        workers: Option<usize>,
        work_steal: bool,
        thread_name: Option<&str>,
        max_blocking_threads: Option<usize>,
        thread_keep_alive: Option<Duration>,
    ) -> PyResult<Self> {
        let workers = workers.unwrap_or_else(parallelism);
        if workers == 0
            || max_blocking_threads == Some(0)
            || workers
                .checked_add(max_blocking_threads.unwrap_or(512))
                .is_none()
        {
            return Err(PyValueError::new_err("Invalid runtime thread counts"));
        }

        let thread_name = thread_name.unwrap_or(env!("CARGO_PKG_NAME"));
        if thread_name.contains('\0') {
            return Err(PyValueError::new_err("thread_name must not contain NUL"));
        }

        Ok(py.detach(|| {
            RuntimeBuilder::new(workers, thread_name)
                .work_steal(work_steal)
                .blocking_pool_opts(BlockingPoolOpts {
                    max_threads: max_blocking_threads,
                    thread_keep_alive,
                })
                .build()
                .into()
        }))
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        // The final owner may be released on a worker or while holding the GIL.
        if let Some(runtime) = self.inner.take().and_then(Arc::into_inner) {
            match runtime {
                PingoraRuntime::Steal { runtime, .. } => runtime.shutdown_background(),
                PingoraRuntime::NoSteal(runtime) => drop(runtime),
            }
        }
    }
}

/// Workers used when none are given: one per available CPU.
fn parallelism() -> usize {
    thread::available_parallelism().map_or(1, usize::from)
}

/// Create the shared runtime on first use and retain it for the process lifetime.
pub fn get() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();

    fn create() -> Runtime {
        let runtime = RuntimeBuilder::new(parallelism(), env!("CARGO_PKG_NAME")).build();
        runtime.into()
    }

    if let Some(runtime) = RUNTIME.get() {
        return runtime;
    }

    // Never wait for another initializer while holding the interpreter.
    Python::try_attach(|py| py.detach(|| RUNTIME.get_or_init(create)))
        .unwrap_or_else(|| RUNTIME.get_or_init(create))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_worker() {
        let runtime = Runtime::from(
            RuntimeBuilder::new(2, "wreq-affinity")
                .work_steal(false)
                .build(),
        );
        let first = runtime.select().unwrap();
        let second = runtime.select().unwrap();
        let caller = std::thread::current().id();
        let mut ids = Vec::new();
        for worker in [&first, &second, &first.clone(), &second.clone()] {
            let id = crate::client::nogil::block_on(worker, async {
                let thread = std::thread::current().id();
                for _ in 0..8 {
                    tokio::task::yield_now().await;
                    assert_eq!(thread, std::thread::current().id());
                }
                let child = tokio::spawn(async { std::thread::current().id() })
                    .await
                    .unwrap();
                assert_eq!(thread, child);
                Ok(thread)
            })
            .unwrap();
            assert_ne!(id, caller);
            ids.push(id);
        }
        assert_eq!(ids[0], ids[2]);
        assert_eq!(ids[1], ids[3]);
        // Different clients may select the same worker; each selection stays fixed.
    }

    #[test]
    fn last_owner_can_be_released_on_a_worker() {
        struct NotifyOnDrop(std::sync::mpsc::Sender<()>);

        impl Drop for NotifyOnDrop {
            fn drop(&mut self) {
                let _ = self.0.send(());
            }
        }

        for steal in [false, true] {
            let runtime = Runtime::from(
                RuntimeBuilder::new(1, "wreq-drop")
                    .work_steal(steal)
                    .build(),
            );
            let weak = Arc::downgrade(runtime.inner.as_ref().unwrap());
            let handle = runtime.handle().clone();
            let (dropped, released) = std::sync::mpsc::channel();
            let guard = NotifyOnDrop(dropped);
            let background = handle.spawn(async move {
                let _guard = guard;
                std::future::pending::<()>().await;
            });
            let (tx, rx) = tokio::sync::oneshot::channel();
            let (done, wait) = std::sync::mpsc::channel();
            let owner = runtime.clone();
            let task = handle.spawn(async move {
                let _owner = owner;
                let _ = rx.await;
                done.send(()).unwrap();
            });
            drop(runtime);
            tx.send(()).unwrap();
            wait.recv_timeout(Duration::from_secs(5)).unwrap();
            released.recv_timeout(Duration::from_secs(5)).unwrap();
            assert!(weak.upgrade().is_none());
            handle.block_on(task).unwrap();
            drop(background);
        }
    }
}
