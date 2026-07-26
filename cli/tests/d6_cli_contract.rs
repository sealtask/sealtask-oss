use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;
use uuid::Uuid;

fn cli(home: &TempDir) -> Command {
    let mut command = Command::cargo_bin("sealtask").expect("sealtask binary");
    command
        .env("HOME", home.path())
        .env("SEALTASK_CONFIG_DIR", home.path().join("config"))
        .env_remove("SEALTASK_PAGER")
        .env_remove("PAGER")
        .env_remove("NO_COLOR")
        .env_remove("CLICOLOR")
        .env_remove("CLICOLOR_FORCE");
    command
}

#[test]
fn finite_json_is_rejected_for_streams_with_jsonl_guidance() {
    let home = TempDir::new().expect("temporary home");
    let output = cli(&home)
        .args([
            "--json",
            "tasks",
            "watch",
            "--work-list-id",
            &Uuid::now_v7().to_string(),
        ])
        .output()
        .expect("run CLI");

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("UTF-8 stderr");
    assert_eq!(stderr.lines().count(), 1, "machine error must be one line");
    let error: Value = serde_json::from_str(&stderr).expect("JSON stderr");
    assert_eq!(error["error"]["code"], "validation");
    assert!(error["error"]["message"].as_str().is_some_and(|message| {
        message.contains("--format jsonl") && message.contains("finite JSON")
    }));
}

#[test]
fn streaming_commands_reject_forced_paging_before_authentication() {
    let home = TempDir::new().expect("temporary home");
    let output = cli(&home)
        .args(["--pager", "always", "activity", "follow"])
        .output()
        .expect("run CLI");

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("UTF-8 stderr");
    assert!(stderr.contains("paging is unavailable"));
    assert!(!stderr.contains('\u{1b}'));
}

#[test]
fn jsonl_is_compact_for_finite_discovery_and_advertises_stream_contracts() {
    let home = TempDir::new().expect("temporary home");
    let output = cli(&home)
        .args(["--format", "jsonl", "info"])
        .output()
        .expect("run CLI");

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 stdout");
    assert_eq!(stdout.lines().count(), 1);
    let info: Value = serde_json::from_str(&stdout).expect("JSON info");
    assert!(
        info["outputFormats"]
            .as_array()
            .is_some_and(|formats| formats.iter().any(|format| format == "jsonl"))
    );
    assert_eq!(info["streaming"]["machineFormat"], "jsonl");
    assert_eq!(info["streaming"]["interruptionExitCode"], 130);
    assert_eq!(info["audit"]["encryptedPayloadExcluded"], true);
}

#[test]
fn schema_and_help_discover_d6_commands_and_arguments() {
    let home = TempDir::new().expect("temporary home");
    let schema_output = cli(&home)
        .args(["--json", "schema", "tasks", "watch"])
        .output()
        .expect("run schema");
    assert!(schema_output.status.success());
    let schema: Value = serde_json::from_slice(&schema_output.stdout).expect("JSON command schema");
    assert_eq!(schema["name"], "watch");
    let argument_ids = schema["arguments"]
        .as_array()
        .expect("arguments")
        .iter()
        .filter_map(|argument| argument["id"].as_str())
        .collect::<Vec<_>>();
    for expected in [
        "project",
        "work_list_id",
        "include_completed",
        "include_archived",
    ] {
        assert!(argument_ids.contains(&expected), "missing {expected}");
    }

    let help_output = cli(&home)
        .args(["activity", "follow", "--help"])
        .output()
        .expect("run help");
    assert!(help_output.status.success());
    let help = String::from_utf8(help_output.stdout).expect("UTF-8 help");
    assert!(help.contains("--since"));
    assert!(help.contains("--interval"));
    assert!(help.contains("--format jsonl"));
}

#[test]
fn jsonl_errors_are_compact_and_never_leak_terminal_sequences() {
    let home = TempDir::new().expect("temporary home");
    let output = cli(&home)
        .args([
            "--format", "jsonl", "--pager", "always", "activity", "follow",
        ])
        .output()
        .expect("run CLI");

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("UTF-8 stderr");
    assert_eq!(stderr.lines().count(), 1);
    assert!(!stderr.contains('\u{1b}'));
    let error: Value = serde_json::from_str(&stderr).expect("JSON stderr");
    assert_eq!(error["error"]["code"], "validation");
}
