//! WebSocket Command Utilities
//!
//! This module defines the `Command` enum for representing WebSocket operations
//! (send, receive, close) and provides async helpers for sending commands to the
//! WebSocket background task. It enables safe, concurrent, and ergonomic control
//! of WebSocket communication from Python bindings.

use std::time::Duration;

use futures_util::{SinkExt, StreamExt, TryStreamExt};
use pyo3::prelude::*;
use tokio::sync::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    oneshot::{self, Sender},
};

use super::{
    Error, Message, Utf8Bytes,
    ws::{self, WebSocket},
};
use crate::extractor::StrInput;

/// Commands for WebSocket operations.
pub enum Command {
    /// Send a WebSocket message.
    ///
    /// Contains the message to send and a oneshot sender for the result.
    Send(Message, Sender<PyResult<()>>),

    /// Send multiple WebSocket messages.
    ///
    /// Contains a vector of messages to send and a oneshot sender for the result.
    SendMany(Vec<Message>, Sender<PyResult<()>>),

    /// Receive a WebSocket message.
    ///
    /// Contains an optional timeout and a oneshot sender for the result.
    Recv(Option<Duration>, Sender<PyResult<Option<Message>>>),

    /// Close the WebSocket connection.
    ///
    /// Contains an optional close code, optional reason, and a oneshot sender for the result.
    Close(Option<u16>, Option<StrInput>, Sender<PyResult<()>>),
}

/// The main background task that processes incoming [`Command`]s and interacts with the WebSocket.
///
/// Handles sending, receiving, and closing the WebSocket connection based on received commands.
pub async fn task(ws: WebSocket, mut cmd: UnboundedReceiver<Command>) {
    let (mut writer, mut reader) = ws.split();
    while let Some(command) = cmd.recv().await {
        match command {
            Command::Send(msg, tx) => {
                let res = writer
                    .send(msg.0)
                    .await
                    .map_err(Error::Library)
                    .map_err(Into::into);

                let _ = tx.send(res);
            }
            Command::SendMany(many_msg, tx) => {
                let stream = many_msg.into_iter().map(|m| Ok(m.0));
                let res = writer
                    .send_all(&mut futures_util::stream::iter(stream))
                    .await
                    .map_err(Error::Library)
                    .map_err(Into::into);

                let _ = tx.send(res);
            }
            Command::Recv(timeout, tx) => {
                let fut = async {
                    reader
                        .try_next()
                        .await
                        .map(|opt| opt.map(Message))
                        .map_err(Error::Library)
                        .map_err(Into::into)
                };

                if let Some(timeout) = timeout {
                    match tokio::time::timeout(timeout, fut).await {
                        Ok(res) => {
                            let _ = tx.send(res);
                        }
                        Err(err) => {
                            let _ = tx.send(Err(Error::Timeout(err).into()));
                        }
                    }
                } else {
                    let _ = tx.send(fut.await);
                }
            }
            Command::Close(code, reason, tx) => {
                let code = code
                    .map(ws::message::CloseCode::from)
                    .unwrap_or(ws::message::CloseCode::NORMAL);
                let reason = reason
                    .map(|reason| reason.0)
                    .map(Utf8Bytes::try_from)
                    .transpose();

                let close_frame = match reason {
                    Ok(Some(reason)) => Some(ws::message::CloseFrame { code, reason }),
                    _ => None,
                };

                let res = writer
                    .send(ws::message::Message::Close(close_frame))
                    .await
                    .map_err(Error::Library)
                    .map_err(Into::into);
                let _ = writer.close().await;
                let _ = tx.send(res);
                break;
            }
        }
    }
}

/// Sends a [`Command::Recv`] to the background task and awaits a message from the WebSocket.
///
/// Returns the received message or an error if the connection is closed or timeout.
#[inline]
pub async fn recv(
    cmd: UnboundedSender<Command>,
    timeout: Option<Duration>,
) -> PyResult<Option<Message>> {
    send_command(cmd, |tx| Command::Recv(timeout, tx)).await?
}

/// Sends a [`Command::Send`] to the background task to transmit a message over the WebSocket.
///
/// Returns Ok if the message was sent successfully, or an error otherwise.
#[inline]
pub async fn send(cmd: UnboundedSender<Command>, message: Message) -> PyResult<()> {
    send_command(cmd, |tx| Command::Send(message, tx)).await?
}

/// Send as [`Command::SendMany`] to the background task to transmit multiple messages over the
/// WebSocket.
///
/// Returns Ok if all messages were sent successfully, or an error otherwise.
#[inline]
pub async fn send_all(cmd: UnboundedSender<Command>, messages: Vec<Message>) -> PyResult<()> {
    if messages.is_empty() {
        return Ok(());
    }
    send_command(cmd, |tx| Command::SendMany(messages, tx)).await?
}

/// Sends a [`Command::Close`] to the background task to gracefully close the WebSocket connection.
///
/// Returns Ok if the connection was closed successfully, or an error otherwise.
#[inline]
pub async fn close(
    cmd: UnboundedSender<Command>,
    code: Option<u16>,
    reason: Option<StrInput>,
) -> PyResult<()> {
    send_command(cmd, |tx| Command::Close(code, reason, tx)).await?
}

async fn send_command<T>(
    cmd: UnboundedSender<Command>,
    make: impl FnOnce(oneshot::Sender<T>) -> Command,
) -> PyResult<T> {
    if cmd.is_closed() {
        return Err(Error::WebSocketDisconnected.into());
    }
    let (tx, rx) = oneshot::channel();
    cmd.send(make(tx))
        .map_err(|_| Error::WebSocketDisconnected)?;
    Ok(rx.await.map_err(|_| Error::WebSocketDisconnected)?)
}
