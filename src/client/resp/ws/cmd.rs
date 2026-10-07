//! The background tasks behind a [`WebSocket`](super::WebSocket) and the requests Python
//! sends them.
//!
//! Reads and writes run on separate tasks, so a pending receive never holds up a send or
//! close. A receive whose caller is gone stops waiting, and a message read just as its
//! caller left is kept for the next receive. Closing also ends the read task.

use std::time::Duration;

use futures_util::{
    SinkExt, StreamExt, TryStreamExt,
    stream::{self, SplitSink, SplitStream},
};
use pyo3::prelude::*;
use tokio::{
    sync::{
        mpsc::{self, UnboundedReceiver, UnboundedSender},
        oneshot::{self, Sender},
    },
    time,
};
use tokio_util::sync::CancellationToken;
use wreq::ws::{
    WebSocket,
    message::{self, CloseCode, CloseFrame, Utf8Bytes},
};

use super::Message;
use crate::{error::Error, extractor::Text, runtime::Runtime};

/// The request channels of a WebSocket's read and write tasks.
#[derive(Clone)]
pub struct Handle {
    reads: UnboundedSender<Read>,
    writes: UnboundedSender<Write>,
}

/// A receive with an optional timeout.
struct Read(Option<Duration>, Sender<PyResult<Option<Message>>>);

/// A write to the WebSocket.
enum Write {
    Send(Message, Sender<PyResult<()>>),
    SendMany(Vec<Message>, Sender<PyResult<()>>),
    Close(Option<u16>, Option<Text>, Sender<PyResult<()>>),
}

/// Start the read and write tasks of `ws` on `runtime`'s selected worker.
pub fn spawn(runtime: &Runtime, ws: WebSocket) -> Handle {
    let (writer, reader) = ws.split();
    let (reads, read_rx) = mpsc::unbounded_channel();
    let (writes, write_rx) = mpsc::unbounded_channel();
    let closed = CancellationToken::new();
    runtime.spawn(read(reader, read_rx, closed.clone()));
    runtime.spawn(write(writer, write_rx, closed));
    Handle { reads, writes }
}

/// Serve receives in order until the WebSocket is closed or dropped.
async fn read(
    mut reader: SplitStream<WebSocket>,
    mut reads: UnboundedReceiver<Read>,
    closed: CancellationToken,
) {
    let mut unclaimed = None;
    loop {
        let Read(timeout, mut tx) = tokio::select! {
            biased;
            _ = closed.cancelled() => return,
            read = reads.recv() => match read {
                Some(read) => read,
                None => return,
            },
        };
        if let Some(res) = unclaimed.take() {
            if let Err(res) = tx.send(res) {
                unclaimed = Some(res);
            }
            continue;
        }
        let next = async {
            match timeout {
                Some(timeout) => time::timeout(timeout, reader.try_next())
                    .await
                    .map_err(Error::Timeout),
                None => Ok(reader.try_next().await),
            }
        };
        // Reading the stream is cancel-safe: leaving for a gone caller loses nothing.
        let res = tokio::select! {
            biased;
            _ = closed.cancelled() => return,
            _ = tx.closed() => continue,
            next = next => match next {
                Ok(next) => next.map(|msg| msg.map(Message)).map_err(Error::Library),
                Err(timeout) => {
                    let _ = tx.send(Err(timeout.into()));
                    continue;
                }
            },
        };
        // The caller may have left after the read finished; keep it for the next receive.
        if let Err(res) = tx.send(res.map_err(Into::into)) {
            unclaimed = Some(res);
        }
    }
}

/// Serve writes in order until a close or the WebSocket is dropped, then end the read task.
async fn write(
    mut writer: SplitSink<WebSocket, message::Message>,
    mut writes: UnboundedReceiver<Write>,
    closed: CancellationToken,
) {
    let _closed = closed.drop_guard();
    while let Some(write) = writes.recv().await {
        match write {
            // A caller gone before its write starts does not send it.
            Write::Send(_, tx) | Write::SendMany(_, tx) if tx.is_closed() => {}
            Write::Send(msg, tx) => {
                let res = writer.send(msg.0).await.map_err(Error::Library);
                let _ = tx.send(res.map_err(Into::into));
            }
            Write::SendMany(messages, tx) => {
                let mut messages = stream::iter(messages.into_iter().map(|msg| Ok(msg.0)));
                let res = writer.send_all(&mut messages).await.map_err(Error::Library);
                let _ = tx.send(res.map_err(Into::into));
            }
            Write::Close(code, reason, tx) => {
                let reason = reason
                    .map(|reason| reason.0)
                    .map(Utf8Bytes::try_from)
                    .transpose();

                // A reason requires a code (RFC 6455 §5.5.1), so a lone reason closes normally.
                let close_frame = match reason {
                    Ok(reason) if code.is_some() || reason.is_some() => Some(CloseFrame {
                        code: code.map(CloseCode::from).unwrap_or(CloseCode::NORMAL),
                        reason: reason.unwrap_or_default(),
                    }),
                    _ => None,
                };

                let res = writer
                    .send(message::Message::Close(close_frame))
                    .await
                    .map_err(Error::Library);
                let _ = writer.close().await;
                let _ = tx.send(res.map_err(Into::into));
                return;
            }
        }
    }
}

/// Receive the next message, or `None` once the peer has closed.
#[inline]
pub async fn recv(handle: Handle, timeout: Option<Duration>) -> PyResult<Option<Message>> {
    request(&handle.reads, |tx| Read(timeout, tx))
        .await
        .ok_or(Error::WebSocketDisconnected)?
}

/// Send a message.
#[inline]
pub async fn send(handle: Handle, message: Message) -> PyResult<()> {
    request(&handle.writes, |tx| Write::Send(message, tx))
        .await
        .ok_or(Error::WebSocketDisconnected)?
}

/// Send messages in order.
#[inline]
pub async fn send_all(handle: Handle, messages: Vec<Message>) -> PyResult<()> {
    if messages.is_empty() {
        return Ok(());
    }
    request(&handle.writes, |tx| Write::SendMany(messages, tx))
        .await
        .ok_or(Error::WebSocketDisconnected)?
}

/// Send a close frame and close the connection.
#[inline]
pub async fn close(handle: Handle, code: Option<u16>, reason: Option<Text>) -> PyResult<()> {
    request(&handle.writes, |tx| Write::Close(code, reason, tx))
        .await
        .ok_or(Error::WebSocketDisconnected)?
}

/// Close like [`close`], treating an already closed connection as done, as a context
/// manager exit does.
#[inline]
pub async fn close_on_exit(handle: Handle) -> PyResult<()> {
    request(&handle.writes, |tx| Write::Close(None, None, tx))
        .await
        .unwrap_or(Ok(()))
}

/// Run a request on a task, or return `None` once the task has ended.
async fn request<R, T>(
    requests: &UnboundedSender<R>,
    make: impl FnOnce(oneshot::Sender<T>) -> R,
) -> Option<T> {
    if requests.is_closed() {
        return None;
    }
    let (tx, rx) = oneshot::channel();
    requests.send(make(tx)).ok()?;
    rx.await.ok()
}
