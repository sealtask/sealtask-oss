//! Explicit, bounded low-level transport for note requests and responses.
//!
//! Note payloads can approach several MiB. This module deliberately keeps JSON
//! encoding and decoding synchronous so callers can place that CPU work on
//! their own bounded blocking executor before or after using the async HTTP
//! methods on [`crate::PublicApiClient`]. The encoded wrappers are tied to
//! sealed request and response marker types, so arbitrary bytes cannot be sent
//! through the note API and a response cannot be decoded as an unrelated type.

use crate::note_transport_limits::{
    MAX_NOTE_DECOMPRESSED_PAGE_BYTES, MAX_NOTE_MUTATION_REQUEST_BYTES,
};
use crate::{BoundedHttpResponse, decode_bounded_json, map_api_error_with_retry_after};
use sealtask_client_core::{PublicError, PublicResult};
use serde::{Serialize, de::DeserializeOwned};
use std::any::type_name;
use std::fmt;
use std::marker::PhantomData;
use std::time::Duration;

pub(crate) mod sealed {
    pub trait Sealed {}
}

/// A request type accepted by the bounded note transport.
///
/// This trait is sealed; only the note request models owned by this crate can
/// be encoded into [`EncodedNoteRequest`].
pub trait NoteRequestPayload: sealed::Sealed + Serialize {}

/// A response type returned by the bounded note transport.
///
/// This trait is sealed; only the response markers owned by this crate can be
/// decoded from [`EncodedNoteResponse`].
pub trait NoteResponsePayload: sealed::Sealed {
    #[doc(hidden)]
    fn decode(response: EncodedNoteResponse<Self>) -> PublicResult<Self>
    where
        Self: Sized;
}

/// Marker returned by a successful note deletion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeleteNoteResponse;

/// A size-checked JSON request bound to one concrete note request model.
pub struct EncodedNoteRequest<T: NoteRequestPayload> {
    body: Vec<u8>,
    marker: PhantomData<fn() -> T>,
}

impl<T: NoteRequestPayload> EncodedNoteRequest<T> {
    /// Serializes and validates one typed note request.
    ///
    /// This method is intentionally synchronous. Callers operating from async
    /// code must invoke it through a bounded blocking executor.
    pub fn encode(payload: &T) -> PublicResult<Self> {
        let body = serde_json::to_vec(payload)
            .map_err(|_| PublicError::unexpected("failed to encode note request"))?;
        validate_request_size(&body)?;
        Ok(Self {
            body,
            marker: PhantomData,
        })
    }

    /// Returns the exact encoded request size.
    #[must_use]
    pub fn encoded_len(&self) -> usize {
        self.body.len()
    }

    pub(crate) fn into_body(self) -> Vec<u8> {
        self.body
    }
}

impl<T: NoteRequestPayload> fmt::Debug for EncodedNoteRequest<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EncodedNoteRequest")
            .field("payload_type", &type_name::<T>())
            .field("encoded_len", &self.body.len())
            .finish_non_exhaustive()
    }
}

/// A bounded HTTP response tied to one concrete note response model.
///
/// The response body is intentionally omitted from `Debug` output.
pub struct EncodedNoteResponse<T: NoteResponsePayload> {
    path: String,
    status: u16,
    retry_after: Option<Duration>,
    received_len: usize,
    body: PublicResult<Vec<u8>>,
    marker: PhantomData<fn() -> T>,
}

impl<T: NoteResponsePayload> EncodedNoteResponse<T> {
    #[cfg(test)]
    pub(crate) fn from_http(
        path: String,
        response: (reqwest::StatusCode, Vec<u8>),
    ) -> PublicResult<Self> {
        validate_response_size(&response.1)?;
        Ok(Self {
            path,
            status: response.0.as_u16(),
            retry_after: None,
            received_len: response.1.len(),
            body: Ok(response.1),
            marker: PhantomData,
        })
    }

    pub(crate) fn from_bounded_http(path: String, response: BoundedHttpResponse) -> Self {
        let status = response.status().as_u16();
        let retry_after = response.retry_after();
        let received_len = response.received_len();
        let body = response
            .into_body()
            .and_then(|body| validate_response_size(&body).map(|()| body));
        Self {
            path,
            status,
            retry_after,
            received_len,
            body,
            marker: PhantomData,
        }
    }

    pub(crate) fn from_complete_bounded_http(
        path: String,
        response: BoundedHttpResponse,
    ) -> PublicResult<Self> {
        if !response.status().is_success() {
            return Ok(Self::from_bounded_http(path, response));
        }
        let status = response.status().as_u16();
        let retry_after = response.retry_after();
        let received_len = response.received_len();
        let body = response.into_body()?;
        validate_response_size(&body)?;
        Ok(Self {
            path,
            status,
            retry_after,
            received_len,
            body: Ok(body),
            marker: PhantomData,
        })
    }

    /// Returns the number of decompressed response bytes observed by the transport.
    #[must_use]
    pub fn encoded_len(&self) -> usize {
        self.received_len
    }

    /// Reports whether the server returned a successful HTTP status.
    ///
    /// This does not parse or expose the response body.
    #[must_use]
    pub fn is_success_status(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// Validates the HTTP status and decodes the response's declared model.
    ///
    /// This method is intentionally synchronous. Callers operating from async
    /// code must invoke it through a bounded blocking executor.
    pub fn decode(self) -> PublicResult<T> {
        T::decode(self)
    }

    fn into_success_body(self) -> PublicResult<(String, Vec<u8>)> {
        let Self {
            path,
            status,
            retry_after,
            body,
            ..
        } = self;
        if (200..300).contains(&status) {
            body.map(|body| (path, body))
        } else {
            match body {
                Ok(body) => {
                    let error_text = String::from_utf8_lossy(&body);
                    Err(map_api_error_with_retry_after(
                        status,
                        &error_text,
                        &path,
                        retry_after,
                    ))
                }
                Err(_) => Err(map_api_error_with_retry_after(
                    status,
                    "",
                    &path,
                    retry_after,
                )),
            }
        }
    }
}

impl<T: NoteResponsePayload> fmt::Debug for EncodedNoteResponse<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EncodedNoteResponse")
            .field("payload_type", &type_name::<T>())
            .field("path", &self.path)
            .field("status", &self.status)
            .field("has_retry_after", &self.retry_after.is_some())
            .field("encoded_len", &self.received_len)
            .field("body_complete", &self.body.is_ok())
            .finish_non_exhaustive()
    }
}

pub(crate) fn decode_json_response<T>(response: EncodedNoteResponse<T>) -> PublicResult<T>
where
    T: NoteResponsePayload + DeserializeOwned,
{
    let (path, body) = response.into_success_body()?;
    let _ = path;
    decode_bounded_json(&body)
}

fn validate_request_size(body: &[u8]) -> PublicResult<()> {
    if body.len() > MAX_NOTE_MUTATION_REQUEST_BYTES {
        return Err(PublicError::payload_too_large(format!(
            "encoded note request exceeds the {MAX_NOTE_MUTATION_REQUEST_BYTES}-byte limit"
        )));
    }
    Ok(())
}

fn validate_response_size(body: &[u8]) -> PublicResult<()> {
    if body.len() > MAX_NOTE_DECOMPRESSED_PAGE_BYTES {
        return Err(PublicError::payload_too_large(format!(
            "note response exceeds the {MAX_NOTE_DECOMPRESSED_PAGE_BYTES}-byte limit"
        )));
    }
    Ok(())
}

impl sealed::Sealed for DeleteNoteResponse {}

impl NoteResponsePayload for DeleteNoteResponse {
    fn decode(response: EncodedNoteResponse<Self>) -> PublicResult<Self> {
        response.into_success_body().map(|_| Self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_size_validation_accepts_exact_limit_and_rejects_one_over() {
        validate_request_size(&vec![b'a'; MAX_NOTE_MUTATION_REQUEST_BYTES])
            .expect("exact encoded request boundary");
        let error = validate_request_size(&vec![b'a'; MAX_NOTE_MUTATION_REQUEST_BYTES + 1])
            .expect_err("one encoded byte over the request limit");
        assert!(matches!(error, PublicError::PayloadTooLarge(_)));
    }

    #[test]
    fn response_size_validation_accepts_exact_limit_and_rejects_one_over() {
        validate_response_size(&vec![b'a'; MAX_NOTE_DECOMPRESSED_PAGE_BYTES])
            .expect("exact encoded response boundary");
        let error = validate_response_size(&vec![b'a'; MAX_NOTE_DECOMPRESSED_PAGE_BYTES + 1])
            .expect_err("one encoded byte over the response limit");
        assert!(matches!(error, PublicError::PayloadTooLarge(_)));
    }

    #[test]
    fn response_debug_redacts_body() {
        let secret = "secret-response-canary";
        let response = EncodedNoteResponse::<DeleteNoteResponse>::from_http(
            "/work-lists/example/notes".to_string(),
            (reqwest::StatusCode::OK, secret.as_bytes().to_vec()),
        )
        .expect("bounded response");

        let debug = format!("{response:?}");
        assert!(!debug.contains(secret));
        assert!(debug.contains(&secret.len().to_string()));
    }
}
