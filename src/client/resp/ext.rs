use pyo3::pybacked::PyBackedStr;
use serde::de::DeserializeOwned;

use crate::{buffer::PyBuffer, error::Error};

/// Body readers for [`wreq::Response`] that return crate [`Error`]s, shared by the sync
/// and async responses.
pub trait ResponseExt {
    /// Decode the body with `encoding`, or the response charset defaulting to UTF-8.
    async fn text(self, encoding: Option<PyBackedStr>) -> Result<String, Error>;

    /// Deserialize the body as JSON.
    async fn json<T: DeserializeOwned>(self) -> Result<T, Error>;

    /// Read the whole body as a read-only buffer.
    async fn bytes(self) -> Result<PyBuffer, Error>;
}

impl ResponseExt for wreq::Response {
    #[inline]
    async fn text(self, encoding: Option<PyBackedStr>) -> Result<String, Error> {
        match encoding {
            Some(encoding) => self.text_with_charset(encoding).await,
            None => self.text().await,
        }
        .map_err(Error::Library)
    }

    #[inline]
    async fn json<T: DeserializeOwned>(self) -> Result<T, Error> {
        self.json::<T>().await.map_err(Error::Library)
    }

    #[inline]
    async fn bytes(self) -> Result<PyBuffer, Error> {
        self.bytes()
            .await
            .map(PyBuffer::from)
            .map_err(Error::Library)
    }
}
