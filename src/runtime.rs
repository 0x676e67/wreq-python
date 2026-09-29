use std::{
    future::Future,
    sync::{Arc, Mutex, MutexGuard, OnceLock},
    time::Duration,
};

use pingora_runtime::{BlockingPoolOpts, Runtime as PingoraRuntime, RuntimeBuilder};
use pyo3::{
    exceptions::{PyRuntimeError, PyValueError},
    prelude::*,
};
use tokio::{runtime::Handle, task::JoinHandle};
use tokio_util::task::AbortOnDropHandle;

/// Tokio workers shared explicitly or through the process-wide default instance.
#[derive(Clone)]
#[pyclass(frozen, skip_from_py_object, module = "wreq.runtime")]
pub struct Runtime(Arc<Inner>);

struct Inner {
    workers: usize,
    work_steal: bool,
    thread_name: String,
    max_blocking_threads: Option<usize>,
    thread_keep_alive: Option<Duration>,
    global: bool,
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    runtime: Option<PingoraRuntime>,
    closed: bool,
    users: usize,
    next_worker: usize,
}

/// A fixed worker selection shared by a client, its responses and active tasks.
#[derive(Clone)]
pub struct Executor(Arc<Lease>);

struct Lease {
    runtime: Runtime,
    worker: usize,
    handle: OnceLock<Handle>,
}

// ===== impl Runtime =====

impl Runtime {
    fn global() -> Self {
        static RUNTIME: OnceLock<Runtime> = OnceLock::new();
        RUNTIME
            .get_or_init(|| {
                Self(Arc::new(Inner {
                    workers: automatic_workers(),
                    work_steal: true,
                    thread_name: "wreq".into(),
                    max_blocking_threads: None,
                    thread_keep_alive: None,
                    global: true,
                    state: Mutex::default(),
                }))
            })
            .clone()
    }

    pub fn bind(&self) -> PyResult<Executor> {
        let mut state = self.0.lock();
        if state.closed {
            return Err(PyRuntimeError::new_err("Runtime is closed"));
        }
        Ok(self.lease(&mut state))
    }

    fn lease(&self, state: &mut State) -> Executor {
        let worker = state.next_worker;
        state.next_worker = (worker + 1) % self.0.workers;
        state.users += 1;
        Executor(Arc::new(Lease {
            runtime: self.clone(),
            worker,
            handle: OnceLock::new(),
        }))
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
    /// Configure workers; OS threads start lazily when first used.
    /// Without work stealing, each client stays on one worker (not CPU-pinned).
    #[new]
    #[pyo3(signature = (*, workers=None, work_steal=true, thread_name="wreq", max_blocking_threads=None, thread_keep_alive=None))]
    fn new(
        workers: Option<usize>,
        work_steal: bool,
        thread_name: &str,
        max_blocking_threads: Option<usize>,
        thread_keep_alive: Option<f64>,
    ) -> PyResult<Self> {
        let workers = workers.unwrap_or_else(automatic_workers);
        if workers == 0
            || max_blocking_threads == Some(0)
            || workers
                .checked_add(max_blocking_threads.unwrap_or(512))
                .is_none()
        {
            return Err(PyValueError::new_err("Invalid runtime thread counts"));
        }
        if thread_name.contains('\0') {
            return Err(PyValueError::new_err("thread_name must not contain NUL"));
        }
        let thread_keep_alive = thread_keep_alive.map(duration).transpose()?;
        Ok(Self(Arc::new(Inner {
            workers,
            work_steal,
            thread_name: thread_name.into(),
            max_blocking_threads,
            thread_keep_alive,
            global: false,
            state: Mutex::default(),
        })))
    }

    /// Return the lazy, shared multi-thread runtime.
    #[staticmethod]
    #[pyo3(name = "default")]
    fn shared() -> Self {
        Self::global()
    }

    #[getter]
    fn workers(&self) -> usize {
        self.0.workers
    }
    #[getter]
    fn work_steal(&self) -> bool {
        self.0.work_steal
    }
    #[getter]
    fn thread_name(&self) -> &str {
        &self.0.thread_name
    }
    #[getter]
    fn max_blocking_threads(&self) -> Option<usize> {
        self.0.max_blocking_threads
    }
    #[getter]
    fn thread_keep_alive(&self) -> Option<f64> {
        self.0.thread_keep_alive.map(|d| d.as_secs_f64())
    }
    #[getter]
    fn closed(&self) -> bool {
        self.0.lock().closed
    }

    /// Shut down an unused custom runtime. Release all clients and responses first.
    /// Timeout is seconds per worker; already running blocking work may outlive it.
    fn shutdown_timeout(&self, py: Python<'_>, timeout: f64) -> PyResult<()> {
        let timeout = duration(timeout)?;
        py.detach(|| {
            let runtime = {
                let mut state = self.0.lock();
                if self.0.global {
                    return Err(PyRuntimeError::new_err(
                        "The default Runtime cannot be shut down",
                    ));
                }
                if state.users != 0 {
                    return Err(PyRuntimeError::new_err(
                        "Runtime is in use; release its clients, responses and tasks first",
                    ));
                }
                state.closed = true;
                state.runtime.take()
            };
            if let Some(runtime) = runtime {
                runtime.shutdown_timeout(timeout);
            }
            Ok(())
        })
    }
}

// ===== impl Inner =====

impl Inner {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl Drop for Inner {
    fn drop(&mut self) {
        if let Some(runtime) = self
            .state
            .get_mut()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .runtime
            .take()
        {
            // Never synchronously wait on our own worker or while holding Python's GIL.
            match runtime {
                PingoraRuntime::Steal { runtime, .. } => runtime.shutdown_background(),
                // Dropping the controls wakes Pingora's dedicated driver threads.
                PingoraRuntime::NoSteal(runtime) => drop(runtime),
            }
        }
    }
}

// ===== impl Executor =====

impl Default for Executor {
    fn default() -> Self {
        let runtime = Runtime::global();
        let mut state = runtime.0.lock();
        runtime.lease(&mut state)
    }
}

impl Executor {
    pub fn runtime(&self) -> Runtime {
        self.0.runtime.clone()
    }

    pub fn handle(&self) -> Handle {
        self.0
            .handle
            .get_or_init(|| {
                let inner = &self.0.runtime.0;
                let mut state = inner.lock();
                let runtime = state.runtime.get_or_insert_with(|| {
                    RuntimeBuilder::new(inner.workers, &inner.thread_name)
                        .work_steal(inner.work_steal)
                        .blocking_pool_opts(BlockingPoolOpts {
                            max_threads: inner.max_blocking_threads,
                            thread_keep_alive: inner.thread_keep_alive,
                        })
                        .build()
                });
                // Serialize lazy initialization and keep the selected no-steal worker stable.
                match runtime {
                    PingoraRuntime::NoSteal(runtime) => {
                        runtime.get_runtime_at(self.0.worker).clone()
                    }
                    runtime => runtime.get_handle().clone(),
                }
            })
            .clone()
    }

    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let owner = self.clone();
        self.handle().spawn(async move {
            let _owner = owner;
            future.await
        })
    }

    pub fn spawn_blocking<F, T>(&self, function: F) -> JoinHandle<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        let owner = self.clone();
        self.handle().spawn_blocking(move || {
            let _owner = owner;
            function()
        })
    }

    /// Only the join is polled on the caller; network work stays on our worker.
    pub fn block_on<F, T>(&self, future: F) -> PyResult<T>
    where
        F: Future<Output = PyResult<T>> + Send + 'static,
        T: Send + 'static,
    {
        self.handle()
            .block_on(AbortOnDropHandle::new(self.spawn(future)))
            .map_err(|err| PyRuntimeError::new_err(err.to_string()))?
    }
}

// ===== impl Lease =====

impl Drop for Lease {
    fn drop(&mut self) {
        self.runtime.0.lock().users -= 1;
    }
}

pub fn get() -> PyResult<Executor> {
    Runtime::global().bind()
}

fn automatic_workers() -> usize {
    std::env::var("TOKIO_WORKER_THREADS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|n| *n > 0)
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(1, usize::from))
}

fn duration(seconds: f64) -> PyResult<Duration> {
    Duration::try_from_secs_f64(seconds).map_err(|_| {
        PyValueError::new_err("Duration must be finite, nonnegative and representable")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_workers_and_lifetime() {
        let runtime = Runtime::new(Some(2), false, "wreq-affinity", Some(2), None).unwrap();
        let first = runtime.bind().unwrap();
        let second = runtime.bind().unwrap();
        assert!(runtime.0.lock().runtime.is_none());
        let caller = std::thread::current().id();
        let mut ids = Vec::new();
        for executor in [&first, &second, &first, &second] {
            let id = executor
                .block_on(async {
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
        assert_ne!(ids[0], ids[1]);
        assert_eq!(runtime.0.lock().users, 2);
        drop((first, second));
        assert_eq!(runtime.0.lock().users, 0);
    }

    #[test]
    fn last_owner_can_be_released_on_a_worker() {
        for steal in [false, true] {
            let runtime = Runtime::new(Some(1), steal, "wreq-drop", None, None).unwrap();
            let weak = Arc::downgrade(&runtime.0);
            let executor = runtime.bind().unwrap();
            let (tx, rx) = tokio::sync::oneshot::channel();
            let (done, wait) = std::sync::mpsc::channel();
            let task = executor.spawn(async move {
                let _ = rx.await;
                done.send(()).unwrap();
            });
            drop((runtime, executor));
            tx.send(()).unwrap();
            wait.recv_timeout(Duration::from_secs(5)).unwrap();
            let deadline = std::time::Instant::now() + Duration::from_secs(5);
            while weak.upgrade().is_some() && std::time::Instant::now() < deadline {
                std::thread::yield_now();
            }
            assert!(weak.upgrade().is_none());
            drop(task);
        }
    }
}
