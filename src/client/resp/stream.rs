//! The response body [`Streamer`], read ahead by a Tokio task for sync and async iteration.

use std::{
    future::{Future, poll_fn},
    mem,
    panic::AssertUnwindSafe,
    pin::pin,
    sync::{
        Arc, Mutex, MutexGuard, PoisonError,
        atomic::{AtomicUsize, Ordering},
    },
};

use bytes::{Bytes, BytesMut};
use futures_util::FutureExt;
use http_body::Body as _;
use http_body_util::BodyExt;
use pyo3::{exceptions::PyRuntimeError, prelude::*};
use tokio::sync::{
    Notify,
    mpsc::{self, error::TryRecvError},
};
use tokio_util::task::AbortOnDropHandle;

use crate::{
    buffer::PyBuffer,
    coroutine::{self, Coroutine},
    error::Error,
    header::HeaderMap,
    runtime::Runtime,
};

/// A response frame exposed as a read-only memoryview or a header map.
#[derive(IntoPyObject)]
pub enum Frame {
    Bytes(PyBuffer),
    Trailers(HeaderMap),
}

/// A body frame as read from the response.
type RawFrame = wreq::Result<http_body::Frame<Bytes>>;

/// A response stream yielding read-only memoryviews and any trailing headers.
///
/// Sync and async iteration share [`Streamer::next`]; [`Iteration`] picks the ready frames
/// the caller reads and the error that ends iteration.
#[pyclass(subclass, frozen)]
pub struct Streamer {
    reader: Arc<Reader>,
    runtime: Runtime,
    /// Whether the event loop may poll the body: an HTTP/1 body, or one held in full.
    loop_polls: bool,
}

/// Frames buffered between the read-ahead task and Python readers, which poll the
/// buffer and wait on `arrived`; the task signals it once per burst of frames.
struct Reader {
    state: Mutex<State>,
    arrived: Arc<Notify>,
    /// Buffered frames returned since `__anext__` last yielded to the event loop.
    since_yield: AtomicUsize,
}

/// Who holds the response body.
enum State {
    /// Unread, or read only by [`Streamer::ready_frame`] on the caller.
    Idle(Box<wreq::Response>),
    /// A successfully prefetched body, taken once without polling a transport body.
    Buffered(Option<Bytes>),
    /// Held by the read-ahead task, whose frames arrive on `rx`; dropping `_task` aborts it.
    /// Without a task, `rx` holds only a final error.
    Reading {
        rx: mpsc::Receiver<PyResult<Frame>>,
        _task: Option<AbortOnDropHandle<()>>,
    },
    /// Ended, failed or closed; reads return the end-of-iteration error.
    Closed,
}

/// Wakes readers however the read-ahead task ends, including when aborted.
struct NotifyOnDrop(Arc<Notify>);

/// How Python iterates a [`Streamer`], which sets the frames read on the caller and the error
/// that ends iteration.
#[derive(Clone, Copy)]
enum Iteration {
    /// `__next__` on a blocking caller.
    Sync,
    /// `__anext__` on the event loop.
    Async,
}

// ===== impl Streamer =====

impl Streamer {
    /// Frames buffered ahead of Python. A frame holds one transport read, up to 408 KiB on
    /// HTTP/1 by default, or data frames merged up to [`Self::MERGE_LIMIT`], so the 8 queued
    /// frames plus the one being sent stay under about 3.6 MiB.
    const READ_AHEAD: usize = 8;

    /// Data frames that already arrived are merged up to this size, so a body sent in small
    /// frames, such as 16 KiB HTTP/2 DATA frames, reaches Python in fewer chunks without
    /// waiting for more data.
    const MERGE_LIMIT: usize = 256 * 1024;

    /// Only frames smaller than this are merged; larger ones are sent without a copy.
    const MERGE_FRAME: usize = 64 * 1024;

    /// Buffered frames returned before `__anext__` yields to the event loop once.
    const YIELD_EVERY: usize = 8;

    /// Create a new [`Streamer`] instance.
    #[inline]
    pub fn new(resp: wreq::Response, runtime: Runtime, loop_polls: bool) -> Streamer {
        Self::with_state(State::Idle(Box::new(resp)), runtime, loop_polls)
    }

    /// A body already read to EOF, with no trailers or errors left to deliver.
    pub fn buffered(bytes: Bytes, runtime: Runtime) -> Streamer {
        Self::with_state(
            State::Buffered((!bytes.is_empty()).then_some(bytes)),
            runtime,
            true,
        )
    }

    fn with_state(state: State, runtime: Runtime, loop_polls: bool) -> Streamer {
        let reader = Reader {
            state: Mutex::new(state),
            arrived: Arc::new(Notify::new()),
            since_yield: AtomicUsize::new(0),
        };
        Streamer {
            reader: Arc::new(reader),
            runtime,
            loop_polls,
        }
    }

    /// The read-ahead task: run [`pump`](Self::pump), report a panic as an error frame,
    /// and wake readers however it ends.
    async fn read_ahead(
        resp: wreq::Response,
        tx: mpsc::Sender<PyResult<Frame>>,
        arrived: Arc<Notify>,
    ) {
        let end = NotifyOnDrop(arrived);
        // Without this a panic would drop `tx` and read as the end of the body.
        if AssertUnwindSafe(Self::pump(resp, &tx, &end.0))
            .catch_unwind()
            .await
            .is_err()
        {
            let panicked = Err(PyRuntimeError::new_err("response body reader panicked"));
            let _ = burst(tx.send(panicked), &end.0, &mut true).await;
        }
        // Readers wake only after the sender drops, so they see the end.
        drop(tx);
    }

    /// Send frames until the body ends, a frame fails or the reader is gone.
    async fn pump(mut resp: wreq::Response, tx: &mpsc::Sender<PyResult<Frame>>, arrived: &Notify) {
        let mut unannounced = false;
        // A frame read past merged data, sent next; `Some(None)` is the end of the body.
        let mut pending = None;
        loop {
            let frame = match pending.take() {
                Some(frame) => frame,
                None => burst(resp.frame(), arrived, &mut unannounced).await,
            };
            let Some(frame) = frame else {
                break;
            };
            let (frame, next) = Self::merge(frame, &mut resp);
            pending = next;
            let Some(frame) = Self::convert(frame, &mut resp) else {
                continue;
            };
            let failed = frame.is_err();
            if burst(tx.send(frame), arrived, &mut unannounced)
                .await
                .is_err()
                || failed
            {
                break;
            }
            unannounced = true;
        }
    }

    /// Merge the data frames `resp` already holds into a data `frame`, up to
    /// [`Self::MERGE_LIMIT`]. Returns the merged frame and the frame read past it, if any.
    fn merge(frame: RawFrame, resp: &mut wreq::Response) -> (RawFrame, Option<Option<RawFrame>>) {
        let first = match frame.map(http_body::Frame::into_data) {
            Ok(Ok(data)) if data.len() < Self::MERGE_FRAME => data,
            Ok(Ok(data)) => return (Ok(http_body::Frame::data(data)), None),
            Ok(Err(frame)) => return (Ok(frame), None),
            Err(err) => return (Err(err), None),
        };
        // Frames are gathered first and copied once into a buffer of their exact size, so a
        // chunk holds no spare capacity; a single frame is sent without a copy.
        let mut rest = Vec::new();
        let mut len = first.len();
        let next = loop {
            if len >= Self::MERGE_LIMIT {
                break None;
            }
            let more = match resp.frame().now_or_never() {
                Some(Some(Ok(frame))) => match frame.into_data() {
                    Ok(more) => more,
                    Err(frame) => break Some(Some(Ok(frame))),
                },
                Some(other) => break Some(other),
                None => break None,
            };
            // A large frame, or one that would pass the limit, starts the next chunk, so no
            // chunk grows beyond the limit or a single frame.
            if more.len() >= Self::MERGE_FRAME || len + more.len() > Self::MERGE_LIMIT {
                break Some(Some(Ok(http_body::Frame::data(more))));
            }
            if !more.is_empty() {
                len += more.len();
                rest.push(more);
            }
        };
        let data = if rest.is_empty() {
            first
        } else {
            let mut buf = BytesMut::with_capacity(len);
            buf.extend_from_slice(&first);
            rest.iter().for_each(|frame| buf.extend_from_slice(frame));
            buf.freeze()
        };
        (Ok(http_body::Frame::data(data)), next)
    }

    /// Read the next frame, skipping unknown kinds; `None` at the end of the body.
    async fn next_frame(resp: &mut wreq::Response) -> Option<PyResult<Frame>> {
        loop {
            let frame = resp.frame().await?;
            if let Some(frame) = Self::convert(frame, resp) {
                return Some(frame);
            }
        }
    }

    /// Convert a body frame, skipping unknown kinds.
    fn convert(frame: RawFrame, resp: &mut wreq::Response) -> Option<PyResult<Frame>> {
        match frame.map(|frame| frame.into_data()) {
            Ok(Ok(bytes)) => Some(Ok(Frame::Bytes(PyBuffer::from(bytes)))),
            Ok(Err(frame)) => frame
                .into_trailers()
                .ok()
                .map(|trailers| Ok(Frame::Trailers(HeaderMap(trailers)))),
            Err(err) => {
                // Conservatively keep the connection out of the pool after a body error.
                resp.forbid_recycle();
                Some(Err(Error::Library(err).into()))
            }
        }
    }

    /// Read a frame the body already holds, before any read-ahead task starts, so a reader
    /// of a short body never waits on another thread.
    fn ready_frame(&self, iteration: Iteration) -> Option<PyResult<Frame>> {
        let mut state = self.reader.lock();
        if let State::Buffered(bytes) = &mut *state {
            return Some(
                bytes
                    .take()
                    .map(|bytes| Frame::Bytes(PyBuffer::from(bytes)))
                    .ok_or_else(|| iteration.end().into()),
            );
        }
        let State::Idle(resp) = &mut *state else {
            return None;
        };
        if !iteration.takes_ready(resp, self.loop_polls) {
            return None;
        }
        let Some(frame) = self.runtime.poll_now(Self::next_frame(resp))? else {
            *state = State::Closed;
            return Some(Err(iteration.end().into()));
        };
        if frame.is_err() {
            *state = State::Closed;
            return Some(frame);
        }
        // Confirm the end before returning: an unpolled body's read timeout keeps running
        // while the caller works on this frame. Remaining frames go to the read-ahead task,
        // so a stalled caller does not stall the network read.
        match self.runtime.poll_now(Self::next_frame(resp)) {
            Some(None) => *state = State::Closed,
            next => {
                if let State::Idle(resp) = mem::replace(&mut *state, State::Closed) {
                    *state = self.read_ahead_from(*resp, next.flatten());
                }
            }
        }
        Some(frame)
    }

    /// Start the read-ahead task after `first`, a frame already read from `resp`.
    fn read_ahead_from(&self, resp: wreq::Response, first: Option<PyResult<Frame>>) -> State {
        let (tx, rx) = mpsc::channel(Self::READ_AHEAD);
        if let Some(first) = first {
            let failed = first.is_err();
            // The channel is new, so it has room.
            let _ = tx.try_send(first);
            if failed {
                return State::Reading { rx, _task: None };
            }
        }
        let task = self
            .runtime
            .spawn(Self::read_ahead(resp, tx, self.reader.arrived.clone()));
        State::Reading {
            rx,
            _task: Some(AbortOnDropHandle::new(task)),
        }
    }

    /// Start the read-ahead task if needed, then return a buffered frame, the end of
    /// `iteration` once the body is done or closed, or `None` if the read must wait for
    /// `arrived`.
    fn try_next(&self, iteration: Iteration) -> Option<PyResult<Frame>> {
        let mut state = self.reader.lock();
        if let State::Idle(_) = *state
            && let State::Idle(resp) = mem::replace(&mut *state, State::Closed)
        {
            *state = self.read_ahead_from(*resp, None);
        }
        let State::Reading { rx, .. } = &mut *state else {
            return Some(Err(iteration.end().into()));
        };
        match rx.try_recv() {
            Ok(frame) => Some(frame),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                *state = State::Closed;
                Some(Err(iteration.end().into()))
            }
        }
    }

    /// Refuse async use on a current-thread runtime: nothing would drive the read while a
    /// coroutine waits, so iteration would hang once the buffered frames run out.
    fn refuse_async(&self) -> PyResult<()> {
        if self.runtime.is_current_thread() {
            return Err(PyRuntimeError::new_err(
                "Streams on a CURRENT_THREAD runtime only support blocking iteration",
            ));
        }
        Ok(())
    }

    /// Return a frame the body already holds, else wait for the next one.
    async fn next(&self, iteration: Iteration) -> PyResult<Frame> {
        if let Some(frame) = self.ready_frame(iteration) {
            return frame;
        }
        loop {
            // Readers are woken by `notify_waiters`, which reaches a `Notified` from its
            // creation, so it needs no `enable` and a frame sent after `try_next` still wakes it.
            let arrived = pin!(self.reader.arrived.notified());
            if let Some(frame) = self.try_next(iteration) {
                return frame;
            }
            self.reader.since_yield.store(0, Ordering::Relaxed);
            arrived.await;
        }
    }
}

#[pymethods]
impl Streamer {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    fn __next__(&self, py: Python) -> PyResult<Frame> {
        // Refused before the first poll, which returns frames already received without
        // releasing the GIL, so a nested read never leaves the stream half consumed.
        self.runtime.block_on_eager(py, self.next(Iteration::Sync))
    }

    fn __enter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    /// Release the body and end any pending read; returned views stay valid.
    fn __exit__<'py>(
        &self,
        _exc_type: &Bound<'py, PyAny>,
        _exc_value: &Bound<'py, PyAny>,
        _traceback: &Bound<'py, PyAny>,
    ) {
        self.reader.close();
    }
}

#[pymethods]
impl Streamer {
    fn __aiter__(slf: PyRef<Self>) -> PyResult<PyRef<Self>> {
        slf.refuse_async()?;
        Ok(slf)
    }

    /// Read the next frame when awaited.
    fn __anext__(slf: Bound<'_, Self>) -> PyResult<Bound<'_, Coroutine>> {
        slf.get().refuse_async()?;
        let py = slf.py();
        let slf = slf.unbind();
        coroutine::local(py, "Streamer.__anext__", async move {
            let this = slf.get();
            // Buffered frames complete without suspending; yield to the event loop
            // periodically so timeouts, cancellation and other tasks run.
            if this.reader.since_yield.fetch_add(1, Ordering::Relaxed) >= Self::YIELD_EVERY {
                this.reader.since_yield.store(0, Ordering::Relaxed);
                coroutine::yield_now().await;
            }
            this.next(Iteration::Async).await
        })
    }

    fn __aenter__(slf: Bound<'_, Self>) -> PyResult<Bound<'_, Coroutine>> {
        slf.get().refuse_async()?;
        coroutine::ready("Streamer.__aenter__", slf)
    }

    /// Release the body and end any pending read; returned views stay valid.
    fn __aexit__<'py>(
        &self,
        py: Python<'py>,
        _exc_type: Py<PyAny>,
        _exc_val: Py<PyAny>,
        _traceback: Py<PyAny>,
    ) -> PyResult<Bound<'py, Coroutine>> {
        let reader = self.reader.clone();
        coroutine::local(py, "Streamer.__aexit__", async move {
            reader.close();
            Ok(())
        })
    }
}

/// Await `fut`, waking readers when it suspends with frames sent since the last wake, so
/// a burst of ready frames costs one wake while a slow stream still delivers each at once.
/// Waking with nothing sent would make a reader yield to the loop for no frame.
async fn burst<F: Future>(fut: F, arrived: &Notify, unannounced: &mut bool) -> F::Output {
    let mut fut = pin!(fut);
    poll_fn(|cx| {
        let poll = fut.as_mut().poll(cx);
        if poll.is_pending() && mem::take(unannounced) {
            arrived.notify_waiters();
        }
        poll
    })
    .await
}

// ===== impl Reader =====

impl Reader {
    #[inline]
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Release the body and end any waiting read.
    fn close(&self) {
        let state = mem::replace(&mut *self.lock(), State::Closed);
        drop(state);
        self.arrived.notify_waiters();
    }
}

// ===== impl NotifyOnDrop =====

impl Drop for NotifyOnDrop {
    fn drop(&mut self) {
        self.0.notify_waiters();
    }
}

// ===== impl Iteration =====

impl Iteration {
    /// Whether the caller takes a frame `resp` already holds. On the event loop only a body the
    /// loop may poll and of known length is read, whatever its size: taking a frame it holds
    /// is cheap, while a decompressing body is not.
    fn takes_ready(self, resp: &wreq::Response, loop_polls: bool) -> bool {
        match self {
            Iteration::Sync => true,
            Iteration::Async => loop_polls && resp.size_hint().exact().is_some(),
        }
    }

    /// The error that ends iteration.
    fn end(self) -> Error {
        match self {
            Iteration::Sync => Error::StopIteration,
            Iteration::Async => Error::StopAsyncIteration,
        }
    }
}
