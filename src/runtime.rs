//! The Tokio runtime behind clients, and how calls wait on it.
//!
//! Requests, and async reads that must wait, run as tasks on the client's selected worker
//! ([`Runtime::block_on_task`], [`Runtime::run_task`]); blocking reads are polled by the
//! caller ([`Runtime::block_on`], [`Runtime::block_on_eager`]). These blocking calls release
//! the GIL while they wait; on a current-thread runtime they drive it and run the request
//! themselves instead of spawning it.
//!
//! Every other runtime operation goes through [`Runtime`] too; `clippy.toml` rejects direct
//! Tokio handles, spawns and yields elsewhere.
#![allow(clippy::disallowed_methods, clippy::disallowed_types)]

use std::{
    cell::{Cell, RefCell},
    future::Future,
    pin::pin,
    sync::{Arc, OnceLock},
    task::Context,
    thread,
    time::Duration,
};

use futures_util::FutureExt;
use pingora_runtime::{BlockingPoolOpts, Runtime as PingoraRuntime, RuntimeBuilder};
use pyo3::{
    exceptions::{PyRuntimeError, PyValueError},
    prelude::*,
};
use tokio::{
    runtime::{Builder, Handle},
    task::{self, JoinError, JoinHandle},
};
use tokio_util::task::AbortOnDropHandle;

/// How a [`Runtime`] schedules client work.
#[pyclass(eq, eq_int, frozen, from_py_object)]
#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum Scheduler {
    /// Worker threads share client work and steal it from each other.
    WORK_STEALING,
    /// Each worker runs its own single-thread runtime, and a client stays on one worker.
    PER_WORKER,
    /// No worker threads: blocking calls drive its IO, taking turns when threads share it,
    /// and nothing runs between calls. Best with a client and runtime per thread; only
    /// blocking clients can use it.
    CURRENT_THREAD,
}

/// Shared Tokio runtime, released after its clients and active work are dropped.
#[derive(Clone)]
#[pyclass(frozen, skip_from_py_object)]
pub struct Runtime {
    /// Shared owner; `None` only once `Drop` has taken it to shut the runtime down.
    inner: Option<Arc<Inner>>,
    /// The worker this copy runs on; per-worker clients keep their tasks there.
    handle: Handle,
}

/// The runtime behind [`Runtime`].
enum Inner {
    Workers(PingoraRuntime),
    /// Driven only by threads in a blocking call, through [`Driving::drive`].
    CurrentThread(tokio::runtime::Runtime),
}

/// Marks the calling thread as driving a current-thread runtime until dropped.
struct Driving;

thread_local! {
    static DRIVING: Cell<bool> = const { Cell::new(false) };
    /// An error an upload iterator run on this thread chose to re-raise from the blocking
    /// call this thread is driving, in place of that call's output.
    static INTERRUPT: RefCell<Option<PyErr>> = const { RefCell::new(None) };
}

// ===== impl Runtime =====

impl Runtime {
    /// Whether blocking calls drive this runtime on their own thread.
    pub fn is_current_thread(&self) -> bool {
        matches!(self.inner.as_deref(), Some(Inner::CurrentThread(_)))
    }

    /// Share the runtime and select a worker for a new client.
    pub fn select(&self) -> PyResult<Self> {
        let inner = self
            .inner
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("Runtime is unavailable"))?;
        Ok(Self {
            handle: inner.handle(),
            inner: Some(inner.clone()),
        })
    }

    /// Poll `future` once inside the runtime, where a read timeout can start its timer;
    /// `None` if it must wait. A pending future is dropped, so pass `&mut` to keep it.
    pub fn poll_now<F: Future>(&self, future: F) -> Option<F::Output> {
        let _runtime = self.handle.enter();
        future.now_or_never()
    }

    /// Poll `future` to completion on the caller, detached from Python. A current-thread
    /// runtime runs its tasks and IO here meanwhile.
    pub fn block_on<F, T>(&self, py: Python<'_>, future: F) -> PyResult<T>
    where
        F: Future<Output = PyResult<T>> + Send,
        T: Send,
    {
        py.detach(|| self.wait(future))
    }

    /// Like [`block_on`](Self::block_on), but poll once still attached first and return
    /// without releasing the GIL when ready, as a buffered body or frame usually is.
    pub fn block_on_eager<F, T>(&self, py: Python<'_>, future: F) -> PyResult<T>
    where
        F: Future<Output = PyResult<T>> + Send,
        T: Send,
    {
        // Refused before the first poll, which a later refusal would leave half done.
        Self::refuse_nested()?;
        let mut future = pin!(future);
        match self.poll_now(future.as_mut()) {
            Some(output) => output,
            None => self.block_on(py, future),
        }
    }

    /// Run `future` as a task on the selected worker while the caller waits detached. A
    /// current-thread runtime's only worker is its caller, which drives the future itself.
    pub fn block_on_task<F, T>(&self, py: Python<'_>, future: F) -> PyResult<T>
    where
        F: Future<Output = PyResult<T>> + Send + 'static,
        T: Send + 'static,
    {
        py.detach(|| self.wait_task(future))
    }

    /// Run `future` as a task on the selected worker and wait for it from any executor.
    /// Dropping the wait aborts the task, which keeps the runtime alive until it ends.
    #[inline]
    pub async fn run_task<F, T>(self, future: F) -> PyResult<T>
    where
        F: Future<Output = PyResult<T>> + Send + 'static,
        T: Send + 'static,
    {
        let task = self.handle.clone().spawn(async move {
            let _owner = self;
            future.await
        });
        AbortOnDropHandle::new(task).await.map_err(join_error)?
    }

    /// Spawn `future` on the selected worker. Dropping the handle detaches the task, which
    /// does not keep the runtime alive.
    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.handle.spawn(future)
    }

    /// [`block_on`](Self::block_on) for a caller already detached. It refuses a nested call
    /// itself, where Tokio would panic.
    fn wait<T>(&self, future: impl Future<Output = PyResult<T>>) -> PyResult<T> {
        Self::refuse_nested()?;
        match self.inner.as_deref() {
            Some(Inner::CurrentThread(runtime)) => Driving::drive(runtime, future),
            _ => self.handle.block_on(future),
        }
    }

    /// [`block_on_task`](Self::block_on_task) for a caller already detached; refused before
    /// the spawn, so a nested request starts nothing.
    fn wait_task<F, T>(&self, future: F) -> PyResult<T>
    where
        F: Future<Output = PyResult<T>> + Send + 'static,
        T: Send + 'static,
    {
        Self::refuse_nested()?;
        match self.inner.as_deref() {
            Some(Inner::CurrentThread(runtime)) => Driving::drive(runtime, future),
            _ => self
                .handle
                .block_on(self.handle.spawn(future))
                .map_err(join_error)?,
        }
    }
}

/// Operations on the calling thread: the shared runtime, the runtime the thread is in, and
/// the state of a current-thread runtime it drives.
impl Runtime {
    /// The shared runtime, created on first use and retained for the process lifetime.
    pub fn shared() -> &'static Runtime {
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

    /// Run `f` on the blocking pool of the runtime this thread is in, or the shared runtime's
    /// outside one, so a drop on a client's runtime does not start the shared one.
    pub fn spawn_blocking<F, R>(f: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        current().spawn_blocking(f)
    }

    /// Wait for `future` detached from Python on a blocking-pool thread of a runtime whose
    /// workers drive its IO and timers; a current-thread runtime's pool has none. Unlike
    /// [`block_on`](Self::block_on), it neither refuses nested calls nor needs a `PyResult`;
    /// create timers inside `future`.
    pub fn block_on_in_pool<F>(py: Python<'_>, future: F) -> F::Output
    where
        F: Future + Send,
        F::Output: Send,
    {
        let handle = current();
        py.detach(|| handle.block_on(future))
    }

    /// Wake the task after the runtime polls its IO and other tasks, or at once outside a
    /// runtime.
    pub fn defer_wake(cx: &mut Context<'_>) {
        let _ = pin!(task::yield_now()).poll(cx);
    }

    /// Whether this thread drives a current-thread runtime, which then runs the Python code
    /// its tasks call, such as upload iterators.
    pub fn driving() -> bool {
        DRIVING.get()
    }

    /// Re-raise `err` from the blocking call this thread is driving, once it returns.
    pub fn interrupt(err: PyErr) {
        INTERRUPT.set(Some(err));
    }

    /// Fail a blocking wait on a thread driving a current-thread runtime, where Tokio would
    /// panic, as when an upload iterator it reads sends a blocking request.
    pub fn refuse_nested() -> PyResult<()> {
        if Self::driving() {
            return Err(PyRuntimeError::new_err(
                "Cannot make a blocking call from code a CURRENT_THREAD runtime is running",
            ));
        }
        Ok(())
    }
}

impl From<Inner> for Runtime {
    fn from(runtime: Inner) -> Self {
        let handle = runtime.handle();
        Self {
            inner: Some(Arc::new(runtime)),
            handle,
        }
    }
}

impl From<PingoraRuntime> for Runtime {
    fn from(runtime: PingoraRuntime) -> Self {
        Inner::Workers(runtime).into()
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
    /// Create the runtime and start its workers, if any.
    /// Workers default to CPU parallelism and must be None with CURRENT_THREAD.
    /// Thread names default to the package name.
    /// Thread counts must be positive; thread_keep_alive is a nonnegative timedelta.
    #[new]
    #[pyo3(signature = (
        *,
        scheduler = Scheduler::WORK_STEALING,
        workers = None,
        thread_name = None,
        max_blocking_threads = None,
        thread_keep_alive = None,
    ))]
    fn new(
        py: Python<'_>,
        scheduler: Scheduler,
        workers: Option<usize>,
        thread_name: Option<&str>,
        max_blocking_threads: Option<usize>,
        thread_keep_alive: Option<Duration>,
    ) -> PyResult<Self> {
        if max_blocking_threads == Some(0) {
            return Err(PyValueError::new_err("Invalid runtime thread counts"));
        }
        let thread_name = thread_name.unwrap_or(env!("CARGO_PKG_NAME"));
        if thread_name.contains('\0') {
            return Err(PyValueError::new_err("thread_name must not contain NUL"));
        }

        Ok(match scheduler {
            Scheduler::CURRENT_THREAD => {
                if workers.is_some() {
                    return Err(PyValueError::new_err(
                        "CURRENT_THREAD runtimes have no workers",
                    ));
                }
                let mut builder = Builder::new_current_thread();
                builder.enable_all().thread_name(thread_name);
                if let Some(max_threads) = max_blocking_threads {
                    builder.max_blocking_threads(max_threads);
                }
                if let Some(keep_alive) = thread_keep_alive {
                    builder.thread_keep_alive(keep_alive);
                }
                let runtime = builder
                    .build()
                    .map_err(|err| PyRuntimeError::new_err(err.to_string()))?;
                Inner::CurrentThread(runtime).into()
            }
            _ => {
                let workers = workers.unwrap_or_else(parallelism);
                if workers == 0
                    || workers
                        .checked_add(max_blocking_threads.unwrap_or(512))
                        .is_none()
                {
                    return Err(PyValueError::new_err("Invalid runtime thread counts"));
                }
                py.detach(|| {
                    RuntimeBuilder::new(workers, thread_name)
                        .work_steal(scheduler == Scheduler::WORK_STEALING)
                        .blocking_pool_opts(BlockingPoolOpts {
                            max_threads: max_blocking_threads,
                            thread_keep_alive,
                        })
                        .build()
                        .into()
                })
            }
        })
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        // The final owner may be released on a worker or while holding the GIL.
        if let Some(runtime) = self.inner.take().and_then(Arc::into_inner) {
            match runtime {
                Inner::Workers(PingoraRuntime::Steal { runtime, .. })
                | Inner::CurrentThread(runtime) => runtime.shutdown_background(),
                Inner::Workers(PingoraRuntime::NoSteal(runtime)) => drop(runtime),
            }
        }
    }
}

// ===== impl Inner =====

impl Inner {
    fn handle(&self) -> Handle {
        match self {
            // No-steal workers are lazy upstream; this starts them.
            Inner::Workers(runtime) => runtime.get_handle().clone(),
            Inner::CurrentThread(runtime) => runtime.handle().clone(),
        }
    }
}

// ===== impl Driving =====

impl Driving {
    /// Drive `runtime` on this thread until `future` completes, marked as driving it; an
    /// interrupt an upload iterator stashed meanwhile replaces the output.
    fn drive<T>(
        runtime: &tokio::runtime::Runtime,
        future: impl Future<Output = PyResult<T>>,
    ) -> PyResult<T> {
        DRIVING.set(true);
        let _driving = Driving;
        let output = runtime.block_on(future);
        INTERRUPT.take().map_or(output, Err)
    }
}

impl Drop for Driving {
    fn drop(&mut self) {
        DRIVING.set(false);
        // Left only by an unwinding call; a later call must not raise it.
        INTERRUPT.take();
    }
}

/// A task that panicked or was aborted, as a Python error.
fn join_error(err: JoinError) -> PyErr {
    PyRuntimeError::new_err(err.to_string())
}

/// Workers used when none are given: one per available CPU.
fn parallelism() -> usize {
    thread::available_parallelism().map_or(1, usize::from)
}

/// The runtime this thread is in, else the shared one.
fn current() -> Handle {
    Handle::try_current().unwrap_or_else(|_| Runtime::shared().handle.clone())
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
            let id = worker
                .wait_task(async {
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
    fn current_thread_runs_on_its_callers() {
        let current_thread = || {
            Runtime::from(Inner::CurrentThread(
                Builder::new_current_thread().enable_all().build().unwrap(),
            ))
        };
        let runtime = current_thread();
        // Spawned tasks and timers run on the blocked caller.
        let child = runtime
            .wait_task(async {
                Ok(tokio::spawn(async {
                    tokio::time::sleep(Duration::from_millis(1)).await;
                    thread::current().id()
                })
                .await
                .unwrap())
            })
            .unwrap();
        assert_eq!(child, thread::current().id());
        // A blocking call from code the runtime runs fails instead of panicking, and the
        // last owner of another runtime can be released there.
        let other = current_thread();
        let nested = runtime
            .wait(async {
                let nested = other.wait(async { Ok(()) }).is_err();
                drop(other);
                Ok(nested)
            })
            .unwrap();
        assert!(nested);
        assert!(!Runtime::driving());
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
            let handle = runtime.handle.clone();
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
