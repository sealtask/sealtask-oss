#![cfg(unix)]

#[path = "support/idle_stdin.rs"]
mod idle_stdin;

use std::fs::File;
use std::io::{self, Read, Seek};
use std::os::fd::FromRawFd;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;
use tempfile::TempDir;

fn cli(home: &TempDir) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sealtask"));
    command
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", home.path())
        .env("SEALTASK_CONFIG_DIR", home.path().join("config"))
        .env("SEALTASK_API_URL", "http://127.0.0.1:1")
        .env("TERM", "xterm-256color")
        .stdin(Stdio::null());
    command
}

fn pty() -> (File, File) {
    let mut master = -1;
    let mut slave = -1;
    let mut size = libc::winsize {
        ws_row: 4,
        ws_col: 40,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    // SAFETY: openpty initializes both descriptors; optional name/termios are null.
    let result = unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &raw mut size,
        )
    };
    assert_eq!(result, 0, "openpty: {}", io::Error::last_os_error());
    // SAFETY: these descriptors were just allocated and each File owns one.
    let (master, slave) = unsafe { (File::from_raw_fd(master), File::from_raw_fd(slave)) };
    (master, slave)
}

fn parser_stderr_pty(command: Command) -> (i32, String) {
    let (code, captured, _) = capture_pty(command, false);
    (code, captured)
}

fn capture_pty(mut command: Command, stdout: bool) -> (i32, String, Vec<u8>) {
    use std::os::fd::AsRawFd;
    let (mut master, slave) = pty();
    // SAFETY: master is a live descriptor; no flags other than O_NONBLOCK change.
    assert_eq!(
        unsafe { libc::fcntl(master.as_raw_fd(), libc::F_SETFL, libc::O_NONBLOCK) },
        0
    );
    let mut stderr_capture = tempfile::tempfile().expect("stderr capture");
    if stdout {
        command
            .stdout(slave)
            .stderr(stderr_capture.try_clone().expect("clone capture"));
    } else {
        command.stdout(Stdio::null()).stderr(slave);
    }
    let mut child = command.spawn().expect("spawn CLI");
    drop(command);
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut captured = Vec::new();
    let mut buffer = [0; 8192];
    let status = loop {
        match master.read(&mut buffer) {
            Ok(count) => captured.extend_from_slice(&buffer[..count]),
            Err(error)
                if error.kind() == io::ErrorKind::WouldBlock
                    || error.raw_os_error() == Some(libc::EIO) => {}
            Err(error) => panic!("PTY read: {error}"),
        }
        if let Some(status) = child.try_wait().expect("poll CLI") {
            // Drain bytes already queued after process exit.
            while let Ok(count) = master.read(&mut buffer) {
                if count == 0 {
                    break;
                }
                captured.extend_from_slice(&buffer[..count]);
            }
            break status;
        }
        if Instant::now() >= deadline {
            child.kill().expect("kill stalled CLI");
            child.wait().expect("reap CLI");
            panic!("CLI exceeded PTY deadline");
        }
        thread::sleep(Duration::from_millis(5));
    };
    let mut stderr = Vec::new();
    stderr_capture.rewind().expect("rewind stderr");
    stderr_capture
        .read_to_end(&mut stderr)
        .expect("read stderr");
    (
        status.code().expect("exit code"),
        String::from_utf8(captured)
            .expect("UTF-8")
            .replace("\r\n", "\n"),
        stderr,
    )
}

#[test]
fn test_should_escape_hostile_parser_values_on_actual_stderr_ptys() {
    let home = TempDir::new().expect("fixture");
    for color in ["auto", "always", "never"] {
        for payload in [
            "--oops\x1b]0;AUDIT\x07",
            "--oops\rFORGED",
            "--oops\nFORGED",
            "--oops\u{202e}exe.txt",
            "--oops\u{009b}2J",
        ] {
            let mut command = cli(&home);
            command.args(["--color", color, payload]);
            let (code, stderr) = parser_stderr_pty(command);
            assert_eq!(code, 2);
            assert_eq!(stderr.lines().count(), 1, "{color}: {stderr:?}");
            assert!(
                !stderr
                    .chars()
                    .any(|ch| (ch.is_control() && ch != '\n') || ch == '\u{202e}'),
                "{color}: {stderr:?}"
            );
            assert!(stderr.contains("unexpected argument"));
        }
    }
}

#[test]
fn test_should_preserve_parser_source_values_in_machine_diagnostics() {
    let home = TempDir::new().expect("fixture");
    let payload = "--oops\x1b]0;AUDIT\x07\n\u{202e}";
    for format in ["json", "json-pretty", "jsonl"] {
        let output = cli(&home)
            .args(["--format", format, payload])
            .output()
            .expect("CLI");
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.contains(&0x1b));
        let result: Value = serde_json::from_slice(&output.stderr).expect("JSON error");
        let message = result["error"]["message"].as_str().expect("message");
        assert!(message.contains(payload));
        assert!(!message.replace(payload, "").contains('\x1b'));
        let output = cli(&home)
            .args([
                "--format",
                format,
                "auth",
                "unlock",
                "--ttl-seconds",
                "nope",
            ])
            .output()
            .expect("nested CLI");
        assert_eq!(output.status.code(), Some(2));
        let result: Value = serde_json::from_slice(&output.stderr).expect("nested JSON error");
        let message = result["error"]["message"].as_str().expect("message");
        assert!(message.starts_with("error: "));
        assert!(!message.contains('\x1b'));
    }
}

#[test]
fn test_should_keep_ordinary_diagnostics_and_help_structured_without_env_values() {
    let home = TempDir::new().expect("fixture");
    let output = cli(&home)
        .args(["--color", "never", "--bogus"])
        .output()
        .expect("CLI");
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8(output.stderr)
            .expect("UTF-8")
            .contains("\n\nUsage:")
    );
    let output = cli(&home)
        .env("SEALTASK_API_URL", "\x1b]0;SECRET\x07\nFORGED")
        .arg("--help")
        .output()
        .expect("CLI");
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).expect("UTF-8");
    assert!(help.contains("SEALTASK_API_URL"));
    assert!(!help.contains("SECRET"));
    assert!(!help.contains("FORGED"));
    assert!(help.lines().count() > 10);
}

#[test]
fn test_should_fail_when_help_or_version_cannot_be_delivered() {
    let home = TempDir::new().expect("fixture");
    for args in [
        vec!["--help"],
        vec!["--version"],
        vec!["help", "tasks"],
        vec!["--json", "--help"],
    ] {
        let path = home.path().join("output");
        let output_file = File::create(&path).expect("output file");
        let mut command = cli(&home);
        command
            .args(&args)
            .stdout(output_file)
            .stderr(Stdio::piped());
        // SAFETY: only async-signal-safe libc calls run between fork and exec.
        unsafe {
            command.pre_exec(|| {
                libc::signal(libc::SIGXFSZ, libc::SIG_IGN);
                let limit = libc::rlimit {
                    rlim_cur: 0,
                    rlim_max: 0,
                };
                if libc::setrlimit(libc::RLIMIT_FSIZE, &limit) != 0 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let output = command.output().expect("CLI");
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert_eq!(std::fs::metadata(path).expect("metadata").len(), 0);
        assert!(String::from_utf8_lossy(&output.stderr).contains("stdout"));
        if args.contains(&"--json") {
            serde_json::from_slice::<Value>(&output.stderr).expect("JSON failure");
        }
    }
}

#[test]
fn test_should_keep_closed_help_stdout_pipe_as_success() {
    let home = TempDir::new().expect("fixture");
    let mut descriptors = [-1; 2];
    // SAFETY: pipe initializes two descriptors; the reader is closed immediately.
    assert_eq!(unsafe { libc::pipe(descriptors.as_mut_ptr()) }, 0);
    let writer = unsafe {
        libc::close(descriptors[0]);
        File::from_raw_fd(descriptors[1])
    };
    let output = cli(&home)
        .arg("--help")
        .stdout(writer)
        .stderr(Stdio::piped())
        .output()
        .expect("CLI");
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
}

#[test]
fn test_should_escape_invalid_environment_values_on_stderr_ptys() {
    let home = TempDir::new().expect("fixture");
    for (key, value) in [
        ("SEALTASK_COLOR", "always"),
        ("SEALTASK_COLOR", "never"),
        ("NO_COLOR", "1"),
    ] {
        let mut command = cli(&home);
        command
            .env(key, value)
            .env("SEALTASK_RETRY", "bad\x1b]0;TITLE\x07\r\nFORGED\u{202e}")
            .arg("info");
        let (code, stderr) = parser_stderr_pty(command);
        assert_eq!(code, 2);
        assert_eq!(stderr.lines().count(), 1);
        assert!(
            !stderr
                .chars()
                .any(|ch| (ch.is_control() && ch != '\n') || ch == '\u{202e}')
        );
    }
}

#[test]
fn test_should_fail_when_parser_stderr_pipe_is_closed() {
    let home = TempDir::new().expect("fixture");
    for args in [vec!["--bogus"], vec!["--json", "--bogus"]] {
        let mut descriptors = [-1; 2];
        // SAFETY: pipe initializes both descriptors; each is closed once.
        assert_eq!(unsafe { libc::pipe(descriptors.as_mut_ptr()) }, 0);
        let writer = unsafe {
            libc::close(descriptors[0]);
            File::from_raw_fd(descriptors[1])
        };
        let output = cli(&home).args(args).stderr(writer).output().expect("CLI");
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
    }
}

fn wait_bounded(child: &mut std::process::Child) -> std::process::ExitStatus {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = child.try_wait().expect("poll CLI") {
            return status;
        }
        if Instant::now() >= deadline {
            child.kill().expect("kill stalled CLI");
            child.wait().expect("reap CLI");
            panic!("CLI did not exit while stdin remained open");
        }
        thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn test_should_cancel_idle_mutation_stdin_without_eof_or_network() {
    let home = TempDir::new().expect("fixture");
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("listener");
    listener.set_nonblocking(true).expect("nonblocking");
    let url = format!("http://{}", listener.local_addr().expect("address"));
    let id = "00000000-0000-4000-8000-000000000001";
    let code_path = home.path().join("enrollment-code");
    std::fs::write(&code_path, "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA")
        .expect("fixture code");
    let code_file = code_path.to_str().expect("path");
    for input in [
        vec!["tasks", "create", "--input-stdin"],
        vec!["tasks", "create", "--title", "test", "--body-file", "-"],
        vec!["tasks", "update", "--task-id", id, "--input-stdin"],
        vec!["tasks", "delete", "--task-id", id, "--yes", "--input-stdin"],
        vec!["notes", "create", "--input-stdin"],
        vec!["notes", "update", "--note-id", id, "--input-stdin"],
        vec!["notes", "delete", "--note-id", id, "--yes", "--input-stdin"],
        vec!["comments", "create", "--task-id", id, "--input-stdin"],
        vec![
            "comments",
            "update",
            "--task-id",
            id,
            "--comment-id",
            id,
            "--input-stdin",
        ],
        vec![
            "comments",
            "delete",
            "--task-id",
            id,
            "--comment-id",
            id,
            "--yes",
            "--input-stdin",
        ],
        vec![
            "agents",
            "approve",
            "--enrollment-code-file",
            "-",
            "--instructions-file",
            code_file,
            "--fingerprint",
            "fixture",
            "--handle",
            "test",
        ],
        vec![
            "agents",
            "approve",
            "--enrollment-code-file",
            code_file,
            "--instructions-file",
            "-",
            "--fingerprint",
            "fixture",
            "--handle",
            "test",
        ],
    ] {
        for signals in [
            vec![libc::SIGINT],
            vec![libc::SIGTERM],
            vec![libc::SIGINT, libc::SIGTERM],
        ] {
            let stdin = idle_stdin::IdleStdin::new();
            let mut child = cli(&home)
                .env("SEALTASK_API_URL", &url)
                .args(["--json", "--non-interactive"])
                .args(&input)
                .args(["--work-list-id", id])
                .stdin(stdin.stdio())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("CLI");
            stdin.wait_until_read(&mut child);
            for signal in signals {
                // SAFETY: child is our live subprocess; kill retains no memory.
                assert_eq!(unsafe { libc::kill(child.id() as i32, signal) }, 0);
            }
            assert_eq!(wait_bounded(&mut child).code(), Some(130));
            let output = child.wait_with_output().expect("output");
            assert!(output.stdout.is_empty());
            let error: Value = serde_json::from_slice(&output.stderr).expect("JSON interruption");
            assert_eq!(error["error"]["code"], "interrupted");
            assert!(
                error["error"]["message"]
                    .as_str()
                    .expect("message")
                    .contains("not ambiguous")
            );
            assert_eq!(
                listener.accept().expect_err("no network request").kind(),
                io::ErrorKind::WouldBlock
            );
        }
    }
}

#[test]
fn test_should_cancel_pending_confirmation_without_holding_stderr() {
    use std::os::fd::AsRawFd;
    let home = TempDir::new().expect("fixture");
    let (mut master, slave) = pty();
    // Keep the PTY queue alive until the final diagnostic is drained on macOS.
    let _keep_slave_open = slave.try_clone().expect("keep slave");
    // SAFETY: master is a live descriptor.
    assert_eq!(
        unsafe { libc::fcntl(master.as_raw_fd(), libc::F_SETFL, libc::O_NONBLOCK) },
        0
    );
    let id = "00000000-0000-4000-8000-000000000001";
    let mut child = cli(&home)
        .args([
            "--color",
            "never",
            "tasks",
            "delete",
            "--work-list-id",
            id,
            "--task-id",
            id,
        ])
        .stdin(slave.try_clone().expect("clone slave"))
        .stderr(slave)
        .stdout(Stdio::piped())
        .spawn()
        .expect("CLI");
    let mut bytes = Vec::new();
    let mut buffer = [0; 4096];
    let deadline = Instant::now() + Duration::from_secs(5);
    while !String::from_utf8_lossy(&bytes).contains("[y/N]") {
        if let Ok(count) = master.read(&mut buffer) {
            bytes.extend_from_slice(&buffer[..count]);
        }
        if Instant::now() >= deadline || child.try_wait().expect("poll").is_some() {
            let _ = child.kill();
            let _ = child.wait();
            panic!(
                "confirmation not reached: {:?}",
                String::from_utf8_lossy(&bytes)
            );
        }
        thread::sleep(Duration::from_millis(5));
    }
    // SAFETY: child is our live subprocess.
    assert_eq!(unsafe { libc::kill(child.id() as i32, libc::SIGINT) }, 0);
    assert_eq!(wait_bounded(&mut child).code(), Some(130));
    while let Ok(count) = master.read(&mut buffer) {
        if count == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..count]);
    }
    let text = String::from_utf8_lossy(&bytes);
    assert!(
        text.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .contains("not ambiguous"),
        "{text:?}"
    );
}

#[test]
fn test_should_never_launch_a_pager_non_interactively_on_stdout_pty() {
    use std::os::unix::fs::PermissionsExt;
    let home = TempDir::new().expect("fixture");
    let pager = home.path().join("pager with 'quotes'");
    let marker = home.path().join("pager-launched");
    std::fs::write(&pager, "#!/bin/sh\n: > \"$PAGER_MARKER\"\nexec /bin/cat\n")
        .expect("pager fixture");
    std::fs::set_permissions(&pager, std::fs::Permissions::from_mode(0o700))
        .expect("executable pager");
    for (args, mode, expected_code, launches) in [
        (vec!["--non-interactive", "schema"], "auto", 0, false),
        (vec!["--non-interactive", "schema"], "always", 0, false),
        (
            vec!["--non-interactive", "--pager", "auto", "schema"],
            "always",
            0,
            false,
        ),
        (
            vec!["--non-interactive", "--pager", "always", "schema"],
            "auto",
            1,
            false,
        ),
        (
            vec![
                "--non-interactive",
                "--pager",
                "always",
                "--no-pager",
                "schema",
            ],
            "auto",
            0,
            false,
        ),
        (vec!["schema"], "auto", 0, true),
        (vec!["--pager", "always", "schema"], "auto", 0, true),
    ] {
        let _ = std::fs::remove_file(&marker);
        let mut command = cli(&home);
        let quoted_pager =
            shlex::try_quote(pager.to_str().expect("pager path")).expect("quote pager");
        command
            .env("SEALTASK_PAGER", quoted_pager.as_ref())
            .env("PAGER_MARKER", &marker)
            .env("SEALTASK_PAGER_MODE", mode)
            .args(args);
        let (code, stdout, stderr) = capture_pty(command, true);
        assert_eq!(code, expected_code, "{}", String::from_utf8_lossy(&stderr));
        assert_eq!(marker.exists(), launches);
        if expected_code == 0 {
            assert!(stdout.contains("Usage: sealtask"));
            assert!(stderr.is_empty());
        } else {
            assert!(stdout.is_empty());
            assert!(String::from_utf8_lossy(&stderr).contains("--non-interactive"));
        }
    }
}

#[test]
fn test_should_explain_pager_conflict_precedence_and_publish_policy() {
    let home = TempDir::new().expect("fixture");
    for args in [vec!["schema"], vec!["activity", "follow"]] {
        let output = cli(&home)
            .args(["--non-interactive", "--pager", "always"])
            .args(args)
            .output()
            .expect("CLI");
        assert_eq!(output.status.code(), Some(1));
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("--non-interactive cannot be combined")
        );
    }
    let output = cli(&home)
        .args(["--json", "--non-interactive", "--pager", "always", "schema"])
        .output()
        .expect("CLI");
    assert_eq!(output.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&output.stderr).expect("JSON error");
    assert!(
        error["error"]["message"]
            .as_str()
            .expect("message")
            .contains("JSON output")
    );
    let output = cli(&home).args(["--json", "info"]).output().expect("info");
    assert!(output.status.success());
    let info: Value = serde_json::from_slice(&output.stdout).expect("JSON info");
    assert_eq!(
        info["terminalPolicies"]["nonInteractivePaging"]["explicitAlways"],
        "validation_error"
    );
    assert_eq!(
        info["terminalPolicies"]["nonInteractivePaging"]["automatic"],
        "disabled"
    );
    let output = cli(&home).arg("--help").output().expect("help");
    assert!(String::from_utf8_lossy(&output.stdout).contains("Never prompt or page"));
}
