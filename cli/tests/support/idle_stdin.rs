use std::fs::File;
use std::io::{self, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::process::{Child, Stdio};
use std::time::{Duration, Instant};

/// Holds stdin open after a probe byte, without relying on a startup delay.
pub struct IdleStdin {
    reader: File,
    _writer: File,
}

impl IdleStdin {
    pub fn new() -> Self {
        let mut descriptors = [-1; 2];
        // SAFETY: pipe initializes both descriptors, each owned once below.
        assert_eq!(unsafe { libc::pipe(descriptors.as_mut_ptr()) }, 0);
        let (reader, mut writer) = unsafe {
            (
                File::from_raw_fd(descriptors[0]),
                File::from_raw_fd(descriptors[1]),
            )
        };
        for descriptor in &descriptors {
            // SAFETY: both descriptors are live; child receives only its stdin dup.
            assert_eq!(
                unsafe { libc::fcntl(*descriptor, libc::F_SETFD, libc::FD_CLOEXEC) },
                0
            );
        }
        writer.write_all(b" ").expect("queue stdin probe byte");
        Self {
            reader,
            _writer: writer,
        }
    }

    pub fn stdio(&self) -> Stdio {
        self.reader.try_clone().expect("clone stdin reader").into()
    }

    pub fn wait_until_read(&self, child: &mut Child) {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let mut queued: libc::c_int = -1;
            // SAFETY: reader is live and queued points to writable c_int storage.
            let result =
                unsafe { libc::ioctl(self.reader.as_raw_fd(), libc::FIONREAD, &mut queued) };
            if result != 0 || Instant::now() >= deadline {
                let error = io::Error::last_os_error();
                let _ = child.kill();
                let _ = child.wait();
                panic!("stdin reader did not consume probe byte: ioctl={result}, error={error}");
            }
            assert!(
                child.try_wait().expect("poll CLI").is_none(),
                "CLI exited before reaching idle stdin"
            );
            if queued == 0 {
                return;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}
