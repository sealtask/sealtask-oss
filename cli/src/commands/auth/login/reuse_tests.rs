#![cfg(unix)]

use super::reused_session_is_unlocked;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD_NO_PAD;
use chrono::{Duration, Utc};
use sealtask_client_auth::{
    Credentials, RefreshResponse, clear_credentials, save_credentials,
    update_credentials_with_refresh,
};
use sealtask_client_core::PublicError;
use sealtask_client_crypto::SymmetricKey;
use sealtask_client_runtime::{RuntimeClient, lock, serve, session_key, socket_path, unlock};
use std::time::Duration as StdDuration;
use uuid::Uuid;

const WORKER_ENV: &str = "SEALTASK_LOGIN_REUSE_TEST_WORKER";
const TEST_NAME: &str = "commands::auth::login::reuse_tests::reused_login_checks_the_expected_account_under_the_credential_lock";

struct DaemonCleanup;

impl Drop for DaemonCleanup {
    fn drop(&mut self) {
        let _ = lock();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reused_login_checks_the_expected_account_under_the_credential_lock() {
    if std::env::var_os(WORKER_ENV).is_none() {
        let home = tempfile::tempdir().expect("isolated home");
        let output = assert_cmd::Command::new(std::env::current_exe().expect("test executable"))
            .args(["--exact", TEST_NAME, "--nocapture"])
            .env_clear()
            .env("HOME", home.path())
            .env("SEALTASK_CONFIG_DIR", home.path().join(".sealtask"))
            .env(WORKER_ENV, "1")
            .timeout(StdDuration::from_secs(20))
            .output()
            .expect("run isolated login reuse test");
        assert!(
            output.status.success(),
            "login reuse worker failed:\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }

    let account_a = credentials("a@example.test");
    let account_b = credentials("b@example.test");
    let runtime = RuntimeClient::new(&account_a.api_url).expect("runtime");
    save_credentials(&account_a).expect("initial credentials");
    let socket = socket_path().expect("daemon socket");
    let daemon_socket = socket.clone();
    let daemon = tokio::spawn(async move { serve(&daemon_socket).await });
    let _cleanup = DaemonCleanup;
    tokio::time::timeout(StdDuration::from_secs(5), async {
        while std::os::unix::net::UnixStream::connect(&socket).is_err() {
            tokio::time::sleep(StdDuration::from_millis(5)).await;
        }
    })
    .await
    .expect("daemon starts");

    let key_b = session_key(
        &account_b.api_url,
        account_b.user_id,
        &account_b.data_key_ciphertext,
    )
    .expect("account B session key");
    unlock(&key_b, &SymmetricKey::new([0x2b; 32]), 300).expect("unlock B");
    save_credentials(&account_b).expect("account switches after A was refreshed");
    let status = runtime.unlock_status().expect("current account status");
    assert!(
        status.unlocked,
        "B must be unlocked to reproduce false success"
    );
    assert_eq!(status.session_key, Some(key_b));
    assert!(matches!(
        reused_session_is_unlocked(&runtime, &account_a),
        Err(PublicError::Conflict(_))
    ));

    clear_credentials().expect("concurrent logout");
    assert!(runtime.unlock_status().expect("aggregate status").unlocked);
    assert!(matches!(
        reused_session_is_unlocked(&runtime, &account_a),
        Err(PublicError::Conflict(_))
    ));

    let mut rotated_a = account_a.clone();
    update_credentials_with_refresh(
        &mut rotated_a,
        RefreshResponse {
            access_token: "rotated-access".to_string(),
            refresh_token: "rotated-refresh".to_string(),
            expires_in: 900,
            refresh_expires_in: 2_592_000,
            token_type: "Bearer".to_string(),
        },
    );
    save_credentials(&rotated_a).expect("rotate A without changing its identity");
    assert!(
        !reused_session_is_unlocked(&runtime, &account_a).expect("rotation remains valid"),
        "B's unlocked key must not satisfy A's still-locked session"
    );
    let key_a = session_key(
        &account_a.api_url,
        account_a.user_id,
        &account_a.data_key_ciphertext,
    )
    .expect("account A session key");
    unlock(&key_a, &SymmetricKey::new([0x2a; 32]), 300).expect("unlock A");
    assert!(reused_session_is_unlocked(&runtime, &account_a).expect("A remains unlocked"));

    lock().expect("stop daemon");
    tokio::time::timeout(StdDuration::from_secs(5), daemon)
        .await
        .expect("daemon stops")
        .expect("daemon task joins")
        .expect("daemon exits successfully");
}

fn credentials(email: &str) -> Credentials {
    Credentials {
        api_url: "http://127.0.0.1:9".to_string(),
        access_token: "initial-access".to_string(),
        refresh_token: format!("initial-refresh-{email}"),
        access_expires_at: Utc::now() + Duration::minutes(15),
        refresh_expires_at: Utc::now() + Duration::days(30),
        user_id: Uuid::now_v7(),
        email: email.to_string(),
        data_key_ciphertext: STANDARD_NO_PAD.encode([0x17; 32]),
        local_session_id: Some(Uuid::now_v7().to_string()),
    }
}
