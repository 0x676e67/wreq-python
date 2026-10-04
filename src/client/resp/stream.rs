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

use bytes::Bytes;
use futures_util::FutureExt;
use http_body_util::BodyExt;
use pyo3::{exceptions::PyRuntimeError, prelude::*};
use tokio::sync::{
    Notify,
    mpsc::{self, error::TryRecvError},
};
use tokio_util::task::AbortOnDropHandle;

use crate::{
    aio::{self, Coroutine},
    buffer::PyBuffer,
    client::nogil,
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

/// A response stream yielding read-only memoryviews and any trailing headers.
///
/// Sync and async iteration share [`Streamer::next`]; only the wait differs.
#[pyclass(subclass, frozen, skip_from_py_object)]
pub struct Streamer {
    reader: Arc<Reader>,
    runtime: Runtime,
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

// ===== impl Streamer =====

impl Streamer {
    /// Frames buffered ahead of Python. A frame holds at most one transport read, up to
    /// 408 KiB on HTTP/1 by default, so the 8 queued frames plus the one being sent stay
    /// under about 3.6 MiB.
    const READ_AHEAD: usize = 8;

    /// Buffered frames returned before `__anext__` yields to the event loop once.
    const YIELD_EVERY: usize = 8;

    /// Create a new [`Streamer`] instance.
    #[inline]
    pub fn new(resp: wreq::Response, runtime: Runtime) -> Streamer {
        let reader = Reader {
            state: Mutex::new(State::Idle(Box::new(resp))),
            arrived: Arc::new(Notify::new()),
            since_yield: AtomicUsize::new(0),
        };
        Streamer {
            reader: Arc::new(reader),
            runtime,
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
            let _ = burst(tx.send(panicked), &end.0).await;
        }
        // Readers wake only after the sender drops, so they see the end.
        drop(tx);
    }

    /// Send frames until the body ends, a frame fails or the reader is gone.
    async fn pump(mut resp: wreq::Response, tx: &mpsc::Sender<PyResult<Frame>>, arrived: &Notify) {
        while let Some(frame) = burst(resp.frame(), arrived).await {
            let Some(frame) = Self::convert(frame, &mut resp) else {
                continue;
            };
            let failed = frame.is_err();
            if burst(tx.send(frame), arrived).await.is_err() || failed {
                break;
            }
        }
    }

    /// Convert a body frame, skipping unknown kinds.
    fn convert(
        frame: wreq::Result<http_body::Frame<Bytes>>,
        resp: &mut wreq::Response,
    ) -> Option<PyResult<Frame>> {
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

    /// Read a frame the body already holds, before any read-ahead task starts, so a
    /// synchronous reader of a short body never waits on another thread.
    fn ready_frame(&self) -> Option<PyResult<Frame>> {
        let mut state = self.reader.lock();
        let State::Idle(resp) = &mut *state else {
            return None;
        };
        let _runtime = self.runtime.handle().enter();
        let Some(frame) = Self::poll_ready(resp)? else {
            *state = State::Closed;
            return Some(Err(Error::StopIteration.into()));
        };
        if frame.is_err() {
            *state = State::Closed;
            return Some(frame);
        }
        // Confirm the end before returning: an unpolled body's read timeout keeps running
        // while the caller works on this frame. Remaining frames go to the read-ahead task,
        // so a stalled caller does not stall the network read.
        match Self::poll_ready(resp) {
            Some(None) => *state = State::Closed,
            next => {
                if let State::Idle(resp) = mem::replace(&mut *state, State::Closed) {
                    *state = self.read_ahead_from(*resp, next.flatten());
                }
            }
        }
        Some(frame)
    }

    /// Poll the body once: `None` if it must wait, `Some(None)` at its end.
    fn poll_ready(resp: &mut wreq::Response) -> Option<Option<PyResult<Frame>>> {
        loop {
            let Some(frame) = resp.frame().now_or_never()? else {
                return Some(None);
            };
            if let Some(frame) = Self::convert(frame, resp) {
                return Some(Some(frame));
            }
        }
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
        let task =
            self.runtime
                .handle()
                .spawn(Self::read_ahead(resp, tx, self.reader.arrived.clone()));
        State::Reading {
            rx,
            _task: Some(AbortOnDropHandle::new(task)),
        }
    }

    /// Start the read-ahead task if needed, then return a buffered frame, `end()` once the
    /// body is done or closed, or `None` if the read must wait for `arrived`.
    fn try_next(&self, end: fn() -> Error) -> Option<PyResult<Frame>> {
        let mut state = self.reader.lock();
        if let State::Idle(_) = *state
            && let State::Idle(resp) = mem::replace(&mut *state, State::Closed)
        {
            *state = self.read_ahead_from(*resp, None);
        }
        let State::Reading { rx, .. } = &mut *state else {
            return Some(Err(end().into()));
        };
        match rx.try_recv() {
            Ok(frame) => Some(frame),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                *state = State::Closed;
                Some(Err(end().into()))
            }
        }
    }

    /// Wait for the next frame; `end` builds the error that ends iteration.
    async fn next(&self, end: fn() -> Error) -> PyResult<Frame> {
        loop {
            // Readers are woken by `notify_waiters`, which reaches a `Notified` from its
            // creation, so it needs no `enable` and a frame sent after `try_next` still wakes it.
            let arrived = pin!(self.reader.arrived.notified());
            if let Some(frame) = self.try_next(end) {
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
        // Frames already received are returned without releasing the GIL.
        if let Some(frame) = self.ready_frame() {
            return frame;
        }
        nogil::run(py, &self.runtime, self.next(|| Error::StopIteration))
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
    fn __aiter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    /// Read the next frame when awaited.
    fn __anext__(slf: Bound<'_, Self>) -> PyResult<Bound<'_, Coroutine>> {
        let py = slf.py();
        let slf = slf.unbind();
        aio::local(py, "Streamer.__anext__", async move {
            let this = slf.get();
            // Buffered frames complete without suspending; yield to the event loop
            // periodically so timeouts, cancellation and other tasks run.
            if this.reader.since_yield.fetch_add(1, Ordering::Relaxed) >= Self::YIELD_EVERY {
                this.reader.since_yield.store(0, Ordering::Relaxed);
                aio::yield_now().await;
            }
            this.next(|| Error::StopAsyncIteration).await
        })
    }

    fn __aenter__(slf: Bound<'_, Self>) -> PyResult<Bound<'_, Coroutine>> {
        aio::ready("Streamer.__aenter__", slf)
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
        aio::local(py, "Streamer.__aexit__", async move {
            reader.close();
            Ok(())
        })
    }
}

/// Await `fut`, notifying readers whenever it suspends, so a burst of ready
/// frames costs one wake while a slow stream still delivers each frame at once.
async fn burst<F: Future>(fut: F, arrived: &Notify) -> F::Output {
    let mut fut = pin!(fut);
    poll_fn(|cx| {
        let poll = fut.as_mut().poll(cx);
        if poll.is_pending() {
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
