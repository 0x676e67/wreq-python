use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures_util::{Stream, future::poll_fn};
use pyo3::{coroutine::CancelHandle, intern, prelude::*, sync::PyOnceLock};
use tokio::sync::mpsc;

use super::PyBytesLike;
use crate::client::nogil::poll_with_guard;

/// Owns the upload task; dropping the body cancels it on its Python event loop.
pub struct Upload {
    rx: mpsc::Receiver<PyResult<PyBytesLike>>,
    task: Option<(Py<PyAny>, Py<PyAny>)>,
}

#[pyclass(frozen)]
struct Sender(mpsc::Sender<PyResult<PyBytesLike>>);

// ===== impl Upload =====

impl Upload {
    pub fn new(generator: Bound<'_, PyAny>) -> PyResult<Self> {
        static FORWARD: PyOnceLock<Py<PyAny>> = PyOnceLock::new();
        let py = generator.py();
        let event_loop = py.import("asyncio")?.call_method0("get_running_loop")?;
        let forward = FORWARD.get_or_try_init(py, || {
            PyModule::from_code(
                py,
                c"import asyncio

async def forward(gen, sender):
    try:
        try:
            async for item in gen:
                if not await sender.send(item, False):
                    return
        finally:
            close = getattr(gen, 'aclose', None)
            if close is not None:
                await close()
    except BaseException as error:
        await sender.send(error, True)
        if isinstance(error, asyncio.CancelledError):
            raise
",
                c"wreq/_upload.py",
                c"wreq._upload",
            )?
            .getattr("forward")
            .map(Bound::unbind)
        })?;
        let (tx, rx) = mpsc::channel(1);
        let coroutine = forward.bind(py).call1((generator, Sender(tx)))?;
        // create_task captures the caller's contextvars on the running loop.
        let task = match event_loop.call_method1("create_task", (&coroutine,)) {
            Ok(task) => task,
            Err(err) => {
                let _ = coroutine.call_method0("close");
                return Err(err);
            }
        };
        Ok(Self {
            rx,
            task: Some((task.unbind(), event_loop.unbind())),
        })
    }
}

impl Stream for Upload {
    type Item = PyResult<PyBytesLike>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.get_mut().rx.poll_recv(cx)
    }
}

impl Drop for Upload {
    fn drop(&mut self) {
        self.rx.close();
        if let (Some((task, event_loop)), Ok(runtime)) = (self.task.take(), crate::runtime::get()) {
            // Body drop can run on Tokio: acquire the interpreter on a blocking thread.
            runtime.spawn_blocking(move || {
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
    async fn send(
        &self,
        item: Py<PyAny>,
        error: bool,
        #[pyo3(cancel_handle)] mut cancel: CancelHandle,
    ) -> PyResult<bool> {
        let item = Python::attach(|py| {
            if error {
                Ok(Err(PyErr::from_value(item.into_bound(py))))
            } else {
                item.extract(py).map(Ok)
            }
        })?;
        let item = match self.0.try_send(item) {
            Ok(()) => return Ok(true),
            Err(mpsc::error::TrySendError::Closed(_)) => return Ok(false),
            Err(mpsc::error::TrySendError::Full(item)) => item,
        };
        let tx = self.0.clone();
        // Channel readiness is runtime-independent; keep this on the Python loop.
        let mut send = std::pin::pin!(tx.send(item));
        tokio::select! {
            biased;
            exception = poll_fn(|cx| cancel.poll_cancelled(cx)) => {
                Err(Python::attach(|py| PyErr::from_value(exception.into_bound(py))))
            }
            result = poll_fn(|cx| poll_with_guard(send.as_mut(), cx)) => Ok(result.is_ok()),
        }
    }
}
