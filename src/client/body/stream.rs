//! Request bodies streamed from Python iterators and async generators.
//!
//! `iter` reads a Python iterator, inline on a current-thread runtime or ahead of the upload
//! through `pump`, and `async_gen` forwards an async generator from its event loop. Both
//! queue chunks through `queue`, which bounds the bytes read ahead of the connection.

mod async_gen;
mod iter;
mod pump;
mod queue;

use std::{
    pin::Pin,
    sync::{Mutex, MutexGuard, PoisonError},
    task::{Context, Poll},
};

use bytes::Bytes;
use futures_util::{Stream, StreamExt, future::Either};
use pyo3::{
    exceptions::PyStopIteration,
    intern,
    prelude::*,
    types::{PyIterator, PyString},
};

use self::{async_gen::AsyncStream, iter::SyncStream};
use crate::extractor::{Binary, Text};

type Item = PyResult<PyBytesLike>;

/// A request body chunk: `bytes` or `bytearray` data, or a `str` sent as UTF-8.
pub enum PyBytesLike {
    Bytes(Binary),
    String(Text),
}

/// A request body read from a Python iterator or async generator, for `body=` and
/// multipart parts.
pub struct PyStream(Either<SyncStream, AsyncStream>);

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
    /// The chunk's length in bytes.
    fn len(&self) -> usize {
        match self {
            PyBytesLike::Bytes(b) => b.0.len(),
            PyBytesLike::String(s) => s.0.len(),
        }
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
            Either::Right(AsyncStream::new(ob.to_owned())?)
        } else {
            Either::Left(SyncStream::new(ob.to_owned().unbind()))
        };
        Ok(PyStream(source))
    }
}

impl Stream for PyStream {
    type Item = Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.get_mut().0.poll_next_unpin(cx)
    }
}

/// Call `__next__`, ending the stream at StopIteration.
fn next_item(py: Python<'_>, iter: &Py<PyAny>) -> Option<Item> {
    match iter.call_method0(py, intern!(py, "__next__")) {
        Ok(ob) => Some(ob.extract(py)),
        Err(err) if err.is_instance_of::<PyStopIteration>(py) => None,
        Err(err) => Some(Err(err)),
    }
}

/// Lock `mutex`, recovering it from a panicked holder.
#[inline]
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
