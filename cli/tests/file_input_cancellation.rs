#![cfg(unix)]

use std::ffi::CString;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, Write};
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;
use tempfile::TempDir;

struct RunningCli {
    child: Child,
    stdout: File,
    stderr: File,
}

impl Drop for RunningCli {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn capture(file: &mut File) -> String {
    file.rewind().expect("rewind capture");
    let mut text = String::new();
    file.read_to_string(&mut text).expect("read capture");
    text
}

fn wait_until(mut ready: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !ready() {
        assert!(Instant::now() < deadline, "CLI exceeded input deadline");
        thread::sleep(Duration::from_millis(5));
    }
}

fn start(home: &TempDir, fifo: &Path, body: bool, json: bool, url: &str) -> RunningCli {
    let stdout = tempfile::tempfile().expect("stdout capture");
    let stderr = tempfile::tempfile().expect("stderr capture");
    let mut command = Command::new(env!("CARGO_BIN_EXE_sealtask"));
    command
        .env_clear()
        .env("HOME", home.path())
        .env("SEALTASK_CONFIG_DIR", home.path().join("config"))
        .env("SEALTASK_API_URL", url)
        .env("TERM", "dumb")
        .current_dir(home.path())
        .args(["--non-interactive", if json { "--json" } else { "-v" }])
        .args([
            "tasks",
            "create",
            "--work-list-id",
            "00000000-0000-4000-8000-000000000001",
        ])
        .stdin(Stdio::null())
        .stdout(stdout.try_clone().expect("clone stdout"))
        .stderr(stderr.try_clone().expect("clone stderr"));
    if body {
        command.args(["--title", "Fixture", "--body-file"]);
    } else {
        command.arg("--input-file");
    }
    command.arg(fifo);
    RunningCli {
        child: command.spawn().expect("spawn CLI"),
        stdout,
        stderr,
    }
}

#[test]
fn test_should_cancel_fifo_open_and_idle_reads_without_network_or_eof() {
    for body in [false, true] {
        for attach_writer in [false, true] {
            for signal in [libc::SIGINT, libc::SIGTERM] {
                let home = TempDir::new().expect("fixture");
                let fifo = home.path().join("input.fifo");
                let name = CString::new(fifo.as_os_str().as_bytes()).expect("FIFO path");
                // SAFETY: name is a NUL-terminated path in our private fixture.
                assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
                let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("listener");
                listener
                    .set_nonblocking(true)
                    .expect("nonblocking listener");
                let url = format!("http://{}", listener.local_addr().expect("address"));
                let mut cli = start(&home, &fifo, body, attach_writer, &url);
                let mut writer = None;
                if attach_writer {
                    // A nonblocking writer can attach only once the CLI opens the FIFO.
                    wait_until(|| {
                        assert!(cli.child.try_wait().expect("poll CLI").is_none());
                        match OpenOptions::new()
                            .write(true)
                            .custom_flags(libc::O_NONBLOCK)
                            .open(&fifo)
                        {
                            Ok(file) => {
                                writer = Some(file);
                                true
                            }
                            Err(error) if error.raw_os_error() == Some(libc::ENXIO) => false,
                            Err(error) => panic!("attach FIFO writer: {error}"),
                        }
                    });
                    writer
                        .as_mut()
                        .expect("writer")
                        .write_all(b" ")
                        .expect("probe byte");
                    let monitor = OpenOptions::new()
                        .read(true)
                        .custom_flags(libc::O_NONBLOCK)
                        .open(&fifo)
                        .expect("monitor FIFO");
                    wait_until(|| {
                        assert!(cli.child.try_wait().expect("poll CLI").is_none());
                        let mut queued: libc::c_int = -1;
                        // SAFETY: monitor is live and queued is writable c_int storage.
                        assert_eq!(
                            unsafe {
                                libc::ioctl(monitor.as_raw_fd(), libc::FIONREAD, &mut queued)
                            },
                            0
                        );
                        queued == 0
                    });
                } else {
                    // The start event is emitted inside the supervised operation,
                    // before file open. No writer will ever unblock this FIFO.
                    wait_until(|| {
                        assert!(cli.child.try_wait().expect("poll CLI").is_none());
                        capture(&mut cli.stderr).contains("event=start")
                    });
                }
                // SAFETY: signal targets only our still-live subprocess.
                assert_eq!(unsafe { libc::kill(cli.child.id() as i32, signal) }, 0);
                let mut status = None;
                wait_until(|| {
                    status = cli.child.try_wait().expect("poll exit");
                    status.is_some()
                });
                assert_eq!(status.expect("exit status").code(), Some(130));
                assert!(capture(&mut cli.stdout).is_empty());
                let error = capture(&mut cli.stderr);
                if attach_writer {
                    let error: Value = serde_json::from_str(&error).expect("structured error");
                    assert_eq!(error["error"]["code"], "interrupted");
                    assert_eq!(error["error"]["outcome"], "interrupted");
                } else {
                    assert!(error.contains("not ambiguous"), "{error}");
                }
                assert_eq!(
                    listener.accept().expect_err("no remote connection").kind(),
                    io::ErrorKind::WouldBlock
                );
                drop(writer);
            }
        }
    }
}
