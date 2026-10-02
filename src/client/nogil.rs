use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Condvar, Mutex, OnceLock, PoisonError},
    task::{Context, Poll, Wake, Waker},
};

use pyo3::{coroutine::CancelHandle, exceptions::PyRuntimeError, prelude::*};
use tokio_util::task::AbortOnDropHandle;

use crate::runtime::Runtime;

/// A future that allows Python threads to run while it is being polled or executed.
/// It also handles cancellation and spawns the task in tokio runtime.
pub struct NoGIL<T> {
    handle: AbortOnDropHandle<PyResult<T>>,
    cancel: CancelHandle,
}

struct GuardedWaker(Waker);

/// Wakes Python coroutines from a dedicated thread, attaching once per batch so
/// Tokio workers never wait for the interpreter while they own network tasks.
struct WakeRelay {
    /// The relay thread exists only in the process that started it.
    pid: u32,
    pending: Mutex<Vec<Waker>>,
    ready: Condvar,
}

// ===== impl NoGIL =====

impl<T> NoGIL<T>
where
    T: Send + 'static,
{
    /// Spawn internal work without a Python cancellation source.
    #[inline]
    pub fn new<Fut>(runtime: &Runtime, fut: Fut) -> Self
    where
        Fut: Future<Output = PyResult<T>> + Send + 'static,
    {
        Self::with_cancel(runtime, fut, CancelHandle::new())
    }

    /// Spawn with Python cancellation, keeping the runtime alive until the task ends.
    #[inline]
    pub fn with_cancel<Fut>(runtime: &Runtime, fut: Fut, cancel: CancelHandle) -> Self
    where
        Fut: Future<Output = PyResult<T>> + Send + 'static,
    {
        let owner = runtime.clone();
        // Spawning only queues the task, so it needs no detach from Python.
        let handle = runtime.handle().spawn(async move {
            let _owner = owner;
            fut.await
        });
        Self {
            handle: AbortOnDropHandle::new(handle),
            cancel,
        }
    }
}

impl<T> Future for NoGIL<T>
where
    T: Send + 'static,
{
    type Output = PyResult<T>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        // A Python throw must win even when the Tokio task has already finished.
        if let Poll::Ready(exc) = this.cancel.poll_cancelled(cx) {
            this.handle.abort();
            return Poll::Ready(Err(Python::attach(|py| {
                PyErr::from_value(exc.into_bound(py))
            })));
        }

        // Polling a join handle never blocks; keep the attachment.
        match poll_with_guard(Pin::new(&mut this.handle), cx) {
            Poll::Ready(Ok(result)) => Poll::Ready(result),
            Poll::Ready(Err(e)) => Poll::Ready(Err(PyRuntimeError::new_err(e.to_string()))),
            Poll::Pending => Poll::Pending,
        }
    }
}

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

/// Protect Python wakers retained by futures that can be woken from Rust threads.
pub fn poll_with_guard<F: Future>(future: Pin<&mut F>, cx: &mut Context<'_>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(&guard(cx.waker())))
}

/// Wrap a Python waker so Rust threads wake it through the relay.
pub fn guard(waker: &Waker) -> Waker {
    Waker::from(Arc::new(GuardedWaker(waker.clone())))
}

// ===== impl GuardedWaker =====

impl Wake for GuardedWaker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        WakeRelay::wake(self.0.clone());
    }
}

// ===== impl WakeRelay =====

impl WakeRelay {
    /// Queue a Python waker, falling back to waking inline without a relay thread.
    /// A forked child never touches the relay state inherited from its parent.
    fn wake(waker: Waker) {
        static RELAY: OnceLock<Option<&'static WakeRelay>> = OnceLock::new();
        let relay = RELAY.get_or_init(|| {
            let relay: &'static WakeRelay = Box::leak(Box::new(WakeRelay {
                pid: std::process::id(),
                pending: Mutex::new(Vec::new()),
                ready: Condvar::new(),
            }));
            std::thread::Builder::new()
                .name("wreq-python-waker".into())
                .spawn(move || relay.run())
                .ok()
                .map(|_| relay)
        });

        match relay {
            Some(relay) if relay.pid == std::process::id() => {
                let mut pending = relay.pending.lock().unwrap_or_else(PoisonError::into_inner);
                pending.push(waker);
                if pending.len() == 1 {
                    relay.ready.notify_one();
                }
            }
            _ => Self::wake_all(&mut vec![waker]),
        }
    }

    fn run(&self) {
        let mut batch = Vec::new();
        loop {
            {
                let mut pending = self.pending.lock().unwrap_or_else(PoisonError::into_inner);
                while pending.is_empty() {
                    pending = self
                        .ready
                        .wait(pending)
                        .unwrap_or_else(PoisonError::into_inner);
                }
                std::mem::swap(&mut *pending, &mut batch);
            }
            Self::wake_all(&mut batch);
        }
    }

    /// Wake a batch under one attachment. If Python is unavailable, skip the
    /// wakes instead of invoking PyO3's infallible attachment path.
    fn wake_all(batch: &mut Vec<Waker>) {
        Python::try_attach(|_| batch.drain(..).for_each(Waker::wake));
        batch.clear();
    }
}
