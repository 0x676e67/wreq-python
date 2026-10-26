//! Short response bodies read along with the head.

use std::{
    pin::Pin,
    task::{Context, Poll},
};

use bytes::{Bytes, BytesMut};
use futures_util::FutureExt;
use http_body::{Frame, SizeHint};
use http_body_util::BodyExt;

/// Bodies of known length up to this size are read with the head when already received.
const PREFETCH_LIMIT: u64 = 64 * 1024;

/// A completed body, or frames still to deliver, safe to poll off-runtime only if held.
pub(super) enum Prefetch {
    Complete(Bytes),
    Remaining { body: wreq::Body, held: bool },
}

/// A body whose leading frames were read with the head, replayed before the rest.
struct Prefetched {
    data: Option<Bytes>,
    /// The frame that ended prefetching: trailers or an error.
    next: Option<wreq::Result<Frame<Bytes>>>,
    /// The unread rest, `None` once the body failed.
    rest: Option<wreq::Body>,
}

/// Data frames read so far; the usual single frame is kept without copying.
#[derive(Default)]
enum Data {
    #[default]
    Empty,
    One(Bytes),
    Many(BytesMut),
}

/// Read the frames of a body of known length up to [`PREFETCH_LIMIT`] that already arrived,
/// so readers on the event loop need not wait on the runtime.
///
/// Completed bodies keep their bytes directly. Other bodies report whether all remaining
/// frames are held; an HTTP/2 body still tied to its connection stays on the runtime.
/// It never waits for the network: frames still in flight stay with the connection.
pub(super) fn prefetch(mut body: wreq::Body, len: Option<u64>) -> Prefetch {
    let Some(len) = len.filter(|len| *len <= PREFETCH_LIMIT) else {
        return Prefetch::Remaining { body, held: false };
    };
    let mut data = Data::default();
    let next = loop {
        match body.frame().now_or_never() {
            Some(Some(Ok(frame))) => match frame.into_data() {
                Ok(bytes) => {
                    data.push(bytes, len);
                }
                Err(frame) => break Some(Ok(frame)),
            },
            Some(Some(Err(err))) => break Some(Err(err)),
            Some(None) => {
                return Prefetch::Complete(data.into_bytes().unwrap_or_default());
            }
            None => break None,
        }
    };
    let data = data.into_bytes();
    if data.is_none() && next.is_none() {
        return Prefetch::Remaining { body, held: false };
    }
    // A failed body holds nothing more; the error replays after the data.
    let rest = (!matches!(next, Some(Err(_)))).then_some(body);
    let held = rest.is_none();
    Prefetch::Remaining {
        body: wreq::Body::wrap(Prefetched { data, next, rest }),
        held,
    }
}

// ===== impl Data =====

impl Data {
    fn push(&mut self, bytes: Bytes, len: u64) {
        // An empty frame, such as HTTP/2's trailing END_STREAM, adds nothing worth a copy.
        if bytes.is_empty() {
            return;
        }
        match self {
            Data::Empty => *self = Data::One(bytes),
            Data::One(first) => {
                let total = first.len() + bytes.len();
                let len = usize::try_from(len).unwrap_or(total);
                let mut buf = BytesMut::with_capacity(total.max(len));
                buf.extend_from_slice(first);
                buf.extend_from_slice(&bytes);
                *self = Data::Many(buf);
            }
            Data::Many(buf) => buf.extend_from_slice(&bytes),
        }
    }

    fn into_bytes(self) -> Option<Bytes> {
        match self {
            Data::Empty => None,
            Data::One(bytes) => Some(bytes),
            Data::Many(buf) => Some(buf.freeze()),
        }
    }
}

// ===== impl Prefetched =====

impl http_body::Body for Prefetched {
    type Data = Bytes;
    type Error = wreq::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, wreq::Error>>> {
        let this = self.get_mut();
        if let Some(data) = this.data.take() {
            return Poll::Ready(Some(Ok(Frame::data(data))));
        }
        if let Some(next) = this.next.take() {
            return Poll::Ready(Some(next));
        }
        match &mut this.rest {
            Some(rest) => Pin::new(rest).poll_frame(cx),
            None => Poll::Ready(None),
        }
    }

    fn size_hint(&self) -> SizeHint {
        let mut hint = self
            .rest
            .as_ref()
            .map_or_else(|| SizeHint::with_exact(0), wreq::Body::size_hint);
        let held = self.data.as_ref().map_or(0, |data| data.len() as u64);
        // Raise the upper bound first: `set_lower` panics above it.
        if let Some(upper) = hint.upper() {
            hint.set_upper(upper.saturating_add(held));
        }
        hint.set_lower(hint.lower().saturating_add(held));
        hint
    }

    fn is_end_stream(&self) -> bool {
        self.data.is_none()
            && self.next.is_none()
            && self.rest.as_ref().is_none_or(wreq::Body::is_end_stream)
    }
}
