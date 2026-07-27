use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::net::TcpListener;
use std::process::Stdio;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use uuid::Uuid;

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
        .env_remove("SEALTASK_API_URL")
        .env_remove("SEALTASK_PROFILE")
        .env_remove("SEALTASK_PAGER")
        .env_remove("PAGER");
    command
}

fn create_operation(operation_id: &str, title: &str) -> String {
    format!(
        r#"{{"schemaVersion":1,"operationId":"{operation_id}","type":"task.create","project":"id:{}","input":{{"title":"{title}"}}}}"#,
        Uuid::from_u128(1)
    )
}

#[test]
fn batch_help_and_info_publish_the_versioned_contract() {
    let local_state = TempDir::new().expect("temporary local state");
    let help = cli(&local_state)
        .args(["batch", "run", "--help"])
        .output()
        .expect("run help");
    assert!(help.status.success());
    let help = String::from_utf8(help.stdout).expect("UTF-8 help");
    for option in [
        "--input",
        "--jobs",
        "--continue-on-error",
        "--checkpoint",
        "--resume",
        "--dry-run",
    ] {
        assert!(help.contains(option), "missing {option}");
    }

    let info = cli(&local_state)
        .args(["--json", "info"])
        .output()
        .expect("run info");
    assert!(info.status.success());
    let info: Value = serde_json::from_slice(&info.stdout).expect("JSON info");
    assert_eq!(info["batch"]["inputSchemaVersion"], 1);
    assert_eq!(info["batch"]["recordSchemaVersion"], 1);
    assert_eq!(info["batch"]["limits"]["maximumJobs"], 16);
    assert_eq!(info["batch"]["exitCodes"]["partialFailure"], 3);
    assert_eq!(info["taskDryRun"]["willMutate"], false);
}

#[test]
fn finite_json_and_invalid_jobs_fail_before_input_or_authentication() {
    let local_state = TempDir::new().expect("temporary local state");
    let finite = cli(&local_state)
        .args([
            "--json",
            "batch",
            "run",
            "--input",
            "missing-secret-input.jsonl",
        ])
        .output()
        .expect("run finite JSON");
    assert_eq!(finite.status.code(), Some(1));
    assert!(finite.stdout.is_empty());
    let finite: Value = serde_json::from_slice(&finite.stderr).expect("JSON error");
    assert_eq!(finite["error"]["code"], "validation");
    assert!(
        finite["error"]["message"]
            .as_str()
            .is_some_and(|message| message.contains("--format jsonl"))
    );

    for jobs in ["0", "17"] {
        let invalid = cli(&local_state)
            .args([
                "batch",
                "run",
                "--input",
                "missing-secret-input.jsonl",
                "--jobs",
                jobs,
            ])
            .output()
            .expect("run invalid jobs");
        assert_eq!(invalid.status.code(), Some(2));
        assert!(invalid.stdout.is_empty());
        assert!(String::from_utf8_lossy(&invalid.stderr).contains("--jobs <COUNT>"));
    }
}

#[test]
fn whole_input_is_validated_before_any_network_request() {
    let local_state = TempDir::new().expect("temporary local state");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    listener.set_nonblocking(true).expect("nonblocking");
    let api_url = format!("http://{}", listener.local_addr().expect("address"));
    let input = local_state.path().join("batch.jsonl");
    fs::write(
        &input,
        format!(
            "{}\n{{\"schemaVersion\":1,\"operationId\":\"late\",\"type\":\"task.create\"}}\n",
            create_operation("first", "plaintext-title-canary")
        ),
    )
    .expect("write input");

    let output = cli(&local_state)
        .args([
            "--format",
            "jsonl",
            "--api-url",
            &api_url,
            "batch",
            "run",
            "--input",
            input.to_str().expect("path"),
        ])
        .output()
        .expect("run batch");
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("UTF-8 stderr");
    assert!(stderr.contains("line 2"));
    assert!(!stderr.contains("plaintext-title-canary"));
    assert!(
        listener.accept().is_err(),
        "invalid full input unexpectedly reached the API"
    );
}

#[test]
fn checkpoint_contract_errors_use_exit_four_and_machine_codes() {
    let local_state = TempDir::new().expect("temporary local state");
    let input = local_state.path().join("batch.jsonl");
    fs::write(&input, format!("{}\n", create_operation("first", "Task"))).expect("write input");

    let missing_path = cli(&local_state)
        .args([
            "--format",
            "jsonl",
            "batch",
            "run",
            "--input",
            input.to_str().expect("path"),
            "--resume",
        ])
        .output()
        .expect("run missing checkpoint path");
    assert_eq!(missing_path.status.code(), Some(4));
    let missing: Value =
        serde_json::from_slice(&missing_path.stderr).expect("JSON missing checkpoint error");
    assert_eq!(missing["error"]["code"], "checkpoint_conflict");

    let alias = cli(&local_state)
        .args([
            "--format",
            "jsonl",
            "batch",
            "run",
            "--input",
            input.to_str().expect("path"),
            "--checkpoint",
            input.to_str().expect("path"),
        ])
        .output()
        .expect("run checkpoint alias");
    assert_eq!(alias.status.code(), Some(4));
    let alias: Value = serde_json::from_slice(&alias.stderr).expect("JSON alias error");
    assert_eq!(alias["error"]["code"], "checkpoint_conflict");
}

#[cfg(unix)]
#[test]
fn sigint_stops_new_batch_work_and_emits_an_interrupted_summary() {
    let local_state = TempDir::new().expect("temporary local state");
    let input = local_state.path().join("large-batch.jsonl");
    let body_canary = "plaintext-body-canary".repeat(40);
    let mut contents = String::with_capacity(10_000 * 1_000);
    for index in 0..10_000 {
        contents.push_str(&format!(
            "{}\n",
            create_operation(&format!("op-{index}"), &body_canary)
        ));
    }
    fs::write(&input, contents).expect("write large input");

    let binary = assert_cmd::cargo::cargo_bin("sealtask");
    let child = std::process::Command::new(binary)
        .current_dir(local_state.path())
        .env("HOME", local_state.path())
        .env(
            "SEALTASK_CONFIG_DIR",
            local_state.path().join("sealtask-config"),
        )
        .env("NO_COLOR", "1")
        .args([
            "--format",
            "jsonl",
            "batch",
            "run",
            "--input",
            input.to_str().expect("path"),
            "--continue-on-error",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn batch");
    std::thread::sleep(Duration::from_millis(75));
    let result = unsafe { libc::kill(child.id().cast_signed(), libc::SIGINT) };
    assert_eq!(result, 0, "send SIGINT");
    let output = child.wait_with_output().expect("wait for batch");
    assert_eq!(
        output.status.code(),
        Some(130),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 stdout");
    let summary: Value =
        serde_json::from_str(stdout.lines().last().expect("summary")).expect("JSON summary");
    assert_eq!(summary["type"], "batch.summary");
    assert_eq!(summary["interrupted"], true);
    assert!(!stdout.contains("plaintext-body-canary"));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("plaintext-body-canary"));
}

#[cfg(unix)]
#[test]
fn sigint_interrupts_batch_waiting_on_open_stdin_with_a_typed_error() {
    let local_state = TempDir::new().expect("temporary local state");
    let binary = assert_cmd::cargo::cargo_bin("sealtask");
    let mut child = std::process::Command::new(binary)
        .current_dir(local_state.path())
        .env("HOME", local_state.path())
        .env(
            "SEALTASK_CONFIG_DIR",
            local_state.path().join("sealtask-config"),
        )
        .env("NO_COLOR", "1")
        .args(["--format", "jsonl", "batch", "run", "--input", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn batch with open stdin");
    let mut stdin = child.stdin.take().expect("batch stdin");

    // A two-MiB partial line cannot fit in a Unix pipe without the child
    // actively draining it. Once this returns, signal installation and the
    // stdin reader are both live, while EOF deliberately remains withheld.
    stdin
        .write_all(&vec![b' '; 2 * 1024 * 1024])
        .expect("fill and drain the open stdin pipe");
    let result = unsafe { libc::kill(child.id().cast_signed(), libc::SIGINT) };
    assert_eq!(result, 0, "send SIGINT");

    let deadline = Instant::now() + Duration::from_secs(5);
    let status = loop {
        if let Some(status) = child.try_wait().expect("poll interrupted batch") {
            break status;
        }
        if Instant::now() >= deadline {
            child.kill().expect("kill stalled batch after timeout");
            drop(stdin);
            let _ = child.wait();
            panic!("batch did not exit while stdin remained open");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    drop(stdin);
    let output = child.wait_with_output().expect("collect interrupted batch");

    assert_eq!(status.code(), Some(130));
    assert_eq!(output.status.code(), Some(130));
    assert!(
        output.stdout.is_empty(),
        "stdout must remain a JSONL record stream"
    );
    let error: Value = serde_json::from_slice(&output.stderr).expect("typed JSON error");
    assert_eq!(error["error"]["code"], "interrupted");
    assert_eq!(error["error"]["outcome"], "interrupted");
    assert!(
        error["error"]["message"]
            .as_str()
            .is_some_and(|message| message.contains("stdin"))
    );
}
