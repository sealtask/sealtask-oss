use assert_cmd::Command;
use chrono::{Duration as ChronoDuration, Utc};
use sealtask_client_auth::Credentials;
use serde_json::Value;
use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tempfile::TempDir;
use uuid::Uuid;

const ACCESS_TOKEN_SECRET: &str = "access-token-secret-canary";
const REFRESH_TOKEN_SECRET: &str = "refresh-token-secret-canary";
const SERVER_ERROR_SECRET: &str = "server-error-secret-canary";

#[derive(Debug)]
struct ObservedRequest {
    method_and_path: String,
    request_id: Option<String>,
    has_expected_authorization: bool,
}

struct RetryServer {
    api_url: String,
    requests: Arc<Mutex<Vec<ObservedRequest>>>,
    shutdown: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl RetryServer {
    fn start(user_id: Uuid) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind retry test server");
        listener
            .set_nonblocking(true)
            .expect("make retry test server nonblocking");
        let api_url = format!(
            "http://{}",
            listener.local_addr().expect("retry test server address")
        );
        let requests = Arc::new(Mutex::new(Vec::new()));
        let requests_for_thread = Arc::clone(&requests);
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_for_thread = Arc::clone(&shutdown);

        let thread = thread::spawn(move || {
            while !shutdown_for_thread.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        handle_retry_request(stream, user_id, &requests_for_thread);
                    }
                    Err(error) if error.kind() == ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("retry test server failed to accept request: {error}"),
                }
            }
        });

        Self {
            api_url,
            requests,
            shutdown,
            thread: Some(thread),
        }
    }

    fn finish(mut self) -> Vec<ObservedRequest> {
        self.shutdown.store(true, Ordering::SeqCst);
        self.thread
            .take()
            .expect("retry test server thread")
            .join()
            .expect("retry test server completed");
        Arc::try_unwrap(self.requests)
            .expect("retry request observations are no longer shared")
            .into_inner()
            .expect("retry request observations lock")
    }
}

fn handle_retry_request(
    mut stream: TcpStream,
    user_id: Uuid,
    requests: &Arc<Mutex<Vec<ObservedRequest>>>,
) {
    stream
        .set_nonblocking(false)
        .expect("make accepted retry test stream blocking");
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("set request read timeout");
    let request = read_request_headers(&mut stream);
    let request_number = {
        let mut observed = requests.lock().expect("retry request observations lock");
        observed.push(ObservedRequest {
            method_and_path: request.lines().next().unwrap_or_default().to_string(),
            request_id: request_header(&request, "x-request-id").map(str::to_string),
            has_expected_authorization: request_header(&request, "authorization")
                == Some(format!("Bearer {ACCESS_TOKEN_SECRET}").as_str()),
        });
        observed.len()
    };

    let (status, body) = if request_number == 1 {
        (
            "503 Service Unavailable",
            format!(r#"{{"error":"temporary","message":"{SERVER_ERROR_SECRET}"}}"#),
        )
    } else {
        (
            "200 OK",
            serde_json::json!({
                "id": user_id,
                "email": "operator@example.test",
                "name": "Retry Operator",
                "timezone": "UTC",
                "avatarColor": "blue",
                "dataKeyCiphertext": "fixture-data-key",
                "workspaceLockTimeoutMinutes": 30,
                "themePreference": "system",
                "emailVerified": true,
                "lastAccessedWorkListId": null
            })
            .to_string(),
        )
    };
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(response.as_bytes())
        .expect("write retry test response");
}

fn read_request_headers(stream: &mut TcpStream) -> String {
    let mut request = Vec::new();
    let mut chunk = [0_u8; 1024];
    loop {
        let read = stream.read(&mut chunk).expect("read retry test request");
        assert!(read > 0, "request ended before its headers");
        request.extend_from_slice(&chunk[..read]);
        assert!(request.len() <= 64 * 1024, "request headers were too large");
        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }
    String::from_utf8(request).expect("retry test request headers are UTF-8")
}

fn request_header<'a>(request: &'a str, expected_name: &str) -> Option<&'a str> {
    request.lines().skip(1).find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.eq_ignore_ascii_case(expected_name)
            .then(|| value.trim())
    })
}

fn seed_credentials(local_state: &TempDir, api_url: &str, user_id: Uuid) {
    let config_dir = local_state.path().join("sealtask-config");
    fs::create_dir_all(&config_dir).expect("create CLI config directory");
    let credentials = Credentials {
        api_url: api_url.to_string(),
        access_token: ACCESS_TOKEN_SECRET.to_string(),
        refresh_token: REFRESH_TOKEN_SECRET.to_string(),
        access_expires_at: Utc::now() + ChronoDuration::hours(1),
        refresh_expires_at: Utc::now() + ChronoDuration::hours(2),
        user_id,
        email: "operator@example.test".to_string(),
        data_key_ciphertext: "fixture-data-key".to_string(),
    };
    let credentials_path = config_dir.join("credentials.json");
    fs::write(
        &credentials_path,
        serde_json::to_vec_pretty(&credentials).expect("serialize credentials fixture"),
    )
    .expect("write credentials fixture");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        fs::set_permissions(&config_dir, fs::Permissions::from_mode(0o700))
            .expect("secure CLI config directory");
        fs::set_permissions(&credentials_path, fs::Permissions::from_mode(0o600))
            .expect("secure credentials fixture");
    }
}

fn assert_no_secret_leak(output: &std::process::Output) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    for secret in [
        ACCESS_TOKEN_SECRET,
        REFRESH_TOKEN_SECRET,
        SERVER_ERROR_SECRET,
    ] {
        assert!(!stdout.contains(secret), "stdout leaked a secret canary");
        assert!(!stderr.contains(secret), "stderr leaked a secret canary");
    }
}

fn cli(local_state: &TempDir) -> Command {
    let mut command = Command::cargo_bin("sealtask").expect("sealtask binary");
    command
        .current_dir(local_state.path())
        .env("HOME", local_state.path())
        .env(
            "SEALTASK_CONFIG_DIR",
            local_state.path().join("sealtask-config"),
        )
        .env("NO_COLOR", "1")
        .env_remove("SEALTASK_RETRY")
        .env_remove("SEALTASK_API_URL")
        .env_remove("SEALTASK_PROFILE")
        .env_remove("SEALTASK_PAGER")
        .env_remove("PAGER")
        .env_remove("CLICOLOR")
        .env_remove("CLICOLOR_FORCE");
    command
}

fn configured_retries(output: std::process::Output) -> u64 {
    assert!(
        output.status.success(),
        "info failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let info: Value = serde_json::from_slice(&output.stdout).expect("JSON info");
    info["retries"]["configuredRetries"]
        .as_u64()
        .expect("configured retry count")
}

#[test]
fn info_json_reports_the_default_retry_limit() {
    let local_state = TempDir::new().expect("temporary local state");
    let output = cli(&local_state)
        .args(["info", "--json"])
        .output()
        .expect("run CLI");

    assert_eq!(configured_retries(output), 2);
}

#[test]
fn retry_flag_accepts_both_supported_boundaries() {
    for expected in [0, 10] {
        let local_state = TempDir::new().expect("temporary local state");
        let output = cli(&local_state)
            .args(["--retry", &expected.to_string(), "--json", "info"])
            .output()
            .expect("run CLI");

        assert_eq!(configured_retries(output), expected);
    }
}

#[test]
fn retry_limit_can_be_configured_from_the_environment() {
    let local_state = TempDir::new().expect("temporary local state");
    let output = cli(&local_state)
        .env("SEALTASK_RETRY", "6")
        .args(["--json", "info"])
        .output()
        .expect("run CLI");

    assert_eq!(configured_retries(output), 6);
}

#[test]
fn invalid_retry_values_fail_in_clap_before_authentication() {
    for invalid in ["11", "not-a-number"] {
        let local_state = TempDir::new().expect("temporary local state");
        let output = cli(&local_state)
            .args(["--retry", invalid, "me"])
            .output()
            .expect("run CLI");

        assert_eq!(
            output.status.code(),
            Some(2),
            "unexpected status for {invalid}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8(output.stderr).expect("UTF-8 stderr");
        assert!(stderr.contains("--retry <COUNT>"));
        assert!(stderr.contains(invalid));
        assert!(!stderr.to_ascii_lowercase().contains("credential"));
        assert!(!stderr.to_ascii_lowercase().contains("authentication"));
    }
}

#[test]
fn debug_telemetry_reports_retry_limit_without_url_secrets() {
    let local_state = TempDir::new().expect("temporary local state");
    let output = cli(&local_state)
        .env(
            "SEALTASK_CONFIG_DIR",
            local_state.path().join("config-secret-canary"),
        )
        .env("SEALTASK_PROFILE", "profile-secret-canary")
        .args([
            "--debug",
            "--retry",
            "7",
            "--api-url",
            "https://example.test:8443/private-secret-canary",
            "info",
        ])
        .output()
        .expect("run CLI");

    assert!(output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("UTF-8 stderr");
    assert!(stderr.contains("event=config"));
    assert!(stderr.contains("api_origin=https://example.test:8443"));
    assert!(stderr.contains("retry_limit=7"));
    for secret in [
        "config-secret-canary",
        "profile-secret-canary",
        "/private-secret-canary",
    ] {
        assert!(
            !stderr.contains(secret),
            "debug telemetry leaked URL secret {secret:?}: {stderr}"
        );
    }
}

#[test]
fn replay_safe_get_retries_a_503_and_reuses_its_request_id() {
    let local_state = TempDir::new().expect("temporary local state");
    let user_id = Uuid::new_v4();
    let server = RetryServer::start(user_id);
    seed_credentials(&local_state, &server.api_url, user_id);

    let output = cli(&local_state)
        .args(["--json", "--api-url", &server.api_url, "me"])
        .output()
        .expect("run CLI");
    let requests = server.finish();

    assert!(
        output.status.success(),
        "me failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    assert_no_secret_leak(&output);
    let user: Value = serde_json::from_slice(&output.stdout).expect("JSON user");
    assert_eq!(user["id"], user_id.to_string());

    assert_eq!(requests.len(), 2, "default retry should make two attempts");
    assert!(
        requests
            .iter()
            .all(|request| request.method_and_path == "GET /me HTTP/1.1")
    );
    assert!(
        requests
            .iter()
            .all(|request| request.has_expected_authorization)
    );
    let request_id = requests[0]
        .request_id
        .as_deref()
        .expect("first attempt request ID");
    Uuid::parse_str(request_id).expect("request ID is a UUID");
    assert_eq!(
        requests[1].request_id.as_deref(),
        Some(request_id),
        "retry must preserve the logical request ID"
    );
}

#[test]
fn retry_zero_is_exactly_single_shot_for_a_replay_safe_get() {
    let local_state = TempDir::new().expect("temporary local state");
    let user_id = Uuid::new_v4();
    let server = RetryServer::start(user_id);
    seed_credentials(&local_state, &server.api_url, user_id);

    let output = cli(&local_state)
        .args(["--retry", "0", "--json", "--api-url", &server.api_url, "me"])
        .output()
        .expect("run CLI");
    let requests = server.finish();

    assert_eq!(
        output.status.code(),
        Some(1),
        "single-shot 503 unexpectedly succeeded"
    );
    assert!(output.stdout.is_empty());
    assert_no_secret_leak(&output);
    let error: Value = serde_json::from_slice(&output.stderr).expect("JSON error");
    assert_eq!(error["error"]["code"], "http_server_error");
    assert_eq!(error["error"]["httpStatus"], 503);
    assert_eq!(error["error"]["retryable"], true);

    assert_eq!(requests.len(), 1, "--retry 0 must make exactly one attempt");
    assert_eq!(requests[0].method_and_path, "GET /me HTTP/1.1");
    assert!(requests[0].has_expected_authorization);
    let request_id = requests[0]
        .request_id
        .as_deref()
        .expect("single attempt request ID");
    Uuid::parse_str(request_id).expect("request ID is a UUID");
}
