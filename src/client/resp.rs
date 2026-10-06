mod ext;
mod http;
mod stream;
mod ws;

pub use self::{
    http::{BlockingResponse, Response},
    stream::Streamer,
    ws::{BlockingWebSocket, WebSocket, msg::Message},
};

/// Buffered bodies of known length up to this size are read on the event loop, and on a
/// blocking caller without first releasing the GIL.
const READ_ATTACHED: u64 = 64 * 1024;

/// The largest unread body the event loop polls for a response of `version`: `limit` up to
/// HTTP/1.1, else 0. An HTTP/2 stream shares its connection's state behind a lock that the
/// connection task holds while it handles frames, so polling one would stall the loop.
fn loop_limit(version: wreq::Version, limit: u64) -> u64 {
    if version <= wreq::Version::HTTP_11 {
        limit
    } else {
        0
    }
}
