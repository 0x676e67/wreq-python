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

/// Whether the event loop may poll an unread body of `version`: up to HTTP/1.1, whose body
/// frames the connection task hands over, and never after it, not even an empty body. An
/// HTTP/2 stream shares its connection's state behind a lock that the connection task holds
/// while it handles frames, so polling one would stall the loop.
fn loop_polls(version: wreq::Version) -> bool {
    version <= wreq::Version::HTTP_11
}

/// The largest unread body the event loop collects for a response of `version`.
fn loop_limit(version: wreq::Version, limit: u64) -> Option<u64> {
    loop_polls(version).then_some(limit)
}
