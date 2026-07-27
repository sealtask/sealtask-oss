use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

fn cli(home: &TempDir) -> Command {
    let state_root = home
        .path()
        .canonicalize()
        .expect("canonical temporary home");
    let mut command = Command::cargo_bin("sealtask").expect("sealtask binary");
    command
        .env("HOME", &state_root)
        .env("SEALTASK_CONFIG_DIR", state_root.join("config"))
        .env_remove("SEALTASK_OFFLINE")
        .env_remove("SEALTASK_PAGER")
        .env_remove("PAGER")
        .env_remove("NO_COLOR")
        .env_remove("CLICOLOR")
        .env_remove("CLICOLOR_FORCE");
    command
}

fn json_stderr(output: &std::process::Output) -> Value {
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).lines().count(),
        1,
        "machine errors must remain one compact document"
    );
    serde_json::from_slice(&output.stderr).expect("JSON stderr")
}

#[test]
fn offline_rejects_mutations_and_uncached_surfaces_before_authentication() {
    let home = TempDir::new().expect("temporary home");
    for arguments in [
        vec!["--offline", "--json", "me"],
        vec!["--offline", "--json", "stats"],
        vec!["--offline", "--json", "projects", "audit"],
        vec![
            "--offline",
            "--json",
            "tasks",
            "create",
            "--work-list-id",
            "018f4a76-c9f2-7f38-a09a-2ac748db8ee8",
            "--title",
            "must-not-run",
        ],
    ] {
        let output = cli(&home)
            .args(arguments)
            .output()
            .expect("run offline rejection");
        assert_eq!(output.status.code(), Some(1));
        let error = json_stderr(&output);
        assert_eq!(error["error"]["code"], "validation");
        assert!(
            error["error"]["message"]
                .as_str()
                .is_some_and(|message| message.contains("no network request was attempted"))
        );
    }
}

#[test]
fn offline_rejects_raw_and_stream_protocols_before_authentication() {
    let home = TempDir::new().expect("temporary home");
    for arguments in [
        vec!["--offline", "--json", "projects", "list", "--raw"],
        vec!["--offline", "--format", "jsonl", "tasks", "watch"],
        vec!["--offline", "--format", "jsonl", "activity", "follow"],
    ] {
        let output = cli(&home)
            .args(arguments)
            .output()
            .expect("run offline rejection");
        assert_eq!(output.status.code(), Some(1));
        let error = json_stderr(&output);
        assert_eq!(error["error"]["code"], "validation");
    }
}

#[test]
fn cache_status_and_clear_are_local_and_do_not_require_credentials() {
    let home = TempDir::new().expect("temporary home");
    let status_output = cli(&home)
        .args(["--offline", "--json", "cache", "status"])
        .output()
        .expect("run cache status");
    assert!(status_output.status.success());
    assert!(status_output.stderr.is_empty());
    let status: Value = serde_json::from_slice(&status_output.stdout).expect("JSON cache status");
    assert_eq!(status["schemaVersion"], 1);
    assert_eq!(status["enabled"], true);
    assert_eq!(status["mode"], "offline");
    assert_eq!(status["present"], false);

    let clear_output = cli(&home)
        .args(["--offline", "--json", "cache", "clear"])
        .output()
        .expect("run cache clear");
    assert!(clear_output.status.success());
    assert!(clear_output.stderr.is_empty());
    let clear: Value = serde_json::from_slice(&clear_output.stdout).expect("JSON cache clear");
    assert_eq!(clear["schemaVersion"], 1);
    assert_eq!(clear["cleared"], false);
}

#[test]
fn browse_fails_closed_without_a_private_terminal_before_authentication() {
    let home = TempDir::new().expect("temporary home");
    for arguments in [
        vec!["--json", "browse"],
        vec!["--non-interactive", "browse"],
    ] {
        let output = cli(&home)
            .args(arguments)
            .output()
            .expect("run browse validation");
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8(output.stderr).expect("UTF-8 stderr");
        assert!(stderr.contains("interactive") || stderr.contains("controlling terminal"));
        assert!(!stderr.contains('\u{1b}'));
    }
}

#[test]
fn help_schema_and_info_discover_d8_contracts() {
    let home = TempDir::new().expect("temporary home");
    let help_output = cli(&home).arg("--help").output().expect("run root help");
    assert!(help_output.status.success());
    let help = String::from_utf8(help_output.stdout).expect("UTF-8 help");
    for expected in ["--offline", "browse", "cache"] {
        assert!(help.contains(expected), "root help missing {expected}");
    }

    let schema_output = cli(&home)
        .args(["--json", "schema", "cache", "verify"])
        .output()
        .expect("run cache schema");
    assert!(schema_output.status.success());
    let schema: Value = serde_json::from_slice(&schema_output.stdout).expect("JSON schema");
    assert_eq!(schema["name"], "verify");
    assert!(
        schema["arguments"]
            .as_array()
            .is_some_and(|arguments| arguments.iter().any(|argument| {
                argument["id"] == "password_stdin" && argument["long"] == "password-stdin"
            }))
    );

    let info_output = cli(&home)
        .args(["--json", "info"])
        .output()
        .expect("run info");
    assert!(info_output.status.success());
    let info: Value = serde_json::from_slice(&info_output.stdout).expect("JSON info");
    assert_eq!(info["readCache"]["onlineFallback"], false);
    assert_eq!(info["readCache"]["snapshotAgeReported"], true);
    assert_eq!(info["readCache"]["plaintextSnapshotSidecar"], false);
    assert_eq!(
        info["readCache"]["coordinationSidecar"],
        "opaque_invalidation_generation"
    );
    assert_eq!(info["invocationCache"]["enabledForCliReads"], true);
    assert_eq!(info["browse"]["controllingTerminalOnly"], true);
    assert_eq!(
        info["browse"]["redirectedStandardStreamsReceiveContent"],
        false
    );
}
