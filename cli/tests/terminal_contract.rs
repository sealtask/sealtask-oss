#![cfg(unix)]

use std::fs::File;
use std::io::{self, Read};
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
            &mut size,
        )
    };
    assert_eq!(result, 0, "openpty: {}", io::Error::last_os_error());
    // SAFETY: these descriptors were just allocated and each File owns one.
    let (master, slave) = unsafe { (File::from_raw_fd(master), File::from_raw_fd(slave)) };
    (master, slave)
}

fn parser_stderr_pty(mut command: Command) -> (i32, String) {
    use std::os::fd::AsRawFd;
    let (mut master, slave) = pty();
    // SAFETY: master is a live descriptor; no flags other than O_NONBLOCK change.
    assert_eq!(
        unsafe { libc::fcntl(master.as_raw_fd(), libc::F_SETFL, libc::O_NONBLOCK) },
        0
    );
    let mut child = command
        .stdout(Stdio::null())
        .stderr(slave)
        .spawn()
        .expect("spawn CLI");
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
    (
        status.code().expect("exit code"),
        String::from_utf8(captured)
            .expect("UTF-8")
            .replace("\r\n", "\n"),
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
