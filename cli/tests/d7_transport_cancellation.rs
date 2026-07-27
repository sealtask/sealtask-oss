#![cfg(unix)]

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{Duration, Utc};
use sealtask_client_auth::{Credentials, normalize_api_url};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Output, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration as StdDuration, Instant};
use tempfile::TempDir;
use uuid::Uuid;

const ORIGINAL_ACCESS_TOKEN: &str = "original-access-token";
const ORIGINAL_REFRESH_TOKEN: &str = "original-refresh-token";
const ROTATED_ACCESS_TOKEN: &str = "rotated-access-token";
const ROTATED_REFRESH_TOKEN: &str = "rotated-refresh-token";

#[test]
fn task_create_signal_during_rotation_persists_credentials_without_sending_the_mutation() {
    let local_state = TempDir::new().expect("temporary local state");
    let config_dir = local_state.path().join("config");
    let keychain_dir = local_state.path().join("keychain");
    let work_list_id = Uuid::now_v7();
    let user_id = Uuid::now_v7();
    let (refresh_seen_sender, refresh_seen) = mpsc::channel();
    let (release_refresh, refresh_release) = mpsc::channel();
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind refresh server");
    let api_url = format!("http://{}", listener.local_addr().expect("server address"));
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("refresh connection");
        let refresh_request = read_request(&mut stream);
        refresh_seen_sender
            .send(())
            .expect("notify refresh observation");
        refresh_release
            .recv_timeout(StdDuration::from_secs(5))
            .expect("release refresh response");
        let body = serde_json::json!({
            "accessToken": ROTATED_ACCESS_TOKEN,
            "refreshToken": ROTATED_REFRESH_TOKEN,
            "expiresIn": 3_600,
            "refreshExpiresIn": 7_200,
            "tokenType": "Bearer"
        })
        .to_string();
        write_response(&mut stream, "200 OK", &body);
        drop(stream);

        let later_requests = collect_later_requests(&listener);
        (refresh_request, later_requests)
    });

    let credentials = Credentials {
        api_url: api_url.clone(),
        access_token: ORIGINAL_ACCESS_TOKEN.to_string(),
        refresh_token: ORIGINAL_REFRESH_TOKEN.to_string(),
        access_expires_at: Utc::now() - Duration::minutes(1),
        refresh_expires_at: Utc::now() + Duration::hours(2),
        user_id,
        email: "operator@example.test".to_string(),
        data_key_ciphertext: URL_SAFE_NO_PAD.encode(b"data-key-binding"),
    };
    seed_local_state(&config_dir, &keychain_dir, &credentials);

    let binary = assert_cmd::cargo::cargo_bin!("sealtask");
    let child = Command::new(binary)
        .current_dir(local_state.path())
        .env("HOME", local_state.path())
        .env("SEALTASK_CONFIG_DIR", &config_dir)
        .env("SEALTASK_TEST_KEYCHAIN_DIR", &keychain_dir)
        .env_remove("SEALTASK_PROFILE")
        .env_remove("SEALTASK_API_URL")
        .env_remove("SEALTASK_RETRY")
        .env("NO_COLOR", "1")
        .args([
            "--json",
            "--api-url",
            &api_url,
            "--non-interactive",
            "tasks",
            "create",
            "--work-list-id",
            &work_list_id.to_string(),
            "--title",
            "must not be sent",
            "--idempotency-key",
            "refresh-interruption-contract",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn CLI");

    refresh_seen
        .recv_timeout(StdDuration::from_secs(10))
        .expect("CLI reaches credential refresh");
    let signal_result = unsafe { libc::kill(child.id().cast_signed(), libc::SIGINT) };
    assert_eq!(signal_result, 0, "send SIGINT");
    thread::sleep(StdDuration::from_millis(100));
    release_refresh.send(()).expect("release refresh response");

    let output = wait_for_output(child, StdDuration::from_secs(10));
    let (refresh_request, later_requests) = server.join().expect("refresh server");

    assert_eq!(output.status.code(), Some(130));
    assert!(output.stdout.is_empty());
    let error: serde_json::Value =
        serde_json::from_slice(&output.stderr).expect("typed JSON interruption");
    assert_eq!(error["error"]["code"], "interrupted");
    assert_eq!(error["error"]["outcome"], "interrupted");
    assert!(
        error["error"]["message"]
            .as_str()
            .is_some_and(|message| message.contains("durably persisted"))
    );

    assert!(refresh_request.starts_with("POST /auth/refresh "));
    assert!(
        later_requests.is_empty(),
        "no resource read or task mutation may follow cancellation: {later_requests:?}"
    );
    let persisted: Credentials = serde_json::from_slice(
        &fs::read(config_dir.join("credentials.json")).expect("read persisted credentials"),
    )
    .expect("decode persisted credentials");
    assert_eq!(persisted.access_token, ROTATED_ACCESS_TOKEN);
    assert_eq!(persisted.refresh_token, ROTATED_REFRESH_TOKEN);
}

#[test]
fn dropped_refresh_response_after_signal_reports_ambiguous_session_state() {
    let local_state = TempDir::new().expect("temporary local state");
    let config_dir = local_state.path().join("config");
    let keychain_dir = local_state.path().join("keychain");
    let work_list_id = Uuid::now_v7();
    let user_id = Uuid::now_v7();
    let (refresh_seen_sender, refresh_seen) = mpsc::channel();
    let (release_refresh, refresh_release) = mpsc::channel();
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind refresh server");
    let api_url = format!("http://{}", listener.local_addr().expect("server address"));
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("refresh connection");
        let refresh_request = read_request(&mut stream);
        refresh_seen_sender
            .send(())
            .expect("notify refresh observation");
        refresh_release
            .recv_timeout(StdDuration::from_secs(5))
            .expect("release dropped refresh response");
        drop(stream);
        let later_requests = collect_later_requests(&listener);
        (refresh_request, later_requests)
    });

    let credentials = Credentials {
        api_url: api_url.clone(),
        access_token: ORIGINAL_ACCESS_TOKEN.to_string(),
        refresh_token: ORIGINAL_REFRESH_TOKEN.to_string(),
        access_expires_at: Utc::now() - Duration::minutes(1),
        refresh_expires_at: Utc::now() + Duration::hours(2),
        user_id,
        email: "operator@example.test".to_string(),
        data_key_ciphertext: URL_SAFE_NO_PAD.encode(b"data-key-binding"),
    };
    seed_local_state(&config_dir, &keychain_dir, &credentials);

    let binary = assert_cmd::cargo::cargo_bin!("sealtask");
    let child = Command::new(binary)
        .current_dir(local_state.path())
        .env("HOME", local_state.path())
        .env("SEALTASK_CONFIG_DIR", &config_dir)
        .env("SEALTASK_TEST_KEYCHAIN_DIR", &keychain_dir)
        .env_remove("SEALTASK_PROFILE")
        .env_remove("SEALTASK_API_URL")
        .env_remove("SEALTASK_RETRY")
        .env("NO_COLOR", "1")
        .args([
            "--json",
            "--api-url",
            &api_url,
            "--non-interactive",
            "tasks",
            "create",
            "--work-list-id",
            &work_list_id.to_string(),
            "--title",
            "must not be sent",
            "--idempotency-key",
            "lost-refresh-response-contract",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn CLI");

    refresh_seen
        .recv_timeout(StdDuration::from_secs(10))
        .expect("CLI reaches credential refresh");
    let signal_result = unsafe { libc::kill(child.id().cast_signed(), libc::SIGINT) };
    assert_eq!(signal_result, 0, "send SIGINT");
    thread::sleep(StdDuration::from_millis(100));
    release_refresh
        .send(())
        .expect("drop accepted refresh response");

    let output = wait_for_output(child, StdDuration::from_secs(10));
    let (refresh_request, later_requests) = server.join().expect("refresh server");

    assert_eq!(output.status.code(), Some(130));
    assert!(output.stdout.is_empty());
    let error: serde_json::Value =
        serde_json::from_slice(&output.stderr).expect("typed JSON interruption");
    assert_eq!(error["error"]["code"], "interrupted");
    assert_eq!(error["error"]["outcome"], "ambiguous");
    assert!(
        error["error"]["message"]
            .as_str()
            .is_some_and(|message| message.contains("rotated the session"))
    );
    assert!(
        error["error"]["hint"]
            .as_str()
            .is_some_and(|hint| hint.contains("auth login"))
    );
    assert!(refresh_request.starts_with("POST /auth/refresh "));
    assert!(
        later_requests.is_empty(),
        "no resource request may follow a lost refresh response: {later_requests:?}"
    );

    let persisted: Credentials = serde_json::from_slice(
        &fs::read(config_dir.join("credentials.json")).expect("read persisted credentials"),
    )
    .expect("decode persisted credentials");
    assert_eq!(persisted.access_token, ORIGINAL_ACCESS_TOKEN);
    assert_eq!(persisted.refresh_token, ORIGINAL_REFRESH_TOKEN);
}

#[test]
fn dropped_refresh_response_during_batch_resolution_preserves_session_ambiguity() {
    let local_state = TempDir::new().expect("temporary local state");
    let config_dir = local_state.path().join("config");
    let keychain_dir = local_state.path().join("keychain");
    let input_path = local_state.path().join("batch.jsonl");
    let user_id = Uuid::now_v7();
    let (refresh_seen_sender, refresh_seen) = mpsc::channel();
    let (release_refresh, refresh_release) = mpsc::channel();
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind refresh server");
    let api_url = format!("http://{}", listener.local_addr().expect("server address"));
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("refresh connection");
        let refresh_request = read_request(&mut stream);
        refresh_seen_sender
            .send(())
            .expect("notify refresh observation");
        refresh_release
            .recv_timeout(StdDuration::from_secs(5))
            .expect("release dropped refresh response");
        drop(stream);
        let later_requests = collect_later_requests(&listener);
        (refresh_request, later_requests)
    });

    let credentials = Credentials {
        api_url: api_url.clone(),
        access_token: ORIGINAL_ACCESS_TOKEN.to_string(),
        refresh_token: ORIGINAL_REFRESH_TOKEN.to_string(),
        access_expires_at: Utc::now() - Duration::minutes(1),
        refresh_expires_at: Utc::now() + Duration::hours(2),
        user_id,
        email: "operator@example.test".to_string(),
        data_key_ciphertext: URL_SAFE_NO_PAD.encode(b"data-key-binding"),
    };
    seed_local_state(&config_dir, &keychain_dir, &credentials);
    let operation = serde_json::json!({
        "schemaVersion": 1,
        "operationId": "batch-resolution-refresh",
        "type": "task.create",
        "project": "slug:release-project",
        "input": {
            "title": "batch resolution plaintext must not be sent"
        }
    });
    fs::write(
        &input_path,
        format!(
            "{}\n",
            serde_json::to_string(&operation).expect("encode batch operation")
        ),
    )
    .expect("write batch input");

    let binary = assert_cmd::cargo::cargo_bin!("sealtask");
    let child = Command::new(binary)
        .current_dir(local_state.path())
        .env("HOME", local_state.path())
        .env("SEALTASK_CONFIG_DIR", &config_dir)
        .env("SEALTASK_TEST_KEYCHAIN_DIR", &keychain_dir)
        .env_remove("SEALTASK_PROFILE")
        .env_remove("SEALTASK_API_URL")
        .env_remove("SEALTASK_RETRY")
        .env("NO_COLOR", "1")
        .args([
            "--format",
            "jsonl",
            "--api-url",
            &api_url,
            "--non-interactive",
            "batch",
            "run",
            "--input",
            input_path.to_str().expect("UTF-8 batch path"),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn batch CLI");

    refresh_seen
        .recv_timeout(StdDuration::from_secs(10))
        .expect("batch reaches credential refresh during target resolution");
    let signal_result = unsafe { libc::kill(child.id().cast_signed(), libc::SIGINT) };
    assert_eq!(signal_result, 0, "send SIGINT");
    thread::sleep(StdDuration::from_millis(100));
    release_refresh
        .send(())
        .expect("drop accepted refresh response");

    let output = wait_for_output(child, StdDuration::from_secs(10));
    let (refresh_request, later_requests) = server.join().expect("refresh server");

    assert_eq!(output.status.code(), Some(130));
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 JSONL output");
    assert!(!stdout.contains("batch resolution plaintext must not be sent"));
    let summary: serde_json::Value = serde_json::from_str(
        stdout
            .lines()
            .last()
            .expect("batch emits a terminal summary"),
    )
    .expect("decode batch summary");
    assert_eq!(summary["type"], "batch.summary");
    assert_eq!(summary["interrupted"], true);
    assert_eq!(summary["notRun"], 1);

    let error: serde_json::Value =
        serde_json::from_slice(&output.stderr).expect("typed JSON interruption");
    assert_eq!(error["error"]["code"], "interrupted");
    assert_eq!(error["error"]["outcome"], "ambiguous");
    assert!(
        error["error"]["hint"]
            .as_str()
            .is_some_and(|hint| hint.contains("auth login"))
    );
    assert!(refresh_request.starts_with("POST /auth/refresh "));
    assert!(
        later_requests.is_empty(),
        "batch must not issue target or mutation requests after a lost refresh response: {later_requests:?}"
    );
}

#[test]
fn simultaneous_signal_and_scheduled_refresh_loss_preserve_session_ambiguity() {
    let local_state = TempDir::new().expect("temporary local state");
    let config_dir = local_state.path().join("config");
    let keychain_dir = local_state.path().join("keychain");
    let input_path = local_state.path().join("batch.jsonl");
    let work_list_id = Uuid::now_v7();
    let user_id = Uuid::now_v7();
    let (refresh_seen_sender, refresh_seen) = mpsc::channel();
    let (release_refresh, refresh_release) = mpsc::channel();
    let (refresh_dropped_sender, refresh_dropped) = mpsc::channel();
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind refresh server");
    let api_url = format!("http://{}", listener.local_addr().expect("server address"));
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("refresh connection");
        let refresh_request = read_request(&mut stream);
        refresh_seen_sender
            .send(())
            .expect("notify refresh observation");
        refresh_release
            .recv_timeout(StdDuration::from_secs(5))
            .expect("release dropped refresh response");
        drop(stream);
        refresh_dropped_sender
            .send(())
            .expect("confirm refresh response loss");
        let later_requests = collect_later_requests(&listener);
        (refresh_request, later_requests)
    });

    let credentials = Credentials {
        api_url: api_url.clone(),
        access_token: ORIGINAL_ACCESS_TOKEN.to_string(),
        refresh_token: ORIGINAL_REFRESH_TOKEN.to_string(),
        access_expires_at: Utc::now() - Duration::minutes(1),
        refresh_expires_at: Utc::now() + Duration::hours(2),
        user_id,
        email: "operator@example.test".to_string(),
        data_key_ciphertext: URL_SAFE_NO_PAD.encode(b"data-key-binding"),
    };
    seed_local_state(&config_dir, &keychain_dir, &credentials);
    let operation = serde_json::json!({
        "schemaVersion": 1,
        "operationId": "batch-simultaneous-refresh-loss",
        "type": "task.create",
        "project": format!("id:{work_list_id}"),
        "input": {
            "title": "simultaneous refresh plaintext must not be sent"
        }
    });
    fs::write(
        &input_path,
        format!(
            "{}\n",
            serde_json::to_string(&operation).expect("encode batch operation")
        ),
    )
    .expect("write batch input");

    let binary = assert_cmd::cargo::cargo_bin!("sealtask");
    let child = Command::new(binary)
        .current_dir(local_state.path())
        .env("HOME", local_state.path())
        .env("SEALTASK_CONFIG_DIR", &config_dir)
        .env("SEALTASK_TEST_KEYCHAIN_DIR", &keychain_dir)
        .env_remove("SEALTASK_PROFILE")
        .env_remove("SEALTASK_API_URL")
        .env_remove("SEALTASK_RETRY")
        .env("NO_COLOR", "1")
        .args([
            "--format",
            "jsonl",
            "--api-url",
            &api_url,
            "--non-interactive",
            "batch",
            "run",
            "--input",
            input_path.to_str().expect("UTF-8 batch path"),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn batch CLI");

    refresh_seen
        .recv_timeout(StdDuration::from_secs(10))
        .expect("batch reaches credential refresh during scheduled preparation");
    let stop_result = unsafe { libc::kill(child.id().cast_signed(), libc::SIGSTOP) };
    assert_eq!(stop_result, 0, "pause CLI at the refresh boundary");
    release_refresh
        .send(())
        .expect("drop accepted refresh response");
    refresh_dropped
        .recv_timeout(StdDuration::from_secs(5))
        .expect("server drops response while CLI is paused");
    let signal_result = unsafe { libc::kill(child.id().cast_signed(), libc::SIGINT) };
    assert_eq!(signal_result, 0, "queue SIGINT while the CLI is paused");
    let continue_result = unsafe { libc::kill(child.id().cast_signed(), libc::SIGCONT) };
    assert_eq!(continue_result, 0, "resume CLI with both events ready");

    let output = wait_for_output(child, StdDuration::from_secs(10));
    let (refresh_request, later_requests) = server.join().expect("refresh server");

    assert_eq!(
        output.status.code(),
        Some(130),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 JSONL output");
    assert!(stdout.contains("\"code\":\"session_outcome_ambiguous\""));
    assert!(!stdout.contains("simultaneous refresh plaintext must not be sent"));
    let summary: serde_json::Value = serde_json::from_str(
        stdout
            .lines()
            .last()
            .expect("batch emits a terminal summary"),
    )
    .expect("decode batch summary");
    assert_eq!(summary["type"], "batch.summary");
    assert_eq!(summary["interrupted"], true);

    let error: serde_json::Value =
        serde_json::from_slice(&output.stderr).expect("typed JSON interruption");
    assert_eq!(error["error"]["code"], "interrupted");
    assert_eq!(error["error"]["outcome"], "ambiguous");
    assert!(
        error["error"]["hint"]
            .as_str()
            .is_some_and(|hint| hint.contains("auth login"))
    );
    assert!(refresh_request.starts_with("POST /auth/refresh "));
    assert!(
        later_requests.is_empty(),
        "no resource request may follow simultaneous refresh loss and SIGINT: {later_requests:?}"
    );
}

#[test]
fn second_signal_during_batch_rotation_is_session_ambiguous_and_stops_promptly() {
    let local_state = TempDir::new().expect("temporary local state");
    let config_dir = local_state.path().join("config");
    let keychain_dir = local_state.path().join("keychain");
    let input_path = local_state.path().join("batch.jsonl");
    let work_list_id = Uuid::now_v7();
    let user_id = Uuid::now_v7();
    let (refresh_seen_sender, refresh_seen) = mpsc::channel();
    let (release_refresh, refresh_release) = mpsc::channel();
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind refresh server");
    let api_url = format!("http://{}", listener.local_addr().expect("server address"));
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("refresh connection");
        let refresh_request = read_request(&mut stream);
        refresh_seen_sender
            .send(())
            .expect("notify refresh observation");
        refresh_release
            .recv_timeout(StdDuration::from_secs(15))
            .expect("release force-stopped refresh");
        drop(stream);
        let later_requests = collect_later_requests(&listener);
        (refresh_request, later_requests)
    });

    let credentials = Credentials {
        api_url: api_url.clone(),
        access_token: ORIGINAL_ACCESS_TOKEN.to_string(),
        refresh_token: ORIGINAL_REFRESH_TOKEN.to_string(),
        access_expires_at: Utc::now() - Duration::minutes(1),
        refresh_expires_at: Utc::now() + Duration::hours(2),
        user_id,
        email: "operator@example.test".to_string(),
        data_key_ciphertext: URL_SAFE_NO_PAD.encode(b"data-key-binding"),
    };
    seed_local_state(&config_dir, &keychain_dir, &credentials);
    let operation = serde_json::json!({
        "schemaVersion": 1,
        "operationId": "batch-refresh-force-stop",
        "type": "task.create",
        "project": format!("id:{work_list_id}"),
        "input": {
            "title": "batch plaintext must not be sent"
        }
    });
    fs::write(
        &input_path,
        format!(
            "{}\n",
            serde_json::to_string(&operation).expect("encode batch operation")
        ),
    )
    .expect("write batch input");

    let binary = assert_cmd::cargo::cargo_bin!("sealtask");
    let child = Command::new(binary)
        .current_dir(local_state.path())
        .env("HOME", local_state.path())
        .env("SEALTASK_CONFIG_DIR", &config_dir)
        .env("SEALTASK_TEST_KEYCHAIN_DIR", &keychain_dir)
        .env_remove("SEALTASK_PROFILE")
        .env_remove("SEALTASK_API_URL")
        .env_remove("SEALTASK_RETRY")
        .env("NO_COLOR", "1")
        .args([
            "--format",
            "jsonl",
            "--api-url",
            &api_url,
            "--non-interactive",
            "batch",
            "run",
            "--input",
            input_path.to_str().expect("UTF-8 batch path"),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn batch CLI");

    refresh_seen
        .recv_timeout(StdDuration::from_secs(10))
        .expect("batch reaches credential refresh during operation preparation");
    let first_signal = unsafe { libc::kill(child.id().cast_signed(), libc::SIGINT) };
    assert_eq!(first_signal, 0, "send first SIGINT");
    thread::sleep(StdDuration::from_millis(100));
    let second_signal = unsafe { libc::kill(child.id().cast_signed(), libc::SIGINT) };
    assert_eq!(second_signal, 0, "send second SIGINT");

    let output = wait_for_output(child, StdDuration::from_secs(10));
    release_refresh
        .send(())
        .expect("release force-stopped refresh server");
    let (refresh_request, later_requests) = server.join().expect("refresh server");

    assert_eq!(output.status.code(), Some(130));
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 JSONL output");
    assert!(stdout.contains("\"code\":\"session_outcome_ambiguous\""));
    assert!(!stdout.contains("batch plaintext must not be sent"));
    let summary: serde_json::Value = serde_json::from_str(
        stdout
            .lines()
            .last()
            .expect("batch emits a terminal summary"),
    )
    .expect("decode batch summary");
    assert_eq!(summary["type"], "batch.summary");
    assert_eq!(summary["interrupted"], true);

    let error: serde_json::Value =
        serde_json::from_slice(&output.stderr).expect("typed JSON interruption");
    assert_eq!(error["error"]["code"], "interrupted");
    assert_eq!(error["error"]["outcome"], "ambiguous");
    assert!(
        error["error"]["hint"]
            .as_str()
            .is_some_and(|hint| hint.contains("auth login"))
    );
    assert!(refresh_request.starts_with("POST /auth/refresh "));
    assert!(
        later_requests.is_empty(),
        "batch must not send a resource request after forced refresh cancellation: {later_requests:?}"
    );
}

fn seed_local_state(
    config_dir: &std::path::Path,
    keychain_dir: &std::path::Path,
    credentials: &Credentials,
) {
    fs::create_dir_all(config_dir).expect("create config directory");
    fs::create_dir_all(keychain_dir).expect("create keychain directory");
    let credentials_path = config_dir.join("credentials.json");
    fs::write(
        &credentials_path,
        serde_json::to_vec_pretty(credentials).expect("serialize credentials"),
    )
    .expect("write credentials");

    let ciphertext = URL_SAFE_NO_PAD
        .decode(&credentials.data_key_ciphertext)
        .expect("decode data-key binding");
    let fingerprint = URL_SAFE_NO_PAD.encode(Sha256::digest(ciphertext));
    let entry = format!(
        "{}::{}::{fingerprint}",
        normalize_api_url(&credentials.api_url),
        credentials.user_id
    );
    let keychain_name = format!(
        "persisted-data-key-{}.bin",
        URL_SAFE_NO_PAD.encode(Sha256::digest(entry.as_bytes()))
    );
    let keychain_path = keychain_dir.join(keychain_name);
    fs::write(&keychain_path, [0x42; 32]).expect("write persisted data key");

    fs::set_permissions(config_dir, fs::Permissions::from_mode(0o700))
        .expect("secure config directory");
    fs::set_permissions(keychain_dir, fs::Permissions::from_mode(0o700))
        .expect("secure keychain directory");
    fs::set_permissions(&credentials_path, fs::Permissions::from_mode(0o600))
        .expect("secure credentials");
    fs::set_permissions(&keychain_path, fs::Permissions::from_mode(0o600))
        .expect("secure persisted data key");
}

fn read_request(stream: &mut TcpStream) -> String {
    stream
        .set_read_timeout(Some(StdDuration::from_secs(5)))
        .expect("set request timeout");
    let mut request = Vec::new();
    let mut buffer = [0_u8; 1_024];
    let header_end = loop {
        if let Some(position) = request.windows(4).position(|window| window == b"\r\n\r\n") {
            break position + 4;
        }
        let read = stream.read(&mut buffer).expect("read request headers");
        assert_ne!(read, 0, "request ended before headers");
        request.extend_from_slice(&buffer[..read]);
    };
    let headers = std::str::from_utf8(&request[..header_end]).expect("UTF-8 headers");
    let content_length = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().expect("content length"))
        })
        .unwrap_or(0);
    while request.len() < header_end + content_length {
        let read = stream.read(&mut buffer).expect("read request body");
        assert_ne!(read, 0, "request body ended early");
        request.extend_from_slice(&buffer[..read]);
    }
    String::from_utf8(request).expect("UTF-8 request")
}

fn write_response(stream: &mut TcpStream, status: &str, body: &str) {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .expect("write response");
}

fn collect_later_requests(listener: &TcpListener) -> Vec<String> {
    listener
        .set_nonblocking(true)
        .expect("make listener nonblocking");
    let deadline = Instant::now() + StdDuration::from_secs(1);
    let mut later_requests = Vec::new();
    while Instant::now() < deadline {
        match listener.accept() {
            Ok((mut stream, _)) => {
                later_requests.push(read_request(&mut stream));
                write_response(
                    &mut stream,
                    "500 Internal Server Error",
                    r#"{"error":"unexpected_request"}"#,
                );
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                thread::sleep(StdDuration::from_millis(10));
            }
            Err(error) => panic!("accept later request: {error}"),
        }
    }
    later_requests
}

fn wait_for_output(mut child: std::process::Child, timeout: StdDuration) -> Output {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if child.try_wait().expect("poll CLI").is_some() {
            return child.wait_with_output().expect("collect CLI output");
        }
        thread::sleep(StdDuration::from_millis(20));
    }
    let _ = child.kill();
    panic!("CLI did not exit within {timeout:?}");
}
