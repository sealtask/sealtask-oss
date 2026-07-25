use chrono::{DateTime, Utc};
use hickory_resolver::TokioResolver;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Url, redirect};
use sealtask_client_core::{PublicError, PublicResult};
use std::collections::HashMap;
use std::future::Future;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

const STORAGE_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const STORAGE_READ_TIMEOUT: Duration = Duration::from_secs(30);
const STORAGE_TRANSFER_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const MIN_PRESIGNED_LIFETIME: Duration = Duration::from_secs(1);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StorageProxyPolicy {
    DirectOnly,
}

const STORAGE_PROXY_POLICY: StorageProxyPolicy = StorageProxyPolicy::DirectOnly;

#[derive(Clone)]
pub(crate) struct StorageTransferPolicy {
    trusted_origins: Arc<[TrustedOrigin]>,
    allow_local_http: bool,
    timeouts: StorageTimeouts,
    resolver: Arc<dyn StorageResolver>,
}

impl std::fmt::Debug for StorageTransferPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StorageTransferPolicy")
            .field("trusted_origins", &self.trusted_origins)
            .field("allow_local_http", &self.allow_local_http)
            .field("timeouts", &self.timeouts)
            .field("resolver", &"<async DNS resolver>")
            .finish()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TrustedOrigin {
    scheme: String,
    host: String,
    port: u16,
}

#[derive(Clone, Copy, Debug)]
struct StorageTimeouts {
    connect: Duration,
    read: Duration,
    transfer: Duration,
}

type ResolveFuture = Pin<Box<dyn Future<Output = Result<Vec<IpAddr>, String>> + Send + 'static>>;

trait StorageResolver: Send + Sync {
    fn resolve(&self, host: String) -> ResolveFuture;
}

struct HickoryStorageResolver {
    resolver: TokioResolver,
}

impl HickoryStorageResolver {
    fn from_system_configuration() -> PublicResult<Self> {
        let builder = TokioResolver::builder_tokio().map_err(|err| {
            PublicError::unexpected(format!(
                "failed to configure attachment storage DNS resolver: {err}"
            ))
        })?;
        let resolver = builder.build().map_err(|err| {
            PublicError::unexpected(format!(
                "failed to build attachment storage DNS resolver: {err}"
            ))
        })?;
        Ok(Self { resolver })
    }
}

impl StorageResolver for HickoryStorageResolver {
    fn resolve(&self, host: String) -> ResolveFuture {
        let resolver = self.resolver.clone();
        Box::pin(async move {
            resolver
                .lookup_ip(host)
                .await
                .map(|lookup| lookup.iter().collect())
                .map_err(|err| err.to_string())
        })
    }
}

pub(crate) struct PreparedStorageRequest {
    pub(crate) client: reqwest::Client,
    pub(crate) url: Url,
    pub(crate) headers: HeaderMap,
}

impl std::fmt::Debug for PreparedStorageRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedStorageRequest")
            .field("client", &"<configured storage client>")
            .field("url", &"<redacted>")
            .field("headers", &"<redacted>")
            .finish()
    }
}

impl StorageTransferPolicy {
    pub(crate) fn new<I, S>(api_url: &str, additional_origins: I) -> PublicResult<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let api = Url::parse(api_url)
            .map_err(|err| PublicError::validation(format!("invalid API URL: {err}")))?;
        let api_origin = TrustedOrigin::from_url(&api, "API URL")?;
        let allow_local_http = api.scheme() == "http" && url_host_is_loopback(&api)?;

        if api.scheme() != "https" && !allow_local_http {
            return Err(PublicError::validation(
                "API URL must use HTTPS unless it targets the local loopback interface",
            ));
        }

        let mut trusted_origins = vec![api_origin];
        for origin in additional_origins {
            let origin = parse_explicit_origin(origin.as_ref(), allow_local_http)?;
            if !trusted_origins.contains(&origin) {
                trusted_origins.push(origin);
            }
        }

        Ok(Self {
            trusted_origins: trusted_origins.into(),
            allow_local_http,
            timeouts: StorageTimeouts {
                connect: STORAGE_CONNECT_TIMEOUT,
                read: STORAGE_READ_TIMEOUT,
                transfer: STORAGE_TRANSFER_TIMEOUT,
            },
            resolver: Arc::new(HickoryStorageResolver::from_system_configuration()?),
        })
    }

    pub(crate) async fn prepare(
        &self,
        raw_url: &str,
        raw_headers: &HashMap<String, String>,
        expires_at: DateTime<Utc>,
    ) -> PublicResult<PreparedStorageRequest> {
        let url = Url::parse(raw_url).map_err(|err| {
            PublicError::validation(format!("invalid attachment storage URL: {err}"))
        })?;
        validate_url_shape(&url, self.allow_local_http)?;
        let origin = TrustedOrigin::from_url(&url, "attachment storage URL")?;
        if !self.trusted_origins.contains(&origin) {
            return Err(PublicError::validation(format!(
                "attachment storage origin {}://{}:{} is not trusted; configure it with --storage-origin",
                origin.scheme, origin.host, origin.port
            )));
        }

        let headers = validate_storage_headers(raw_headers)?;
        let resolution_timeout = transfer_timeout(expires_at, self.timeouts.connect)?;
        let host = url
            .host_str()
            .ok_or_else(|| PublicError::validation("attachment storage URL must include a host"))?;
        let port = url.port_or_known_default().ok_or_else(|| {
            PublicError::validation("attachment storage URL must include a supported port")
        })?;
        let resolved = resolve_and_validate_host(
            host,
            port,
            self.allow_local_http,
            url.scheme() == "http",
            resolution_timeout,
            self.resolver.as_ref(),
        )
        .await?;
        let timeout = transfer_timeout(expires_at, self.timeouts.transfer)?;

        let builder = reqwest::Client::builder();
        let mut builder = match STORAGE_PROXY_POLICY {
            StorageProxyPolicy::DirectOnly => builder.no_proxy(),
        }
        .no_hickory_dns()
        .redirect(redirect::Policy::none())
        .connect_timeout(self.timeouts.connect.min(timeout))
        .read_timeout(self.timeouts.read.min(timeout))
        .timeout(timeout);
        if host.parse::<IpAddr>().is_err() {
            builder = builder.resolve_to_addrs(host, &resolved);
        }
        let client = builder.build().map_err(|err| {
            PublicError::unexpected(format!(
                "failed to configure attachment storage client: {err}"
            ))
        })?;

        Ok(PreparedStorageRequest {
            client,
            url,
            headers,
        })
    }

    #[cfg(test)]
    fn with_test_timeouts(mut self, timeouts: StorageTimeouts) -> Self {
        self.timeouts = timeouts;
        self
    }

    #[cfg(test)]
    fn with_test_resolver(mut self, resolver: Arc<dyn StorageResolver>) -> Self {
        self.resolver = resolver;
        self
    }
}

impl TrustedOrigin {
    fn from_url(url: &Url, description: &str) -> PublicResult<Self> {
        let host = url
            .host_str()
            .ok_or_else(|| PublicError::validation(format!("{description} must include a host")))?;
        let port = url.port_or_known_default().ok_or_else(|| {
            PublicError::validation(format!("{description} has an unsupported scheme"))
        })?;
        Ok(Self {
            scheme: url.scheme().to_ascii_lowercase(),
            host: host.to_ascii_lowercase(),
            port,
        })
    }
}

fn parse_explicit_origin(raw: &str, allow_local_http: bool) -> PublicResult<TrustedOrigin> {
    let url = Url::parse(raw.trim()).map_err(|err| {
        PublicError::validation(format!("invalid trusted storage origin '{raw}': {err}"))
    })?;
    validate_url_shape(&url, allow_local_http)?;
    if url.path() != "/" || url.query().is_some() {
        return Err(PublicError::validation(
            "trusted storage origins must not include a path or query",
        ));
    }
    TrustedOrigin::from_url(&url, "trusted storage origin")
}

fn validate_url_shape(url: &Url, allow_local_http: bool) -> PublicResult<()> {
    if !url.username().is_empty() || url.password().is_some() {
        return Err(PublicError::validation(
            "attachment storage URLs must not contain user information",
        ));
    }
    if url.fragment().is_some() {
        return Err(PublicError::validation(
            "attachment storage URLs must not contain fragments",
        ));
    }
    match url.scheme() {
        "https" => Ok(()),
        "http" if allow_local_http && url_host_is_loopback(url)? => Ok(()),
        _ => Err(PublicError::validation(
            "attachment storage URLs must use HTTPS; HTTP is allowed only with a loopback HTTP API",
        )),
    }
}

fn validate_storage_headers(raw: &HashMap<String, String>) -> PublicResult<HeaderMap> {
    let mut headers = HeaderMap::with_capacity(raw.len());
    for (name, value) in raw {
        let normalized = name.to_ascii_lowercase();
        if !storage_header_is_allowed(&normalized) {
            return Err(PublicError::validation(format!(
                "attachment storage response included disallowed header '{name}'"
            )));
        }
        let name = HeaderName::from_str(name).map_err(|_| {
            PublicError::validation("attachment storage response included an invalid header name")
        })?;
        let value = HeaderValue::from_str(value).map_err(|_| {
            PublicError::validation(format!(
                "attachment storage response included an invalid value for header '{name}'"
            ))
        })?;
        headers.insert(name, value);
    }
    Ok(headers)
}

fn storage_header_is_allowed(name: &str) -> bool {
    matches!(
        name,
        "content-length"
            | "content-type"
            | "content-md5"
            | "if-match"
            | "if-none-match"
            | "x-amz-checksum-crc32"
            | "x-amz-checksum-crc32c"
            | "x-amz-checksum-sha1"
            | "x-amz-checksum-sha256"
            | "x-amz-expected-bucket-owner"
            | "x-amz-sdk-checksum-algorithm"
            | "x-amz-security-token"
            | "x-amz-server-side-encryption"
            | "x-amz-server-side-encryption-aws-kms-key-id"
            | "x-amz-server-side-encryption-context"
    )
}

fn transfer_timeout(expires_at: DateTime<Utc>, maximum: Duration) -> PublicResult<Duration> {
    let remaining = (expires_at - Utc::now()).to_std().map_err(|_| {
        PublicError::validation("attachment storage capability has already expired")
    })?;
    if remaining < MIN_PRESIGNED_LIFETIME {
        return Err(PublicError::validation(
            "attachment storage capability expires too soon",
        ));
    }
    Ok(remaining.min(maximum))
}

async fn resolve_and_validate_host(
    host: &str,
    port: u16,
    allow_local: bool,
    require_loopback: bool,
    resolution_timeout: Duration,
    resolver: &dyn StorageResolver,
) -> PublicResult<Vec<SocketAddr>> {
    let addrs = if let Ok(ip) = host.parse::<IpAddr>() {
        vec![SocketAddr::new(ip, port)]
    } else {
        tokio::time::timeout(resolution_timeout, resolver.resolve(host.to_string()))
            .await
            .map_err(|_| PublicError::unexpected("attachment storage DNS resolution timed out"))?
            .map_err(|err| {
                PublicError::unexpected(format!("failed to resolve attachment storage host: {err}"))
            })?
            .into_iter()
            .map(|ip| SocketAddr::new(ip, port))
            .collect::<Vec<_>>()
    };
    if addrs.is_empty() {
        return Err(PublicError::unexpected(
            "attachment storage host did not resolve to an address",
        ));
    }
    for address in &addrs {
        if require_loopback && !address.ip().is_loopback() {
            return Err(PublicError::validation(
                "HTTP attachment storage is restricted to the loopback interface",
            ));
        }
        if ip_is_unsafe(address.ip()) && !(allow_local && address.ip().is_loopback()) {
            return Err(PublicError::validation(
                "attachment storage host resolves to a private or unsafe network address",
            ));
        }
    }
    Ok(addrs)
}

fn url_host_is_loopback(url: &Url) -> PublicResult<bool> {
    let host = url
        .host_str()
        .ok_or_else(|| PublicError::validation("API URL must include a host"))?;
    if host.eq_ignore_ascii_case("localhost") {
        return Ok(true);
    }
    Ok(host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback()))
}

fn ip_is_unsafe(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => ipv4_is_unsafe(ip),
        IpAddr::V6(ip) => ipv6_is_unsafe(ip),
    }
}

fn ipv4_is_unsafe(ip: Ipv4Addr) -> bool {
    let [a, b, ..] = ip.octets();
    ip.is_unspecified()
        || ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_multicast()
        || a == 0
        || (a == 100 && (64..=127).contains(&b))
        || (a == 198 && (18..=19).contains(&b))
        || a >= 240
}

fn ipv6_is_unsafe(ip: Ipv6Addr) -> bool {
    if let Some(ipv4) = ip.to_ipv4_mapped() {
        return ipv4_is_unsafe(ipv4);
    }
    let first = ip.segments()[0];
    ip.is_unspecified()
        || ip.is_loopback()
        || ip.is_multicast()
        || first & 0xfe00 == 0xfc00
        || first & 0xffc0 == 0xfe80
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::task::{Context, Poll};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    struct PendingResolver {
        dropped: Arc<AtomicBool>,
    }

    struct PendingResolution {
        dropped: Arc<AtomicBool>,
    }

    impl Future for PendingResolution {
        type Output = Result<Vec<IpAddr>, String>;

        fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
            Poll::Pending
        }
    }

    impl Drop for PendingResolution {
        fn drop(&mut self) {
            self.dropped.store(true, Ordering::Release);
        }
    }

    impl StorageResolver for PendingResolver {
        fn resolve(&self, _host: String) -> ResolveFuture {
            Box::pin(PendingResolution {
                dropped: self.dropped.clone(),
            })
        }
    }

    fn short_timeouts() -> StorageTimeouts {
        StorageTimeouts {
            connect: Duration::from_millis(50),
            read: Duration::from_millis(50),
            transfer: Duration::from_millis(100),
        }
    }

    #[test]
    fn rejects_unsafe_schemes_and_origins() {
        assert!(
            StorageTransferPolicy::new("http://example.com", std::iter::empty::<&str>()).is_err()
        );
        assert!(
            StorageTransferPolicy::new("https://example.com", ["http://storage.example"]).is_err()
        );
    }

    #[test]
    fn accepts_explicit_https_storage_origin() {
        let policy =
            StorageTransferPolicy::new("https://api.example", ["https://objects.example:443"])
                .expect("storage policy");
        assert_eq!(policy.trusted_origins.len(), 2);
    }

    #[test]
    fn dedicated_storage_client_is_configured_for_direct_connections_only() {
        assert_eq!(STORAGE_PROXY_POLICY, StorageProxyPolicy::DirectOnly);
    }

    #[test]
    fn prepared_request_debug_redacts_storage_capability_secrets() {
        let prepared = PreparedStorageRequest {
            client: reqwest::Client::new(),
            url: Url::parse("https://storage.example/object?signature=url-secret").expect("URL"),
            headers: HeaderMap::from_iter([(
                HeaderName::from_static("x-amz-security-token"),
                HeaderValue::from_static("header-secret"),
            )]),
        };

        let debug = format!("{prepared:?}");
        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("url-secret"));
        assert!(!debug.contains("header-secret"));
    }

    #[test]
    fn permits_loopback_http_only_for_loopback_http_api() {
        let policy = StorageTransferPolicy::new("http://127.0.0.1:3000", ["http://127.0.0.1:4000"])
            .expect("local storage policy");
        assert!(policy.allow_local_http);
    }

    #[test]
    fn rejects_dangerous_storage_headers() {
        for name in [
            "authorization",
            "cookie",
            "host",
            "proxy-authorization",
            "x-http-method-override",
        ] {
            let headers = HashMap::from([(name.to_string(), "secret".to_string())]);
            assert!(validate_storage_headers(&headers).is_err(), "{name}");
        }
        let headers = HashMap::from([(
            "content-type".to_string(),
            "application/octet-stream".to_string(),
        )]);
        assert!(validate_storage_headers(&headers).is_ok());
    }

    #[test]
    fn classifies_private_and_link_local_addresses_as_unsafe() {
        for ip in [
            "127.0.0.1",
            "10.0.0.1",
            "169.254.169.254",
            "192.168.1.1",
            "::1",
            "fe80::1",
            "fc00::1",
        ] {
            assert!(ip.parse::<IpAddr>().is_ok_and(ip_is_unsafe), "{ip}");
        }
        assert!(!ip_is_unsafe("8.8.8.8".parse().expect("public IP")));
    }

    #[tokio::test]
    async fn rejects_private_transfer_target_outside_local_policy() {
        let policy = StorageTransferPolicy::new("https://api.example", ["https://127.0.0.1:4443"])
            .expect("policy");
        let error = policy
            .prepare(
                "https://127.0.0.1:4443/object",
                &HashMap::new(),
                Utc::now() + chrono::Duration::minutes(1),
            )
            .await
            .expect_err("private target");
        assert!(error.to_string().contains("private or unsafe"));
    }

    #[tokio::test]
    async fn dns_timeout_drops_the_underlying_async_resolution() {
        let dropped = Arc::new(AtomicBool::new(false));
        let policy = StorageTransferPolicy::new("https://api.example", ["https://objects.example"])
            .expect("policy")
            .with_test_timeouts(StorageTimeouts {
                connect: Duration::from_millis(20),
                read: Duration::from_millis(20),
                transfer: Duration::from_millis(40),
            })
            .with_test_resolver(Arc::new(PendingResolver {
                dropped: dropped.clone(),
            }));

        let error = policy
            .prepare(
                "https://objects.example/object",
                &HashMap::new(),
                Utc::now() + chrono::Duration::minutes(1),
            )
            .await
            .expect_err("resolution must time out");

        assert!(error.to_string().contains("DNS resolution timed out"));
        assert!(dropped.load(Ordering::Acquire));
    }

    #[tokio::test]
    async fn local_policy_accepts_explicit_loopback_transfer_target() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let address = listener.local_addr().expect("address");
        let origin = format!("http://{address}");
        let policy =
            StorageTransferPolicy::new(&origin, std::iter::empty::<&str>()).expect("local policy");
        policy
            .prepare(
                &format!("{origin}/object"),
                &HashMap::new(),
                Utc::now() + chrono::Duration::minutes(1),
            )
            .await
            .expect("prepared local transfer");
    }

    #[tokio::test]
    async fn storage_client_does_not_follow_redirects() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let address = listener.local_addr().expect("address");
        let origin = format!("http://{address}");
        let policy = StorageTransferPolicy::new(&origin, std::iter::empty::<&str>())
            .expect("local policy")
            .with_test_timeouts(short_timeouts());
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("connection");
            let mut request = [0_u8; 1024];
            let _ = stream.read(&mut request).await.expect("request");
            stream
                .write_all(
                    b"HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:9/leak\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .await
                .expect("response");
        });
        let prepared = policy
            .prepare(
                &format!("{origin}/redirect"),
                &HashMap::new(),
                Utc::now() + chrono::Duration::minutes(1),
            )
            .await
            .expect("prepared request");
        let response = prepared
            .client
            .get(prepared.url)
            .send()
            .await
            .expect("redirect response");
        assert_eq!(response.status(), reqwest::StatusCode::FOUND);
        server.await.expect("server");
    }

    #[tokio::test]
    async fn stalled_storage_response_is_bounded_by_transfer_timeout() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let address = listener.local_addr().expect("address");
        let origin = format!("http://{address}");
        let policy = StorageTransferPolicy::new(&origin, std::iter::empty::<&str>())
            .expect("local policy")
            .with_test_timeouts(short_timeouts());
        let server = tokio::spawn(async move {
            let (_stream, _) = listener.accept().await.expect("connection");
            tokio::time::sleep(Duration::from_secs(5)).await;
        });
        let prepared = policy
            .prepare(
                &format!("{origin}/stalled"),
                &HashMap::new(),
                Utc::now() + chrono::Duration::minutes(1),
            )
            .await
            .expect("prepared request");
        let started = tokio::time::Instant::now();
        let error = prepared
            .client
            .put(prepared.url)
            .body(vec![1_u8])
            .send()
            .await
            .expect_err("stalled request must time out");
        assert!(error.is_timeout());
        assert!(started.elapsed() < Duration::from_secs(1));
        server.abort();
    }
}
