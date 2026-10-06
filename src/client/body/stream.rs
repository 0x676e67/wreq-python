//! Request bodies streamed from Python iterators and async generators.

use std::{
    mem,
    pin::{Pin, pin},
    sync::{
        Arc, Mutex, MutexGuard, PoisonError,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll},
    thread::{self, ThreadId},
    time::Duration,
};

use bytes::Bytes;
use futures_util::Stream;
use pyo3::{
    IntoPyObjectExt,
    exceptions::{PyException, PyKeyboardInterrupt, PyRuntimeError, PyStopIteration},
    intern,
    prelude::*,
    sync::PyOnceLock,
    types::{PyIterator, PyString},
};
use tokio::{
    runtime::Handle,
    sync::{OwnedSemaphorePermit, Semaphore, TryAcquireError, mpsc},
    task::{spawn_blocking, yield_now},
    time,
};

use crate::{
    coroutine::{self, Coroutine},
    extractor::{Binary, Text},
    runtime,
};

type Item = PyResult<PyBytesLike>;

/// A request body chunk: `bytes` or `bytearray` data, or a `str` sent as UTF-8.
pub enum PyBytesLike {
    Bytes(Binary),
    String(Text),
}

/// A request body read from a Python iterator or async generator, for `body=` and
/// multipart parts.
pub struct PyStream(Source);

/// The source behind a [`PyStream`].
enum Source {
    Sync(SyncStream),
    Async(PyAsyncStream),
}

/// A request body from a Python iterator.
///
/// Worker runtimes read the iterator on Tokio's blocking pool: neither a worker nor a
/// blocked caller runs it, so a slow `__next__` cannot delay the response, a timeout or
/// cancellation. A current-thread runtime reads it on its driving thread instead, which
/// waits for the request anyway.
enum SyncStream {
    /// Not polled yet; the first poll picks where the iterator is read. `owner` passed the
    /// iterator to the request.
    Idle {
        iter: Py<PyAny>,
        owner: ThreadId,
    },
    /// Read on the thread driving a current-thread runtime; `flushed` once a poll has
    /// returned to the connection since the last item.
    Inline {
        iter: Py<PyAny>,
        owner: ThreadId,
        flushed: bool,
    },
    Pumped(Pumped),
    /// The inline iterator ended or raised.
    Done,
}

/// An iterator read by pump tasks on the blocking pool, up to [`UPLOAD_BUDGET`] ahead of
/// the upload. When the budget stays full, a pump parks the iterator with the chunk it read
/// and frees its thread; the next item taken restarts it.
struct Pumped {
    rx: mpsc::UnboundedReceiver<Queued>,
    budget: Arc<Semaphore>,
    /// The pump while none runs: before the first poll, or once parked at a full budget.
    parked: Arc<Mutex<Option<Pump>>>,
}

/// What a pump task needs to read the iterator into the body.
struct Pump {
    iter: Py<PyAny>,
    tx: mpsc::UnboundedSender<Queued>,
    budget: Arc<Semaphore>,
    /// A chunk read before the pump parked, queued first when it restarts.
    pending: Option<PyBytesLike>,
}

/// Bytes an upload may queue ahead of the connection.
const UPLOAD_BUDGET: usize = 256 * 1024;

/// The least budget a queued chunk holds, so small chunks queue at most 64 items.
const UPLOAD_CHARGE: usize = 4 * 1024;

/// A request body from a Python async generator, forwarded by a task on the loop that was
/// running at extraction, up to [`UPLOAD_BUDGET`] ahead. Dropping it cancels that task.
struct PyAsyncStream {
    rx: mpsc::UnboundedReceiver<Queued>,
    budget: Arc<Semaphore>,
    /// Set by [`Sender::finish`], so the channel closing reads as the end of the body.
    finished: Arc<AtomicBool>,
    task: Option<(Py<PyAny>, Py<PyAny>)>,
}

/// A queued item and the share of the upload budget it holds until the body takes it.
struct Queued {
    item: Item,
    _share: Option<OwnedSemaphorePermit>,
}

/// The channel end given to the forwarding coroutine: `try_send` queues a chunk within the
/// budget or waits for room, and `send` also queues the error that ends the body. Closing
/// it without `finish` fails the body.
#[pyclass(frozen)]
struct Sender {
    tx: Mutex<Option<mpsc::UnboundedSender<Queued>>>,
    budget: Arc<Semaphore>,
    finished: Arc<AtomicBool>,
}

// ===== impl PyBytesLike =====

impl FromPyObject<'_, '_> for PyBytesLike {
    type Error = PyErr;

    fn extract(ob: Borrowed<PyAny>) -> PyResult<Self> {
        // Checked by type, so a str chunk does not first fail as bytes.
        if ob.is_instance_of::<PyString>() {
            ob.extract().map(PyBytesLike::String)
        } else {
            ob.extract().map(PyBytesLike::Bytes)
        }
    }
}

impl PyBytesLike {
    /// The share of the upload budget this chunk holds while queued; one larger than the
    /// budget takes all of it.
    fn share(&self) -> u32 {
        let len = match self {
            PyBytesLike::Bytes(b) => b.0.len(),
            PyBytesLike::String(s) => s.0.len(),
        };
        u32::try_from(len.clamp(UPLOAD_CHARGE, UPLOAD_BUDGET)).unwrap_or(u32::MAX)
    }
}

impl From<PyBytesLike> for Bytes {
    #[inline]
    fn from(value: PyBytesLike) -> Self {
        match value {
            PyBytesLike::Bytes(b) => b.0,
            PyBytesLike::String(s) => s.0,
        }
    }
}

// ===== impl PyStream =====

impl FromPyObject<'_, '_> for PyStream {
    type Error = PyErr;

    fn extract(ob: Borrowed<PyAny>) -> PyResult<Self> {
        // Iterators are checked first; probing `asend` would raise for each of them.
        let source = if ob.cast::<PyIterator>().is_err() && ob.hasattr(intern!(ob.py(), "asend"))? {
            Source::Async(PyAsyncStream::new(ob.to_owned())?)
        } else {
            Source::Sync(SyncStream::Idle {
                iter: ob.to_owned().unbind(),
                owner: thread::current().id(),
            })
        };
        Ok(PyStream(source))
    }
}

impl Stream for PyStream {
    type Item = Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match &mut self.get_mut().0 {
            Source::Sync(stream) => stream.poll_next(cx),
            Source::Async(stream) => Pin::new(stream).poll_next(cx),
        }
    }
}

// ===== impl SyncStream =====

impl SyncStream {
    fn poll_next(&mut self, cx: &mut Context<'_>) -> Poll<Option<Item>> {
        if let SyncStream::Idle { .. } = self
            && let SyncStream::Idle { iter, owner } = mem::replace(self, SyncStream::Done)
        {
            *self = if runtime::driving() {
                SyncStream::Inline {
                    iter,
                    owner,
                    flushed: false,
                }
            } else {
                SyncStream::Pumped(Pumped::new(iter))
            };
        }
        match self {
            SyncStream::Inline {
                iter,
                owner,
                flushed,
            } => {
                // Return to the connection before each read, so the request head and the
                // chunks already yielded go out before `__next__` can block. The deferred
                // wake lets the runtime poll its IO and the caller first.
                if !mem::replace(flushed, true) {
                    let _ = pin!(yield_now()).poll(cx);
                    return Poll::Pending;
                }
                *flushed = false;
                // Like a pump, stop reading once Python is unavailable.
                let item = Python::try_attach(|py| {
                    let item = next_item(py, iter);
                    // A KeyboardInterrupt or SystemExit reaches the caller as itself: the
                    // owner's call, or for Ctrl+C the main thread signals reach.
                    if let Some(Err(err)) = &item
                        && !err.is_instance_of::<PyException>(py)
                        && (*owner == thread::current().id()
                            || err.is_instance_of::<PyKeyboardInterrupt>(py))
                    {
                        runtime::interrupt(err.clone_ref(py));
                    }
                    item
                })
                .flatten();
                if !matches!(item, Some(Ok(_))) {
                    *self = SyncStream::Done;
                }
                Poll::Ready(item)
            }
            SyncStream::Pumped(pumped) => pumped.poll_next(cx),
            SyncStream::Idle { .. } | SyncStream::Done => Poll::Ready(None),
        }
    }
}

// ===== impl Pumped =====

impl Pumped {
    fn new(iter: Py<PyAny>) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let budget = Arc::new(Semaphore::new(UPLOAD_BUDGET));
        let pump = Pump {
            iter,
            tx,
            budget: budget.clone(),
            pending: None,
        };
        Pumped {
            rx,
            budget,
            parked: Arc::new(Mutex::new(Some(pump))),
        }
    }

    fn poll_next(&mut self, cx: &mut Context<'_>) -> Poll<Option<Item>> {
        // Taking an item returns its share of the budget before the parked check.
        let poll = self
            .rx
            .poll_recv(cx)
            .map(|queued| queued.map(|queued| queued.item));
        // A parked pump waits for budget, which the first poll or a taken item frees. The
        // check follows the take, so a pump parking concurrently is seen here.
        let parked = lock(&self.parked).take();
        if let Some(pump) = parked {
            let parked = self.parked.clone();
            spawn_blocking(move || pump.run(&parked));
        }
        poll
    }
}

impl Drop for Pumped {
    fn drop(&mut self) {
        // A pump waiting for budget stops at once, without reading another item.
        self.budget.close();
    }
}

// ===== impl Pump =====

impl Pump {
    /// How long a pump waits for budget before parking to free its thread.
    const PARK_AFTER: Duration = Duration::from_millis(10);

    /// Queue chunks until the iterator ends or raises, the body is dropped, or the budget
    /// stays full past [`PARK_AFTER`](Self::PARK_AFTER).
    fn run(mut self, parked: &Mutex<Option<Pump>>) {
        let handle = Handle::current();
        // Once Python is unavailable, stop reading without creating a PyErr that could
        // require another attachment to format.
        Python::try_attach(|py| {
            loop {
                if self.tx.is_closed() {
                    return;
                }
                let chunk = match self.pending.take() {
                    Some(chunk) => chunk,
                    None => match next_item(py, &self.iter) {
                        Some(Ok(chunk)) => chunk,
                        Some(Err(err)) => {
                            let _ = self.tx.send(Queued {
                                item: Err(err),
                                _share: None,
                            });
                            return;
                        }
                        None => return,
                    },
                };
                let share = match self.budget.clone().try_acquire_many_owned(chunk.share()) {
                    Ok(share) => share,
                    Err(TryAcquireError::Closed) => return,
                    Err(TryAcquireError::NoPermits) => {
                        let budget = self.budget.clone();
                        let wait = time::timeout(
                            Self::PARK_AFTER,
                            budget.acquire_many_owned(chunk.share()),
                        );
                        match py.detach(|| handle.block_on(wait)) {
                            Ok(Ok(share)) => share,
                            Ok(Err(_)) => return,
                            Err(_) => {
                                // Park under the lock the body takes after receiving, so
                                // either it sees the pump parked or the pump sees budget.
                                let mut slot = lock(parked);
                                match self.budget.clone().try_acquire_many_owned(chunk.share()) {
                                    Ok(share) => share,
                                    Err(TryAcquireError::Closed) => return,
                                    Err(TryAcquireError::NoPermits) => {
                                        self.pending = Some(chunk);
                                        *slot = Some(self);
                                        return;
                                    }
                                }
                            }
                        }
                    }
                };
                let queued = Queued {
                    item: Ok(chunk),
                    _share: Some(share),
                };
                if self.tx.send(queued).is_err() {
                    return;
                }
            }
        });
    }
}

#[inline]
fn lock(parked: &Mutex<Option<Pump>>) -> MutexGuard<'_, Option<Pump>> {
    parked.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Call `__next__`, ending the stream at StopIteration.
fn next_item(py: Python<'_>, iter: &Py<PyAny>) -> Option<Item> {
    match iter.call_method0(py, intern!(py, "__next__")) {
        Ok(ob) => Some(ob.extract(py)),
        Err(err) if err.is_instance_of::<PyStopIteration>(py) => None,
        Err(err) => Some(Err(err)),
    }
}

// ===== impl PyAsyncStream =====

impl PyAsyncStream {
    /// Start forwarding on the running loop, so the body must be extracted in a coroutine.
    fn new(generator: Bound<'_, PyAny>) -> PyResult<Self> {
        static FORWARD: PyOnceLock<Py<PyAny>> = PyOnceLock::new();
        let py = generator.py();
        let event_loop = coroutine::running_loop(py)?;
        let forward = FORWARD.get_or_try_init(py, || {
            PyModule::from_code(
                py,
                c"import asyncio

async def forward(gen, sender):
    try:
        try:
            async for item in gen:
                # Queued at once while the upload budget has room, otherwise awaited.
                sent = sender.try_send(item)
                if not isinstance(sent, bool):
                    sent = await sent
                if not sent:
                    return
        finally:
            close = getattr(gen, 'aclose', None)
            if close is not None:
                await close()
    except asyncio.CancelledError as error:
        # Task cancellation must not wait for space in a retained body, and must not leave
        # the body waiting while a traceback keeps this frame and its sender alive.
        if asyncio.current_task().cancelling():
            sender.close()
        else:
            await sender.send(error, True)
        raise
    except BaseException as error:
        await sender.send(error, True)
    else:
        sender.finish()
",
                c"wreq/_async_stream.py",
                c"wreq._async_stream",
            )?
            .getattr("forward")
            .map(Bound::unbind)
        })?;
        let (tx, rx) = mpsc::unbounded_channel();
        let budget = Arc::new(Semaphore::new(UPLOAD_BUDGET));
        let finished = Arc::new(AtomicBool::new(false));
        let sender = Sender {
            tx: Mutex::new(Some(tx)),
            budget: budget.clone(),
            finished: finished.clone(),
        };
        let coroutine = forward.bind(py).call1((generator, sender))?;
        // create_task captures the caller's contextvars on the running loop.
        let task = event_loop
            .call_method1(intern!(py, "create_task"), (&coroutine,))
            .inspect_err(|_| {
                let _ = coroutine.call_method0(intern!(py, "close"));
            })?;
        Ok(Self {
            rx,
            budget,
            finished,
            task: Some((task.unbind(), event_loop.unbind())),
        })
    }
}

impl Stream for PyAsyncStream {
    type Item = Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        match this.rx.poll_recv(cx) {
            // Taking the item returns its share of the budget.
            Poll::Ready(Some(queued)) => Poll::Ready(Some(queued.item)),
            // Every sender is gone: the end after `finish`. Otherwise the forwarding task was
            // cancelled or destroyed, so the body is incomplete.
            Poll::Ready(None)
                if this.task.take().is_some() && !this.finished.load(Ordering::Acquire) =>
            {
                Poll::Ready(Some(Err(PyRuntimeError::new_err(
                    "async body generator stopped before it finished",
                ))))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Drop for PyAsyncStream {
    fn drop(&mut self) {
        self.rx.close();
        // A send waiting for room resolves to `False` at once.
        self.budget.close();
        if let Some((task, event_loop)) = self.task.take() {
            // Body drop can run on Tokio: cancel from a blocking thread, preferring the current
            // runtime so a drop on a client's own runtime does not start the shared one.
            let handle = Handle::try_current().unwrap_or_else(|_| runtime::get().handle().clone());
            handle.spawn_blocking(move || {
                Python::try_attach(|py| {
                    if let Ok(cancel) = task.bind(py).getattr(intern!(py, "cancel")) {
                        let _ = event_loop.call_method1(
                            py,
                            intern!(py, "call_soon_threadsafe"),
                            (cancel,),
                        );
                    }
                });
            });
        }
    }
}

// ===== impl Sender =====

#[pymethods]
impl Sender {
    /// Queue a chunk: `True` once queued within the budget, `False` once the body is dropped
    /// or the sender closed, and otherwise a coroutine that queues it once there is room.
    fn try_send<'py>(&self, item: Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        let py = item.py();
        let chunk = item.extract::<PyBytesLike>()?;
        let tx = self.lock().clone();
        let sent = match &tx {
            None => false,
            Some(queue) => match self.budget.clone().try_acquire_many_owned(chunk.share()) {
                Ok(share) => queue
                    .send(Queued {
                        item: Ok(chunk),
                        _share: Some(share),
                    })
                    .is_ok(),
                Err(TryAcquireError::Closed) => false,
                // The coroutine keeps the extracted chunk, so it is never copied twice.
                Err(TryAcquireError::NoPermits) => {
                    return self.send_later(py, tx, Ok(chunk)).map(Bound::into_any);
                }
            },
        };
        sent.into_bound_py_any(py)
    }

    /// Queue `item` as the error that ends the body, or a chunk once the budget has room.
    /// Resolves to `False` once the body is dropped or the sender closed, which stops
    /// forwarding.
    fn send<'py>(
        &self,
        py: Python<'py>,
        item: Bound<'py, PyAny>,
        error: bool,
    ) -> PyResult<Bound<'py, Coroutine>> {
        let item = if error {
            Err(PyErr::from_value(item))
        } else {
            Ok(item.extract()?)
        };
        self.send_later(py, self.lock().clone(), item)
    }

    /// Mark the normal end of the body. The last chunk may still be queued: it is read
    /// before the closed channel, so finishing never waits for room.
    fn finish(&self) {
        self.finished.store(true, Ordering::Release);
        // Python may retain the sender after completion, especially on PyPy.
        self.close();
    }

    /// Drop the channel end at once, so the body fails instead of waiting for more.
    fn close(&self) {
        let tx = self.lock().take();
        drop(tx);
    }
}

impl Sender {
    #[inline]
    fn lock(&self) -> MutexGuard<'_, Option<mpsc::UnboundedSender<Queued>>> {
        self.tx.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// A coroutine that queues `item` on `tx` once the budget has room; an error, which ends
    /// the body, never waits.
    fn send_later<'py>(
        &self,
        py: Python<'py>,
        tx: Option<mpsc::UnboundedSender<Queued>>,
        item: Item,
    ) -> PyResult<Bound<'py, Coroutine>> {
        let budget = self.budget.clone();
        // Budget readiness is runtime-independent, so this waits on the Python loop.
        coroutine::local(py, "Sender.send", async move {
            let Some(tx) = tx else {
                return Ok(false);
            };
            let share = match &item {
                Ok(chunk) => match budget.acquire_many_owned(chunk.share()).await {
                    Ok(share) => Some(share),
                    Err(_) => return Ok(false),
                },
                Err(_) => None,
            };
            Ok(tx
                .send(Queued {
                    item,
                    _share: share,
                })
                .is_ok())
        })
    }
}
