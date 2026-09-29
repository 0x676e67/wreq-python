use std::{
    sync::{Arc, OnceLock},
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
pub struct Runtime(Option<Arc<PingoraRuntime>>);

impl Runtime {
    /// Borrow a Tokio handle; in no-steal mode, select once per client and retain it.
    pub fn handle(&self) -> PyResult<&Handle> {
        self.0
            .as_deref()
            .map(PingoraRuntime::get_handle)
            .ok_or_else(|| PyRuntimeError::new_err("Runtime is unavailable"))
    }
}

impl From<PingoraRuntime> for Runtime {
    fn from(runtime: PingoraRuntime) -> Self {
        // No-steal workers are lazy upstream. Start them before sharing the runtime.
        runtime.get_handle();
        Self(Some(Arc::new(runtime)))
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
        let workers =
            workers.unwrap_or_else(|| std::thread::available_parallelism().map_or(1, usize::from));
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
        if let Some(runtime) = self.0.take().and_then(Arc::into_inner) {
            match runtime {
                PingoraRuntime::Steal { runtime, .. } => runtime.shutdown_background(),
                PingoraRuntime::NoSteal(runtime) => drop(runtime),
            }
        }
    }
}

/// Create the shared runtime on first use and retain it for the process lifetime.
pub fn get() -> &'static (Runtime, Handle) {
    static RUNTIME: OnceLock<(Runtime, Handle)> = OnceLock::new();

    fn create() -> (Runtime, Handle) {
        let workers = std::thread::available_parallelism().map_or(1, usize::from);
        let runtime = RuntimeBuilder::new(workers, env!("CARGO_PKG_NAME")).build();
        let handle = runtime.get_handle().clone();
        (runtime.into(), handle)
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
        let first = runtime.handle().unwrap();
        let second = runtime.handle().unwrap();
        let caller = std::thread::current().id();
        let mut ids = Vec::new();
        for handle in [first, second, first, second] {
            let id = crate::client::nogil::block_on(&runtime, handle, async {
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
            let weak = Arc::downgrade(runtime.0.as_ref().unwrap());
            let handle = runtime.handle().unwrap().clone();
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
