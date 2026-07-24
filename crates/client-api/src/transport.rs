use std::time::Duration;

use sealtask_client_auth::{Credentials, refresh_credentials_if_needed};
use sealtask_client_core::{PublicError, PublicResult, ResponseFailureKind, TransportFailureKind};
use serde::{Deserialize, Serialize};

const ACCESS_TOKEN_REFRESH_WINDOW_SECONDS: i64 = 60;
const API_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const API_READ_TIMEOUT: Duration = Duration::from_secs(30);
const API_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
pub(crate) const MAX_RETRY_AFTER_SECONDS: u64 = 24 * 60 * 60;

#[derive(Debug, Clone)]
pub struct PublicApiClient {
    client: reqwest::Client,
    base_url: String,
    credentials: Option<Credentials>,
}

pub(crate) struct BoundedHttpResponse {
    status: reqwest::StatusCode,
    retry_after: Option<Duration>,
    received_len: usize,
    body: PublicResult<Vec<u8>>,
}

impl BoundedHttpResponse {
    fn complete(status: reqwest::StatusCode, retry_after: Option<Duration>, body: Vec<u8>) -> Self {
        Self {
            status,
            retry_after,
            received_len: body.len(),
            body: Ok(body),
        }
    }

    fn failed(
        status: reqwest::StatusCode,
        retry_after: Option<Duration>,
        received_len: usize,
        error: PublicError,
    ) -> Self {
        Self {
            status,
            retry_after,
            received_len,
            body: Err(error),
        }
    }

    pub(crate) fn status(&self) -> reqwest::StatusCode {
        self.status
    }

    pub(crate) fn received_len(&self) -> usize {
        self.received_len
    }

    pub(crate) const fn retry_after(&self) -> Option<Duration> {
        self.retry_after
    }

    pub(crate) fn into_body(self) -> PublicResult<Vec<u8>> {
        self.body
    }
}

impl PublicApiClient {
    pub fn new(base_url: impl Into<String>) -> PublicResult<Self> {
        Ok(Self {
            client: build_control_plane_client()?,
            base_url: normalize_base_url(base_url.into()),
            credentials: None,
        })
    }

    pub fn with_credentials(
        base_url: impl Into<String>,
        credentials: Credentials,
    ) -> PublicResult<Self> {
        Ok(Self {
            client: build_control_plane_client()?,
            base_url: normalize_base_url(base_url.into()),
            credentials: Some(credentials),
        })
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn has_credentials(&self) -> bool {
        self.credentials.is_some()
    }

    pub fn into_credentials(self) -> Option<Credentials> {
        self.credentials
    }

    async fn get_access_token(&mut self) -> PublicResult<String> {
        let credentials = self
            .credentials
            .as_mut()
            .ok_or_else(|| PublicError::validation("not logged in"))?;

        if credentials.access_expires_within(ACCESS_TOKEN_REFRESH_WINDOW_SECONDS) {
            *credentials = refresh_credentials_if_needed(
                &self.client,
                &self.base_url,
                credentials,
                ACCESS_TOKEN_REFRESH_WINDOW_SECONDS,
            )
            .await?;
        }

        Ok(credentials.access_token.clone())
    }

    pub(crate) async fn get<T: for<'de> Deserialize<'de>>(
        &mut self,
        path: &str,
    ) -> PublicResult<T> {
        let url = format!("{}{}", self.base_url, path);
        self.send(self.client.get(url), path).await
    }

    pub(crate) async fn get_bounded<T: for<'de> Deserialize<'de>>(
        &mut self,
        path: &str,
        max_decompressed_bytes: usize,
    ) -> PublicResult<T> {
        let url = format!("{}{}", self.base_url, path);
        self.send_bounded(self.client.get(url), path, max_decompressed_bytes)
            .await
    }

    pub(crate) async fn get_bounded_body(
        &mut self,
        path: &str,
        max_decompressed_bytes: usize,
    ) -> PublicResult<BoundedHttpResponse> {
        let url = format!("{}{}", self.base_url, path);
        self.send_bounded_body(self.client.get(url), path, max_decompressed_bytes)
            .await
    }

    pub(crate) async fn post<T: for<'de> Deserialize<'de>, B: Serialize>(
        &mut self,
        path: &str,
        body: &B,
    ) -> PublicResult<T> {
        let url = format!("{}{}", self.base_url, path);
        self.send(self.client.post(url).json(body), path).await
    }

    pub(crate) async fn patch<T: for<'de> Deserialize<'de>, B: Serialize>(
        &mut self,
        path: &str,
        body: &B,
    ) -> PublicResult<T> {
        let url = format!("{}{}", self.base_url, path);
        self.send(self.client.patch(url).json(body), path).await
    }

    pub(crate) async fn post_bounded<T: for<'de> Deserialize<'de>, B: Serialize>(
        &mut self,
        path: &str,
        body: &B,
        max_decompressed_bytes: usize,
    ) -> PublicResult<T> {
        let url = format!("{}{}", self.base_url, path);
        self.send_bounded(
            self.client.post(url).json(body),
            path,
            max_decompressed_bytes,
        )
        .await
    }

    pub(crate) async fn post_no_content_bounded<B: Serialize>(
        &mut self,
        path: &str,
        body: &B,
        max_decompressed_bytes: usize,
    ) -> PublicResult<()> {
        let url = format!("{}{}", self.base_url, path);
        self.send_no_content_bounded(
            self.client.post(url).json(body),
            path,
            max_decompressed_bytes,
        )
        .await
    }

    pub(crate) async fn delete_no_content_bounded(
        &mut self,
        path: &str,
        max_decompressed_bytes: usize,
    ) -> PublicResult<()> {
        let url = format!("{}{}", self.base_url, path);
        self.send_no_content_bounded(self.client.delete(url), path, max_decompressed_bytes)
            .await
    }

    pub(crate) async fn delete_no_content_with_body<B: Serialize>(
        &mut self,
        path: &str,
        body: &B,
    ) -> PublicResult<()> {
        let url = format!("{}{}", self.base_url, path);
        self.send_no_content(self.client.delete(url).json(body), path)
            .await
    }

    async fn send<T: for<'de> Deserialize<'de>>(
        &mut self,
        request: reqwest::RequestBuilder,
        path: &str,
    ) -> PublicResult<T> {
        let token = self.get_access_token().await?;
        let response = self
            .authorized(request, &token)
            .send()
            .await
            .map_err(|err| map_reqwest_error(err, path))?;

        handle_response(response, path).await
    }

    async fn send_bounded<T: for<'de> Deserialize<'de>>(
        &mut self,
        request: reqwest::RequestBuilder,
        path: &str,
        max_decompressed_bytes: usize,
    ) -> PublicResult<T> {
        let token = self.get_access_token().await?;
        let response = self
            .authorized(request, &token)
            .send()
            .await
            .map_err(map_bounded_transport_error)?;

        handle_bounded_response(response, path, max_decompressed_bytes).await
    }

    pub(crate) async fn send_json_bytes_bounded_body(
        &mut self,
        method: reqwest::Method,
        path: &str,
        body: Vec<u8>,
        max_decompressed_bytes: usize,
    ) -> PublicResult<BoundedHttpResponse> {
        let url = format!("{}{}", self.base_url, path);
        let request = self
            .client
            .request(method, url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body);
        self.send_bounded_body_preserving_status(request, path, max_decompressed_bytes)
            .await
    }

    async fn send_bounded_body(
        &mut self,
        request: reqwest::RequestBuilder,
        path: &str,
        max_decompressed_bytes: usize,
    ) -> PublicResult<BoundedHttpResponse> {
        self.send_bounded_body_preserving_status(request, path, max_decompressed_bytes)
            .await
    }

    async fn send_bounded_body_preserving_status(
        &mut self,
        request: reqwest::RequestBuilder,
        _path: &str,
        max_decompressed_bytes: usize,
    ) -> PublicResult<BoundedHttpResponse> {
        let token = self.get_access_token().await?;
        let response = self
            .authorized(request, &token)
            .send()
            .await
            .map_err(map_bounded_transport_error)?;

        Ok(read_bounded_response_body_preserving_status(response, max_decompressed_bytes).await)
    }

    async fn send_no_content(
        &mut self,
        request: reqwest::RequestBuilder,
        path: &str,
    ) -> PublicResult<()> {
        let token = self.get_access_token().await?;
        let response = self
            .authorized(request, &token)
            .send()
            .await
            .map_err(|err| map_reqwest_error(err, path))?;

        handle_empty_response(response, path).await
    }

    async fn send_no_content_bounded(
        &mut self,
        request: reqwest::RequestBuilder,
        path: &str,
        max_decompressed_bytes: usize,
    ) -> PublicResult<()> {
        let token = self.get_access_token().await?;
        let response = self
            .authorized(request, &token)
            .send()
            .await
            .map_err(map_bounded_transport_error)?;

        handle_bounded_empty_response(response, path, max_decompressed_bytes).await
    }

    fn authorized(
        &self,
        request: reqwest::RequestBuilder,
        access_token: &str,
    ) -> reqwest::RequestBuilder {
        request.bearer_auth(access_token)
    }
}

#[derive(Debug, Deserialize)]
struct ApiErrorResponse {
    pub error: String,
}

async fn handle_response<T: for<'de> Deserialize<'de>>(
    response: reqwest::Response,
    path: &str,
) -> PublicResult<T> {
    let status = response.status();
    let retry_after = parse_retry_after(response.headers());
    if status.is_success() {
        response.json().await.map_err(|error| {
            if error.is_timeout() {
                PublicError::transport(TransportFailureKind::Timeout)
            } else if error.is_connect() {
                PublicError::transport(TransportFailureKind::Connect)
            } else if error.is_body() {
                PublicError::transport(TransportFailureKind::Body)
            } else {
                PublicError::response(
                    ResponseFailureKind::JsonSchema,
                    "API response JSON does not match the expected schema",
                )
            }
        })
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "unknown error".to_string());
        Err(map_api_error_with_retry_after(
            status.as_u16(),
            &error_text,
            path,
            retry_after,
        ))
    }
}

async fn handle_bounded_response<T: for<'de> Deserialize<'de>>(
    response: reqwest::Response,
    path: &str,
    max_decompressed_bytes: usize,
) -> PublicResult<T> {
    let response =
        read_bounded_response_body_preserving_status(response, max_decompressed_bytes).await;
    if response.status().is_success() {
        decode_bounded_json(&response.into_body()?)
    } else {
        Err(map_bounded_error_response(response, path))
    }
}

async fn handle_bounded_empty_response(
    response: reqwest::Response,
    path: &str,
    max_decompressed_bytes: usize,
) -> PublicResult<()> {
    let response =
        read_bounded_response_body_preserving_status(response, max_decompressed_bytes).await;
    if response.status().is_success() {
        response.into_body().map(|_| ())
    } else {
        Err(map_bounded_error_response(response, path))
    }
}

fn map_bounded_error_response(response: BoundedHttpResponse, path: &str) -> PublicError {
    let status = response.status().as_u16();
    let retry_after = response.retry_after();
    let body = response.into_body().unwrap_or_default();
    let error_text = String::from_utf8_lossy(&body);
    map_api_error_with_retry_after(status, &error_text, path, retry_after)
}

async fn read_bounded_response_body_preserving_status(
    mut response: reqwest::Response,
    max_decompressed_bytes: usize,
) -> BoundedHttpResponse {
    let status = response.status();
    let retry_after = parse_retry_after(response.headers());
    let expected_len = response.content_length();
    let mut body = Vec::new();
    loop {
        let chunk = match response.chunk().await {
            Ok(Some(chunk)) => chunk,
            Ok(None) => return BoundedHttpResponse::complete(status, retry_after, body),
            Err(error) => {
                let received_len = body.len();
                return BoundedHttpResponse::failed(
                    status,
                    retry_after,
                    received_len,
                    map_bounded_body_error(&error, expected_len, received_len),
                );
            }
        };
        let Some(next_len) = body.len().checked_add(chunk.len()) else {
            return BoundedHttpResponse::failed(
                status,
                retry_after,
                usize::MAX,
                PublicError::response(
                    ResponseFailureKind::BodyTooLarge,
                    "API response body size overflowed the client safety bound",
                ),
            );
        };
        if next_len > max_decompressed_bytes {
            return BoundedHttpResponse::failed(
                status,
                retry_after,
                next_len,
                PublicError::response(
                    ResponseFailureKind::BodyTooLarge,
                    format!("API response body exceeds the {max_decompressed_bytes}-byte limit"),
                ),
            );
        }
        body.extend_from_slice(&chunk);
    }
}

async fn handle_empty_response(response: reqwest::Response, path: &str) -> PublicResult<()> {
    let status = response.status();
    let retry_after = parse_retry_after(response.headers());
    if status.is_success() {
        Ok(())
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "unknown error".to_string());
        Err(map_api_error_with_retry_after(
            status.as_u16(),
            &error_text,
            path,
            retry_after,
        ))
    }
}

fn normalize_base_url(value: String) -> String {
    value.trim_end_matches('/').to_string()
}

fn build_control_plane_client() -> PublicResult<reqwest::Client> {
    reqwest::Client::builder()
        .hickory_dns(true)
        .connect_timeout(API_CONNECT_TIMEOUT)
        .read_timeout(API_READ_TIMEOUT)
        .timeout(API_REQUEST_TIMEOUT)
        .build()
        .map_err(|err| PublicError::unexpected(format!("failed to configure API client: {err}")))
}

fn map_reqwest_error(err: reqwest::Error, _path: &str) -> PublicError {
    if err.is_timeout() {
        PublicError::transport(TransportFailureKind::Timeout)
    } else if err.is_connect() {
        PublicError::transport(TransportFailureKind::Connect)
    } else {
        PublicError::transport(TransportFailureKind::Other)
    }
}

fn map_bounded_transport_error(error: reqwest::Error) -> PublicError {
    let kind = if error.is_timeout() {
        TransportFailureKind::Timeout
    } else if error.is_connect() {
        TransportFailureKind::Connect
    } else {
        TransportFailureKind::Other
    };
    PublicError::transport(kind)
}

fn map_bounded_body_error(
    error: &reqwest::Error,
    expected_len: Option<u64>,
    received_len: usize,
) -> PublicError {
    if error.is_timeout() {
        return PublicError::transport(TransportFailureKind::Timeout);
    }
    if error.is_connect() {
        return PublicError::transport(TransportFailureKind::Connect);
    }
    if expected_len.is_some_and(|expected| {
        u64::try_from(received_len).is_ok_and(|received| received < expected)
    }) {
        return PublicError::response(
            ResponseFailureKind::BodyTruncated,
            "API response body ended before its declared length",
        );
    }
    PublicError::transport(TransportFailureKind::Body)
}

pub(crate) fn decode_bounded_json<T: for<'de> Deserialize<'de>>(body: &[u8]) -> PublicResult<T> {
    serde_json::from_slice(body).map_err(map_bounded_json_error)
}

fn map_bounded_json_error(error: serde_json::Error) -> PublicError {
    let (kind, message) = match error.classify() {
        serde_json::error::Category::Data => (
            ResponseFailureKind::JsonSchema,
            "API response JSON does not match the expected schema",
        ),
        serde_json::error::Category::Eof | serde_json::error::Category::Syntax => (
            ResponseFailureKind::JsonMalformed,
            "API response contains malformed JSON",
        ),
        serde_json::error::Category::Io => (
            ResponseFailureKind::BodyRead,
            "API response body could not be read",
        ),
    };
    PublicError::response(kind, message)
}

pub(crate) fn parse_retry_after(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
    let value = headers
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim();
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let seconds = value.parse::<u64>().unwrap_or(MAX_RETRY_AFTER_SECONDS);
    Some(Duration::from_secs(seconds.min(MAX_RETRY_AFTER_SECONDS)))
}

#[cfg(test)]
pub(crate) fn map_api_error(status: u16, body: &str, path: &str) -> PublicError {
    map_api_error_with_retry_after(status, body, path, None)
}

pub(crate) fn map_api_error_with_retry_after(
    status: u16,
    body: &str,
    _path: &str,
    retry_after: Option<Duration>,
) -> PublicError {
    if let Ok(api_error) = serde_json::from_str::<ApiErrorResponse>(body) {
        return PublicError::http(status, Some(api_error.error), retry_after);
    }

    PublicError::http(status, None, retry_after)
}
