use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use base64::engine::general_purpose::{STANDARD_NO_PAD, URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use socket2::{Domain, SockAddr, Socket, Type};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;

use sealtask_client_auth::{config_dir, normalize_api_url};
use sealtask_client_core::{PublicError, PublicResult};
use sealtask_client_crypto::SymmetricKey;

const DAEMON_EXECUTABLE_ENV: &str = "SEALTASK_UNLOCK_DAEMON_EXECUTABLE";
const DAEMON_IO_POLL_INTERVAL: Duration = Duration::from_millis(5);
const DAEMON_IO_TIMEOUT: Duration = Duration::from_secs(1);
const DAEMON_MAX_REQUEST_BYTES: u64 = 64 * 1024;
const DAEMON_MAX_RESPONSE_BYTES: usize = 64 * 1024;
const DAEMON_STARTUP_POLL_INTERVAL: Duration = Duration::from_millis(20);
const DAEMON_STARTUP_TIMEOUT: Duration = Duration::from_secs(1);
const SOCKET_FILE_NAME: &str = "unlock.sock";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnlockStatus {
    pub unlocked: bool,
    pub session_key: Option<SessionKey>,
    pub expires_at_unix: Option<u64>,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionKey {
    pub api_url: String,
    pub user_id: Uuid,
    pub data_key_fingerprint: String,
}

#[derive(Debug, Serialize, Deserialize)]
enum DaemonRequest {
    Put {
        session_key: SessionKey,
        data_key_b64: String,
        expires_at_unix: u64,
    },
    Get {
        session_key: SessionKey,
    },
    Status {
        session_key: Option<SessionKey>,
    },
    Delete {
        session_key: SessionKey,
    },
    Shutdown,
}

#[derive(Debug, Serialize, Deserialize)]
enum DaemonResponse {
    Stored,
    Deleted,
    DataKey { data_key_b64: Option<String> },
    Status(UnlockStatus),
    Shutdown,
    Error { message: String },
}

#[derive(Debug, Default)]
struct UnlockStore {
    sessions: HashMap<SessionKey, StoredSession>,
}

#[derive(Debug, Clone)]
struct StoredSession {
    data_key_b64: String,
    expires_at_unix: u64,
}

impl UnlockStore {
    fn put(&mut self, session_key: SessionKey, session: StoredSession) {
        self.sessions.insert(session_key, session);
    }

    fn get(&mut self, session_key: &SessionKey) -> Option<String> {
        self.prune_expired();
        self.sessions
            .get(session_key)
            .map(|session| session.data_key_b64.clone())
    }

    fn delete(&mut self, session_key: &SessionKey) {
        self.prune_expired();
        self.sessions.remove(session_key);
    }

    fn status(&mut self, session_key: Option<&SessionKey>) -> UnlockStatus {
        self.prune_expired();

        match session_key {
            Some(session_key) => build_status(
                Some(session_key.clone()),
                self.sessions
                    .get(session_key)
                    .map(|session| session.expires_at_unix),
            ),
            None => self
                .sessions
                .iter()
                .next()
                .map(|(session_key, session)| {
                    build_status(Some(session_key.clone()), Some(session.expires_at_unix))
                })
                .unwrap_or_else(|| build_status(None, None)),
        }
    }

    fn prune_expired(&mut self) {
        let now = unix_now();
        self.sessions
            .retain(|_, session| session.expires_at_unix > now);
    }
}

pub fn socket_path() -> PublicResult<PathBuf> {
    socket_path_for_config_dir(&config_dir()?)
}

fn socket_path_for_config_dir(config_dir: &Path) -> PublicResult<PathBuf> {
    let preferred_path = config_dir.join(SOCKET_FILE_NAME);
    if SockAddr::unix(&preferred_path).is_ok() {
        return Ok(preferred_path);
    }

    let mut hasher = Sha256::new();
    hasher.update(config_dir.as_os_str().to_string_lossy().as_bytes());
    let digest = hasher.finalize();
    let directory_name = format!("sealtask-unlock-{}", URL_SAFE_NO_PAD.encode(&digest[..16]));
    let fallback_path = Path::new("/tmp")
        .join(directory_name)
        .join(SOCKET_FILE_NAME);
    SockAddr::unix(&fallback_path).map_err(|err| {
        PublicError::unexpected(format!(
            "failed to resolve a short unlock daemon socket path: {err}"
        ))
    })?;
    Ok(fallback_path)
}

pub fn session_key(
    api_url: &str,
    user_id: Uuid,
    data_key_ciphertext: &str,
) -> PublicResult<SessionKey> {
    let ciphertext_bytes = decode_base64(
        data_key_ciphertext.trim(),
        "invalid data key ciphertext for daemon session key",
    )?;

    let mut hasher = Sha256::new();
    hasher.update(ciphertext_bytes);
    let digest = hasher.finalize();

    Ok(SessionKey {
        api_url: normalize_api_url(api_url),
        user_id,
        data_key_fingerprint: STANDARD_NO_PAD.encode(digest),
    })
}

pub fn unlock_status(session_key: Option<&SessionKey>) -> PublicResult<UnlockStatus> {
    let response = match try_send_request(DaemonRequest::Status {
        session_key: session_key.cloned(),
    }) {
        Ok(response) => response,
        Err(err) if is_daemon_unavailable(&err) => {
            return Ok(build_status(session_key.cloned(), None));
        }
        Err(err) => {
            return Err(PublicError::unexpected(format!(
                "failed to query unlock daemon status: {err}"
            )));
        }
    };

    match response {
        DaemonResponse::Status(status) => Ok(status),
        DaemonResponse::Error { message } => Err(PublicError::unexpected(message)),
        _ => Err(PublicError::unexpected(
            "unexpected daemon response to status",
        )),
    }
}

pub fn unlock(
    session_key: &SessionKey,
    data_key: &SymmetricKey,
    ttl_seconds: u64,
) -> PublicResult<()> {
    let expires_at_unix = unlock_expiration(unix_now(), ttl_seconds)?;
    ensure_running()?;

    let response = send_request(DaemonRequest::Put {
        session_key: session_key.clone(),
        data_key_b64: STANDARD_NO_PAD.encode(data_key.as_bytes()),
        expires_at_unix,
    })?;

    match response {
        DaemonResponse::Stored => Ok(()),
        DaemonResponse::Error { message } => Err(PublicError::unexpected(message)),
        _ => Err(PublicError::unexpected(
            "unexpected daemon response to unlock",
        )),
    }
}

pub(crate) fn validate_ttl(ttl_seconds: u64) -> PublicResult<()> {
    unlock_expiration(unix_now(), ttl_seconds).map(|_| ())
}

fn unlock_expiration(now: u64, ttl_seconds: u64) -> PublicResult<u64> {
    if ttl_seconds == 0 {
        return Err(PublicError::validation(
            "unlock TTL must be greater than zero",
        ));
    }

    now.checked_add(ttl_seconds)
        .ok_or_else(|| PublicError::validation("unlock TTL is too large"))
}

pub fn fetch_data_key(session_key: &SessionKey) -> PublicResult<Option<SymmetricKey>> {
    let response = match try_send_request(DaemonRequest::Get {
        session_key: session_key.clone(),
    }) {
        Ok(response) => response,
        Err(err) if is_daemon_unavailable(&err) => return Ok(None),
        Err(err) => {
            return Err(PublicError::unexpected(format!(
                "failed to fetch data key from unlock daemon: {err}"
            )));
        }
    };

    match response {
        DaemonResponse::DataKey {
            data_key_b64: Some(data_key_b64),
        } => {
            let bytes = decode_base64(&data_key_b64, "invalid daemon data key")?;
            SymmetricKey::from_slice(&bytes).map(Some)
        }
        DaemonResponse::DataKey { data_key_b64: None } => Ok(None),
        DaemonResponse::Error { message } => Err(PublicError::unexpected(message)),
        _ => Err(PublicError::unexpected("unexpected daemon response to get")),
    }
}

pub fn lock() -> PublicResult<()> {
    let response = match try_send_request(DaemonRequest::Shutdown) {
        Ok(response) => response,
        Err(err) if is_daemon_unavailable(&err) => return Ok(()),
        Err(err) => {
            return Err(PublicError::unexpected(format!(
                "failed to lock unlock daemon: {err}"
            )));
        }
    };

    match response {
        DaemonResponse::Shutdown => Ok(()),
        DaemonResponse::Error { message } => Err(PublicError::unexpected(message)),
        _ => Err(PublicError::unexpected(
            "unexpected daemon response to shutdown",
        )),
    }
}

pub fn clear_session(session_key: &SessionKey) -> PublicResult<()> {
    let response = match try_send_request(DaemonRequest::Delete {
        session_key: session_key.clone(),
    }) {
        Ok(response) => response,
        Err(err) if is_daemon_unavailable(&err) => return Ok(()),
        Err(err) => {
            return Err(PublicError::unexpected(format!(
                "failed to clear unlock daemon session: {err}"
            )));
        }
    };

    match response {
        DaemonResponse::Deleted => Ok(()),
        DaemonResponse::Error { message } => Err(PublicError::unexpected(message)),
        _ => Err(PublicError::unexpected(
            "unexpected daemon response to delete",
        )),
    }
}

pub async fn serve(socket_path: &Path) -> PublicResult<()> {
    serve_with_timeout(socket_path, DAEMON_IO_TIMEOUT).await
}

async fn serve_with_timeout(socket_path: &Path, request_timeout: Duration) -> PublicResult<()> {
    let socket_dir = socket_path.parent().ok_or_else(|| {
        PublicError::unexpected(format!(
            "unlock daemon socket path has no parent: {}",
            socket_path.display()
        ))
    })?;
    if !socket_dir.exists() {
        std::fs::create_dir_all(socket_dir).map_err(|err| {
            PublicError::unexpected(format!(
                "failed to create unlock daemon directory {}: {err}",
                socket_dir.display()
            ))
        })?;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        std::fs::set_permissions(socket_dir, std::fs::Permissions::from_mode(0o700)).map_err(
            |err| {
                PublicError::unexpected(format!(
                    "failed to secure unlock daemon directory {}: {err}",
                    socket_dir.display()
                ))
            },
        )?;
    }

    if socket_path.exists() {
        std::fs::remove_file(socket_path).map_err(|err| {
            PublicError::unexpected(format!(
                "failed to remove stale unlock daemon socket {}: {err}",
                socket_path.display()
            ))
        })?;
    }

    let listener = tokio::net::UnixListener::bind(socket_path).map_err(|err| {
        PublicError::unexpected(format!(
            "failed to bind unlock daemon socket {}: {err}",
            socket_path.display()
        ))
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        std::fs::set_permissions(socket_path, std::fs::Permissions::from_mode(0o600)).map_err(
            |err| {
                PublicError::unexpected(format!(
                    "failed to secure unlock daemon socket {}: {err}",
                    socket_path.display()
                ))
            },
        )?;
    }

    let store = std::sync::Arc::new(tokio::sync::Mutex::new(UnlockStore::default()));

    loop {
        let (mut stream, _) = listener.accept().await.map_err(|err| {
            PublicError::unexpected(format!("unlock daemon accept failed: {err}"))
        })?;
        let store = store.clone();

        let Some(request) = read_daemon_request(&mut stream, request_timeout).await else {
            continue;
        };
        let response = handle_request(request, &store).await;
        let should_shutdown = matches!(response, DaemonResponse::Shutdown);
        let response_bytes = serde_json::to_vec(&response).map_err(|err| {
            PublicError::unexpected(format!("failed to encode unlock daemon response: {err}"))
        })?;
        let _ = tokio::time::timeout(request_timeout, stream.write_all(&response_bytes)).await;

        if should_shutdown {
            break;
        }
    }

    if socket_path.exists() {
        let _ = std::fs::remove_file(socket_path);
    }

    Ok(())
}

async fn read_daemon_request(
    stream: &mut tokio::net::UnixStream,
    request_timeout: Duration,
) -> Option<DaemonRequest> {
    let mut request_bytes = Vec::new();
    let read_result = tokio::time::timeout(
        request_timeout,
        stream
            .take(DAEMON_MAX_REQUEST_BYTES + 1)
            .read_to_end(&mut request_bytes),
    )
    .await;
    if !matches!(read_result, Ok(Ok(_))) || request_bytes.len() as u64 > DAEMON_MAX_REQUEST_BYTES {
        return None;
    }
    serde_json::from_slice(&request_bytes).ok()
}

async fn handle_request(
    request: DaemonRequest,
    store: &std::sync::Arc<tokio::sync::Mutex<UnlockStore>>,
) -> DaemonResponse {
    match request {
        DaemonRequest::Put {
            session_key,
            data_key_b64,
            expires_at_unix,
        } => {
            store.lock().await.put(
                session_key,
                StoredSession {
                    data_key_b64,
                    expires_at_unix,
                },
            );
            DaemonResponse::Stored
        }
        DaemonRequest::Get { session_key } => DaemonResponse::DataKey {
            data_key_b64: store.lock().await.get(&session_key),
        },
        DaemonRequest::Status { session_key } => {
            DaemonResponse::Status(store.lock().await.status(session_key.as_ref()))
        }
        DaemonRequest::Delete { session_key } => {
            store.lock().await.delete(&session_key);
            DaemonResponse::Deleted
        }
        DaemonRequest::Shutdown => DaemonResponse::Shutdown,
    }
}

fn send_request(request: DaemonRequest) -> PublicResult<DaemonResponse> {
    try_send_request(request).map_err(|err| {
        PublicError::unexpected(format!("failed to communicate with unlock daemon: {err}"))
    })
}

fn try_send_request(request: DaemonRequest) -> PublicResult<DaemonResponse> {
    let socket_path = socket_path()?;
    let deadline = deadline_after(DAEMON_IO_TIMEOUT)?;
    try_send_request_until(&socket_path, request, deadline)
}

fn try_send_request_until(
    socket_path: &Path,
    request: DaemonRequest,
    deadline: Instant,
) -> PublicResult<DaemonResponse> {
    let connect_timeout = remaining_until(deadline, "failed to connect to unlock daemon")?;
    let stream = connect_to_daemon(socket_path, connect_timeout)?;
    send_request_over_stream_until(stream, request, deadline)
}

fn connect_to_daemon(
    socket_path: &Path,
    timeout: Duration,
) -> PublicResult<std::os::unix::net::UnixStream> {
    let socket = Socket::new(Domain::UNIX, Type::STREAM, None).map_err(|err| {
        PublicError::unexpected(format!("failed to create unlock daemon socket: {err}"))
    })?;
    let address = SockAddr::unix(socket_path).map_err(|err| {
        PublicError::unexpected(format!(
            "failed to resolve unlock daemon socket {}: {err}",
            socket_path.display()
        ))
    })?;
    socket.connect_timeout(&address, timeout).map_err(|err| {
        let availability = if matches!(
            err.kind(),
            std::io::ErrorKind::NotFound | std::io::ErrorKind::ConnectionRefused
        ) {
            " (unavailable)"
        } else {
            ""
        };
        PublicError::unexpected(format!(
            "failed to connect to unlock daemon{availability} at {}: {err}",
            socket_path.display()
        ))
    })?;
    Ok(socket.into())
}

#[cfg(test)]
fn send_request_over_stream_with_timeout(
    stream: std::os::unix::net::UnixStream,
    request: DaemonRequest,
    timeout: Duration,
) -> PublicResult<DaemonResponse> {
    let deadline = deadline_after(timeout)?;
    send_request_over_stream_until(stream, request, deadline)
}

fn send_request_over_stream_until(
    mut stream: std::os::unix::net::UnixStream,
    request: DaemonRequest,
    deadline: Instant,
) -> PublicResult<DaemonResponse> {
    stream.set_nonblocking(true).map_err(|err| {
        PublicError::unexpected(format!(
            "failed to configure unlock daemon nonblocking I/O: {err}"
        ))
    })?;
    let payload = serde_json::to_vec(&request).map_err(|err| {
        PublicError::unexpected(format!("failed to encode unlock daemon request: {err}"))
    })?;
    use std::io::{Read, Write};
    let mut written = 0;
    while written < payload.len() {
        remaining_until(deadline, "failed to write unlock daemon request")?;
        match stream.write(&payload[written..]) {
            Ok(0) => {
                return Err(PublicError::unexpected(
                    "failed to write unlock daemon request: socket closed",
                ));
            }
            Ok(count) => written += count,
            Err(err) if err.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                wait_for_socket_progress(deadline, "failed to write unlock daemon request")?;
            }
            Err(err) => {
                return Err(PublicError::unexpected(format!(
                    "failed to write unlock daemon request: {err}"
                )));
            }
        }
    }
    stream.shutdown(std::net::Shutdown::Write).map_err(|err| {
        PublicError::unexpected(format!("failed to finish unlock daemon request: {err}"))
    })?;

    let mut response = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        remaining_until(deadline, "failed to read unlock daemon response")?;
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => {
                if response.len().saturating_add(count) > DAEMON_MAX_RESPONSE_BYTES {
                    return Err(PublicError::unexpected(
                        "unlock daemon response exceeded the size limit",
                    ));
                }
                response.extend_from_slice(&buffer[..count]);
            }
            Err(err) if err.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                wait_for_socket_progress(deadline, "failed to read unlock daemon response")?;
            }
            Err(err) => {
                return Err(PublicError::unexpected(format!(
                    "failed to read unlock daemon response: {err}"
                )));
            }
        }
    }

    serde_json::from_slice(&response).map_err(|err| {
        PublicError::unexpected(format!("failed to decode unlock daemon response: {err}"))
    })
}

fn deadline_after(timeout: Duration) -> PublicResult<Instant> {
    Instant::now()
        .checked_add(timeout)
        .ok_or_else(|| PublicError::validation("unlock daemon timeout is too large"))
}

fn remaining_until(deadline: Instant, operation: &str) -> PublicResult<Duration> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|remaining| !remaining.is_zero())
        .ok_or_else(|| PublicError::unexpected(format!("{operation}: operation timed out")))
}

fn wait_for_socket_progress(deadline: Instant, operation: &str) -> PublicResult<()> {
    let remaining = remaining_until(deadline, operation)?;
    std::thread::sleep(DAEMON_IO_POLL_INTERVAL.min(remaining));
    Ok(())
}

fn ensure_running() -> PublicResult<()> {
    match try_send_request(DaemonRequest::Status { session_key: None }) {
        Ok(_) => Ok(()),
        Err(err) if is_daemon_unavailable(&err) => spawn_daemon(),
        Err(err) => Err(PublicError::unexpected(format!(
            "failed to check unlock daemon: {err}"
        ))),
    }
}

fn spawn_daemon() -> PublicResult<()> {
    let socket_path = socket_path()?;
    let executable = resolve_daemon_executable()?;
    let mut command = daemon_spawn_command(&executable, &socket_path);

    command
        .spawn()
        .map_err(|err| PublicError::unexpected(format!("failed to start unlock daemon: {err}")))?;

    wait_for_daemon_ready(&socket_path, DAEMON_STARTUP_TIMEOUT)
}

fn wait_for_daemon_ready(socket_path: &Path, timeout: Duration) -> PublicResult<()> {
    let deadline = deadline_after(timeout)?;

    while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
        if remaining.is_zero() {
            break;
        }
        std::thread::sleep(DAEMON_STARTUP_POLL_INTERVAL.min(remaining));
        if try_send_request_until(
            socket_path,
            DaemonRequest::Status { session_key: None },
            deadline,
        )
        .is_ok()
        {
            return Ok(());
        }
    }

    Err(PublicError::unexpected(
        "unlock daemon did not become ready in time",
    ))
}

fn is_daemon_unavailable(err: &PublicError) -> bool {
    matches!(err, PublicError::Unexpected(message) if message.contains("failed to connect to unlock daemon (unavailable)"))
}

fn daemon_spawn_command(executable: &Path, socket_path: &Path) -> std::process::Command {
    let mut command = std::process::Command::new(executable);
    command
        .current_dir(std::env::temp_dir())
        .arg("--serve-unlock-daemon")
        .arg(socket_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

fn resolve_daemon_executable() -> PublicResult<PathBuf> {
    if let Some(configured) = configured_daemon_executable() {
        return Ok(configured);
    }

    #[cfg(target_os = "linux")]
    {
        let proc_self_exe = PathBuf::from("/proc/self/exe");
        if proc_self_exe.exists() {
            return Ok(proc_self_exe);
        }
    }

    let current_exe = std::env::current_exe().map_err(|err| {
        PublicError::unexpected(format!("failed to locate current executable: {err}"))
    })?;
    if current_exe.exists() {
        return Ok(current_exe);
    }

    Err(PublicError::unexpected(format!(
        "failed to locate unlock daemon executable: {} does not exist",
        current_exe.display()
    )))
}

fn configured_daemon_executable() -> Option<PathBuf> {
    let path = std::env::var_os(DAEMON_EXECUTABLE_ENV).map(PathBuf::from)?;
    path.exists().then_some(path)
}

fn build_status(session_key: Option<SessionKey>, expires_at_unix: Option<u64>) -> UnlockStatus {
    UnlockStatus {
        unlocked: expires_at_unix.is_some(),
        session_key,
        expires_at_unix,
    }
}

fn decode_base64(value: &str, message: &str) -> PublicResult<Vec<u8>> {
    STANDARD_NO_PAD
        .decode(value)
        .or_else(|_| base64::engine::general_purpose::STANDARD.decode(value))
        .map_err(|err| PublicError::validation(format!("{message}: {err}")))
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should be after unix epoch")
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    struct EnvVarGuard {
        previous: Option<OsString>,
    }

    impl EnvVarGuard {
        #[cfg(target_os = "linux")]
        fn clear() -> Self {
            let previous = std::env::var_os(DAEMON_EXECUTABLE_ENV);
            unsafe {
                std::env::remove_var(DAEMON_EXECUTABLE_ENV);
            }
            Self { previous }
        }

        fn set(path: &Path) -> Self {
            let previous = std::env::var_os(DAEMON_EXECUTABLE_ENV);
            unsafe {
                std::env::set_var(DAEMON_EXECUTABLE_ENV, path);
            }
            Self { previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match self.previous.as_ref() {
                Some(value) => unsafe {
                    std::env::set_var(DAEMON_EXECUTABLE_ENV, value);
                },
                None => unsafe {
                    std::env::remove_var(DAEMON_EXECUTABLE_ENV);
                },
            }
        }
    }

    #[test]
    fn daemon_spawn_command_uses_stable_working_directory() {
        let command = daemon_spawn_command(
            Path::new("/proc/self/exe"),
            Path::new("/tmp/sealtask-unlock.sock"),
        );

        let args = command.get_args().collect::<Vec<_>>();
        assert_eq!(command.get_program(), "/proc/self/exe");
        assert_eq!(
            command.get_current_dir(),
            Some(std::env::temp_dir().as_path())
        );
        assert_eq!(args.len(), 2);
        assert_eq!(args[0], "--serve-unlock-daemon");
        assert_eq!(args[1], "/tmp/sealtask-unlock.sock");
    }

    #[test]
    fn long_profile_config_paths_use_a_short_deterministic_socket_path() {
        let long_config_dir = PathBuf::from("/tmp").join("profile".repeat(40));

        let first = socket_path_for_config_dir(&long_config_dir).expect("short socket path");
        let second = socket_path_for_config_dir(&long_config_dir).expect("stable socket path");

        assert_eq!(first, second);
        assert_eq!(
            first.parent().and_then(Path::parent),
            Some(Path::new("/tmp"))
        );
        assert_eq!(
            first.file_name().and_then(|name| name.to_str()),
            Some(SOCKET_FILE_NAME)
        );
        SockAddr::unix(&first).expect("fallback must fit the platform socket limit");
    }

    #[test]
    fn test_should_reject_zero_and_overflowing_unlock_ttls() {
        assert!(matches!(
            unlock_expiration(10, 0),
            Err(PublicError::Validation(message))
                if message == "unlock TTL must be greater than zero"
        ));
        assert!(matches!(
            unlock_expiration(u64::MAX - 1, 2),
            Err(PublicError::Validation(message)) if message == "unlock TTL is too large"
        ));
        assert_eq!(unlock_expiration(10, 20).expect("valid TTL"), 30);
    }

    #[test]
    fn daemon_request_times_out_when_the_peer_never_replies() {
        use std::io::Read as _;

        let (client, mut peer) =
            std::os::unix::net::UnixStream::pair().expect("create daemon socket pair");
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let peer_thread = std::thread::spawn(move || {
            let mut request = Vec::new();
            peer.read_to_end(&mut request).expect("read daemon request");
            release_rx.recv().expect("release hanging daemon peer");
        });

        let started_at = std::time::Instant::now();
        let error = send_request_over_stream_with_timeout(
            client,
            DaemonRequest::Status { session_key: None },
            Duration::from_millis(25),
        )
        .expect_err("a daemon peer that never replies must time out");
        let elapsed = started_at.elapsed();

        release_tx.send(()).expect("release daemon peer");
        peer_thread.join().expect("join daemon peer");
        assert!(
            matches!(&error, PublicError::Unexpected(message) if message.contains("failed to read unlock daemon response")),
            "unexpected daemon timeout error: {error}"
        );
        assert!(
            elapsed < Duration::from_secs(1),
            "daemon timeout took too long: {elapsed:?}"
        );
    }

    #[test]
    fn daemon_readiness_wait_has_one_overall_budget() {
        let temp_dir = tempfile::TempDir::new().expect("create daemon socket directory");
        let socket_path = temp_dir.path().join("hanging-startup.sock");
        let _listener =
            std::os::unix::net::UnixListener::bind(&socket_path).expect("bind daemon listener");

        let timeout = Duration::from_millis(75);
        let started_at = Instant::now();
        let error = wait_for_daemon_ready(&socket_path, timeout)
            .expect_err("a daemon that never replies must not become ready");
        let elapsed = started_at.elapsed();

        assert!(
            matches!(&error, PublicError::Unexpected(message) if message == "unlock daemon did not become ready in time"),
            "unexpected daemon readiness error: {error}"
        );
        assert!(
            elapsed >= Duration::from_millis(50),
            "daemon readiness returned before exercising its budget: {elapsed:?}"
        );
        assert!(
            elapsed < Duration::from_millis(500),
            "daemon readiness exceeded its overall budget: {elapsed:?}"
        );
    }

    #[test]
    fn daemon_connect_is_bounded_when_the_listener_backlog_is_full() {
        let temp_dir = tempfile::TempDir::new().expect("create daemon socket directory");
        let socket_path = temp_dir.path().join("backlog.sock");
        let address = SockAddr::unix(&socket_path).expect("resolve daemon socket address");
        let listener =
            Socket::new(Domain::UNIX, Type::STREAM, None).expect("create daemon listener socket");
        listener.bind(&address).expect("bind daemon listener");
        listener.listen(1).expect("listen with a bounded backlog");

        let mut queued_clients = Vec::new();
        let mut saturated = false;
        for _ in 0..32 {
            match connect_to_daemon(&socket_path, Duration::from_millis(20)) {
                Ok(stream) => queued_clients.push(stream),
                // A bound listener with a full backlog refuses further connects, but the
                // exact signal is platform dependent: some kernels report a connect
                // timeout or connection refusal, while others surface a POLLHUP with no
                // socket error (`no error set after POLLHUP`). Once at least one client
                // is queued, any connect failure here means the backlog saturated, so
                // treat them uniformly.
                Err(PublicError::Unexpected(message))
                    if !queued_clients.is_empty()
                        && message.contains("failed to connect to unlock daemon") =>
                {
                    saturated = true;
                    break;
                }
                Err(error) => panic!("unexpected backlog connection error: {error}"),
            }
        }
        assert!(saturated, "daemon listener backlog did not saturate");

        let started_at = std::time::Instant::now();
        let error = connect_to_daemon(&socket_path, Duration::from_millis(25))
            .expect_err("a saturated daemon listener must not block connect indefinitely");
        let elapsed = started_at.elapsed();
        assert!(
            matches!(&error, PublicError::Unexpected(message) if message.contains("failed to connect to unlock daemon")),
            "unexpected daemon connect error: {error}"
        );
        assert!(
            elapsed < Duration::from_secs(1),
            "daemon connect timeout took too long: {elapsed:?}"
        );

        drop(queued_clients);
        drop(listener);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn daemon_server_drops_a_partial_request_and_serves_the_next_client() {
        let temp_dir = tempfile::TempDir::new().expect("create daemon socket directory");
        let socket_path = temp_dir.path().join("partial-request.sock");
        let server_path = socket_path.clone();
        let server = tokio::spawn(async move {
            serve_with_timeout(&server_path, Duration::from_millis(25)).await
        });
        let readiness_deadline = Instant::now() + Duration::from_secs(1);
        let mut partial = loop {
            match tokio::net::UnixStream::connect(&socket_path).await {
                Ok(stream) => break stream,
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::NotFound | std::io::ErrorKind::ConnectionRefused
                    ) && Instant::now() < readiness_deadline =>
                {
                    tokio::time::sleep(Duration::from_millis(2)).await;
                }
                Err(error) => panic!("daemon listener did not become ready: {error}"),
            }
        };
        partial
            .write_all(b"{")
            .await
            .expect("write partial daemon request");

        let status_path = socket_path.clone();
        let status_response = tokio::task::spawn_blocking(move || {
            let stream = connect_to_daemon(&status_path, Duration::from_millis(250))?;
            send_request_over_stream_with_timeout(
                stream,
                DaemonRequest::Status { session_key: None },
                Duration::from_millis(500),
            )
        })
        .await
        .expect("join status request")
        .expect("daemon should recover after the partial request");
        assert!(matches!(status_response, DaemonResponse::Status(_)));
        drop(partial);

        let shutdown_path = socket_path.clone();
        let shutdown_response = tokio::task::spawn_blocking(move || {
            let stream = connect_to_daemon(&shutdown_path, Duration::from_millis(250))?;
            send_request_over_stream_with_timeout(
                stream,
                DaemonRequest::Shutdown,
                Duration::from_millis(500),
            )
        })
        .await
        .expect("join shutdown request")
        .expect("shutdown daemon after partial request test");
        assert!(matches!(shutdown_response, DaemonResponse::Shutdown));
        tokio::time::timeout(Duration::from_secs(1), server)
            .await
            .expect("daemon server should stop")
            .expect("daemon server task should not panic")
            .expect("daemon server should stop cleanly");
    }

    #[test]
    fn resolve_daemon_executable_prefers_configured_path() {
        let _lock = env_lock().lock().expect("env lock");
        let configured = tempfile::NamedTempFile::new().expect("temp executable path");
        let _guard = EnvVarGuard::set(configured.path());

        let resolved = resolve_daemon_executable().expect("configured executable");
        assert_eq!(resolved, configured.path());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn resolve_daemon_executable_falls_back_to_proc_self_exe() {
        let _lock = env_lock().lock().expect("env lock");
        let _guard = EnvVarGuard::clear();

        let resolved = resolve_daemon_executable().expect("fallback executable");
        assert_eq!(resolved, PathBuf::from("/proc/self/exe"));
    }
}
