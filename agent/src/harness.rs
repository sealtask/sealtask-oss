use std::{
    collections::VecDeque,
    env,
    future::Future,
    io,
    path::{Path, PathBuf},
    pin::Pin,
    process::{ExitStatus, Stdio},
};

#[cfg(unix)]
use std::collections::{HashMap, HashSet};
#[cfg(windows)]
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
#[cfg(target_os = "linux")]
use std::sync::OnceLock;

use tokio::{
    fs,
    io::{AsyncRead, AsyncReadExt as _, AsyncWriteExt as _},
    process::{Child, Command},
};

use sealtask_client_core::{PublicError, PublicResult};

const MAX_HARNESS_STREAM_BYTES: usize = 128 * 1024;
const MAX_HARNESS_DIAGNOSTIC_BYTES: usize = 8 * 1024;
const MAX_FINAL_MESSAGE_BYTES: u64 = 128 * 1024;
const CODEX_SUBPROCESS_ENV_ALLOWLIST: &str =
    r#"shell_environment_policy.include_only=["PATH","TMPDIR","LANG","LC_ALL"]"#;

#[derive(Debug)]
pub(crate) struct HarnessOutput {
    pub(crate) succeeded: bool,
    pub(crate) summary: String,
    pub(crate) failure_code: Option<String>,
}

pub(crate) trait Harness: Send + Sync {
    fn run<'a>(
        &'a self,
        worktree: &'a Path,
        run_directory: &'a Path,
        prompt: &'a str,
    ) -> Pin<Box<dyn Future<Output = PublicResult<HarnessOutput>> + Send + 'a>>;
}

#[derive(Debug)]
pub(crate) struct CodexHarness {
    executable: PathBuf,
}

struct PlaintextOutputGuard {
    path: PathBuf,
    armed: bool,
}

#[cfg(unix)]
/// Owns a process group while its leader remains live or waitable. Runtime
/// callers must use [`wait_for_process_tree`] instead of reaping the child
/// directly so the numeric PID/PGID cannot be reassigned before termination.
pub(crate) struct ProcessTreeGuard {
    root_process_id: i32,
    process_group_id: Option<i32>,
    owner_process_id: i32,
    baseline_children: HashSet<i32>,
}

#[cfg(windows)]
pub(crate) struct ProcessTreeGuard {
    job: Option<OwnedHandle>,
}

#[cfg(unix)]
pub(crate) struct ProcessTreeConfiguration {
    owner_process_id: i32,
    baseline_children: HashSet<i32>,
}

#[cfg(windows)]
pub(crate) struct ProcessTreeConfiguration;

impl PlaintextOutputGuard {
    fn new(path: PathBuf) -> Self {
        Self { path, armed: true }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for PlaintextOutputGuard {
    fn drop(&mut self) {
        if self.armed {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

#[cfg(unix)]
impl ProcessTreeGuard {
    pub(crate) fn new(
        child: &tokio::process::Child,
        configuration: ProcessTreeConfiguration,
    ) -> PublicResult<Self> {
        let process_id = child
            .id()
            .ok_or_else(|| PublicError::unexpected("Codex process ID was unavailable"))?;
        let process_group_id = i32::try_from(process_id)
            .map_err(|_| PublicError::unexpected("Codex process ID exceeded platform limits"))?;
        Ok(Self {
            root_process_id: process_group_id,
            process_group_id: Some(process_group_id),
            owner_process_id: configuration.owner_process_id,
            baseline_children: configuration.baseline_children,
        })
    }

    fn terminate(&mut self) {
        if let Some(process_group_id) = self.process_group_id.take() {
            let mut owned = HashSet::new();
            for _ in 0..8 {
                let Ok(processes) = process_parent_map() else {
                    break;
                };
                let discovered = owned_processes(
                    &processes,
                    self.root_process_id,
                    self.owner_process_id,
                    &self.baseline_children,
                );
                let previous_count = owned.len();
                for process_id in discovered {
                    if process_id != self.owner_process_id
                        && unsafe { libc::kill(process_id, libc::SIGSTOP) } == 0
                    {
                        owned.insert(process_id);
                    }
                }
                if owned.len() == previous_count {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(2));
            }

            // This guard owns the dedicated process group from spawn until
            // this one-shot cleanup. The contained wait leaves an exited
            // leader waitable, and cancellation drops this guard before the
            // Child, so the kernel cannot reuse the PID/PGID before this kill.
            // Killing the group still reaches surviving members that have
            // been reparented and can no longer be rediscovered by walking the
            // owner's descendants. Individually killing the successfully
            // frozen ownership snapshot also reaches descendants that called
            // setsid(2) or changed process groups. Linux subreaper mode keeps
            // daemonized descendants parented to this service until cleanup.
            unsafe {
                libc::kill(-process_group_id, libc::SIGKILL);
            }
            for process_id in &owned {
                unsafe {
                    libc::kill(*process_id, libc::SIGKILL);
                }
            }
            #[cfg(target_os = "linux")]
            reap_subreaper_descendants(&owned, self.root_process_id);
        }
    }
}

#[cfg(unix)]
impl Drop for ProcessTreeGuard {
    fn drop(&mut self) {
        self.terminate();
    }
}

#[cfg(windows)]
impl ProcessTreeGuard {
    pub(crate) fn new(
        child: &tokio::process::Child,
        _configuration: ProcessTreeConfiguration,
    ) -> PublicResult<Self> {
        use windows_sys::Win32::System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
            SetInformationJobObject,
        };

        let process = child
            .raw_handle()
            .ok_or_else(|| PublicError::unexpected("Codex process handle was unavailable"))?;
        unsafe {
            let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if job.is_null() {
                return Err(PublicError::unexpected(format!(
                    "failed to create Codex process job: {}",
                    std::io::Error::last_os_error()
                )));
            }
            let job = OwnedHandle::from_raw_handle(job.cast());
            let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if SetInformationJobObject(
                job.as_raw_handle().cast(),
                JobObjectExtendedLimitInformation,
                std::ptr::from_ref(&limits).cast(),
                std::mem::size_of_val(&limits) as u32,
            ) == 0
            {
                let error = std::io::Error::last_os_error();
                return Err(PublicError::unexpected(format!(
                    "failed to secure Codex process job: {error}"
                )));
            }
            if AssignProcessToJobObject(job.as_raw_handle().cast(), process.cast()) == 0 {
                let error = std::io::Error::last_os_error();
                return Err(PublicError::unexpected(format!(
                    "failed to assign Codex to its process job: {error}"
                )));
            }
            Ok(Self { job: Some(job) })
        }
    }

    fn terminate(&mut self) {
        self.job.take();
    }
}

#[cfg(windows)]
impl Drop for ProcessTreeGuard {
    fn drop(&mut self) {
        self.terminate();
    }
}

impl CodexHarness {
    pub(crate) fn new(executable: PathBuf) -> Self {
        Self { executable }
    }
}

impl Harness for CodexHarness {
    fn run<'a>(
        &'a self,
        worktree: &'a Path,
        run_directory: &'a Path,
        prompt: &'a str,
    ) -> Pin<Box<dyn Future<Output = PublicResult<HarnessOutput>> + Send + 'a>> {
        Box::pin(async move { self.run_codex(worktree, run_directory, prompt).await })
    }
}

impl CodexHarness {
    async fn run_codex(
        &self,
        worktree: &Path,
        run_directory: &Path,
        prompt: &str,
    ) -> PublicResult<HarnessOutput> {
        let final_message = run_directory.join("codex-final.txt");
        create_private_output_file(&final_message).await?;
        let mut plaintext_guard = PlaintextOutputGuard::new(final_message.clone());

        let mut command = Command::new(&self.executable);
        command
            .arg("exec")
            .arg("--json")
            .arg("--color")
            .arg("never")
            .arg("--sandbox")
            .arg("workspace-write")
            .arg("--ephemeral")
            .arg("--ignore-user-config")
            .arg("-c")
            .arg("approval_policy=\"never\"")
            .arg("-c")
            .arg(CODEX_SUBPROCESS_ENV_ALLOWLIST)
            .arg("--output-last-message")
            .arg(&final_message)
            .arg("-C")
            .arg(worktree)
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        apply_harness_environment(&mut command);
        let process_tree_configuration = configure_process_tree(&mut command)?;

        let mut child = command.spawn().map_err(|error| {
            PublicError::unexpected(format!("failed to launch Codex harness: {error}"))
        })?;
        let mut process_tree = ProcessTreeGuard::new(&child, process_tree_configuration)?;
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| PublicError::unexpected("Codex harness stdin was unavailable"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| PublicError::unexpected("Codex harness stdout was unavailable"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| PublicError::unexpected("Codex harness stderr was unavailable"))?;
        // Start draining both output pipes before the child can consume stdin.
        // Otherwise a child that emits enough early output can block on its
        // output pipe while this task is blocked writing the prompt.
        let stdout_task = tokio::spawn(drain_bounded(stdout));
        let stderr_task = tokio::spawn(drain_bounded(stderr));
        resume_process_tree(&child)?;
        stdin.write_all(prompt.as_bytes()).await.map_err(|error| {
            PublicError::unexpected(format!("failed to send task to Codex harness: {error}"))
        })?;
        stdin.shutdown().await.map_err(|error| {
            PublicError::unexpected(format!("failed to close Codex harness input: {error}"))
        })?;
        drop(stdin);

        let status = wait_for_process_tree(&mut child, &mut process_tree)
            .await
            .map_err(|error| {
                PublicError::unexpected(format!("failed to wait for Codex harness: {error}"))
            })?;
        // Descendants can outlive the direct Codex process and keep its output
        // pipes open. The contained wait terminates the owned tree before it
        // reaps the Unix group leader and before these collectors are joined.
        let stdout = stdout_task.await.map_err(|_| {
            PublicError::unexpected("Codex stdout collector stopped unexpectedly")
        })??;
        let stderr = stderr_task.await.map_err(|_| {
            PublicError::unexpected("Codex stderr collector stopped unexpectedly")
        })??;
        secure_private_file(&final_message).await?;
        let summary = compose_harness_summary(
            status.success(),
            read_bounded_final_message(&final_message).await?,
            &stdout,
            &stderr,
        );
        fs::remove_file(&final_message).await.map_err(|error| {
            PublicError::unexpected(format!(
                "failed to remove plaintext harness output: {error}"
            ))
        })?;
        plaintext_guard.disarm();

        Ok(HarnessOutput {
            succeeded: status.success(),
            summary,
            failure_code: (!status.success()).then(|| "codex_exit".to_string()),
        })
    }
}

fn apply_harness_environment(command: &mut Command) {
    command.env_clear();
    for name in [
        "PATH",
        "HOME",
        "CODEX_HOME",
        "TMPDIR",
        "LANG",
        "LC_ALL",
        "SSL_CERT_FILE",
        "SSL_CERT_DIR",
    ] {
        if let Some(value) = env::var_os(name) {
            command.env(name, value);
        }
    }
}

pub(crate) fn configure_process_tree(
    command: &mut Command,
) -> PublicResult<ProcessTreeConfiguration> {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt as _;

        #[cfg(target_os = "linux")]
        enable_child_subreaper()?;
        let owner_process_id = i32::try_from(std::process::id())
            .map_err(|_| PublicError::unexpected("agent process ID exceeded platform limits"))?;
        let baseline_children = process_parent_map()?
            .into_iter()
            .filter_map(|(process_id, parent_id)| {
                (parent_id == owner_process_id).then_some(process_id)
            })
            .collect();
        command.as_std_mut().process_group(0);
        Ok(ProcessTreeConfiguration {
            owner_process_id,
            baseline_children,
        })
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        command
            .as_std_mut()
            .creation_flags(windows_sys::Win32::System::Threading::CREATE_SUSPENDED);
        Ok(ProcessTreeConfiguration)
    }
}

/// Waits for the contained process and terminates its descendants without
/// releasing a Unix PID/process-group identity before cleanup signals run.
pub(crate) async fn wait_for_process_tree(
    child: &mut Child,
    process_tree: &mut ProcessTreeGuard,
) -> io::Result<ExitStatus> {
    #[cfg(unix)]
    {
        wait_for_child_exit_without_reaping(child).await?;
        process_tree.terminate();
        child.wait().await
    }
    #[cfg(windows)]
    {
        let status = child.wait().await?;
        process_tree.terminate();
        Ok(status)
    }
}

#[cfg(unix)]
async fn wait_for_child_exit_without_reaping(child: &Child) -> io::Result<()> {
    let process_id = child
        .id()
        .ok_or_else(|| io::Error::other("contained process ID was unavailable"))?;
    let process_id = i32::try_from(process_id)
        .map_err(|_| io::Error::other("contained process ID exceeded platform limits"))?;
    tokio::task::spawn_blocking(move || {
        loop {
            let mut information = std::mem::MaybeUninit::<libc::siginfo_t>::uninit();
            let result = unsafe {
                libc::waitid(
                    libc::P_PID,
                    process_id as libc::id_t,
                    information.as_mut_ptr(),
                    libc::WEXITED | libc::WNOWAIT,
                )
            };
            if result == 0 {
                return Ok(());
            }
            let error = io::Error::last_os_error();
            if error.kind() != io::ErrorKind::Interrupted {
                return Err(error);
            }
        }
    })
    .await
    .map_err(io::Error::other)?
}

#[cfg(target_os = "linux")]
fn enable_child_subreaper() -> PublicResult<()> {
    static RESULT: OnceLock<Result<(), String>> = OnceLock::new();
    RESULT
        .get_or_init(|| {
            let result = unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0) };
            (result == 0).then_some(()).ok_or_else(|| {
                format!(
                    "failed to enable Linux child-subreaper containment: {}",
                    std::io::Error::last_os_error()
                )
            })
        })
        .clone()
        .map_err(PublicError::unexpected)
}

#[cfg(target_os = "linux")]
fn process_parent_map() -> PublicResult<HashMap<i32, i32>> {
    let entries = std::fs::read_dir("/proc").map_err(|error| {
        PublicError::unexpected(format!(
            "failed to inspect Linux process containment: {error}"
        ))
    })?;
    let mut processes = HashMap::new();
    for entry in entries.flatten() {
        let Some(process_id) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<i32>().ok())
        else {
            continue;
        };
        let Ok(stat) = std::fs::read_to_string(entry.path().join("stat")) else {
            continue;
        };
        let Some(command_end) = stat.rfind(')') else {
            continue;
        };
        let mut fields = stat[command_end + 1..].split_whitespace();
        let _state = fields.next();
        let Some(parent_id) = fields.next().and_then(|value| value.parse::<i32>().ok()) else {
            continue;
        };
        processes.insert(process_id, parent_id);
    }
    Ok(processes)
}

#[cfg(all(unix, not(target_os = "linux")))]
fn process_parent_map() -> PublicResult<HashMap<i32, i32>> {
    let child = std::process::Command::new("/bin/ps")
        .args(["-axo", "pid=,ppid="])
        .env_clear()
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| {
            PublicError::unexpected(format!(
                "failed to inspect Unix process containment: {error}"
            ))
        })?;
    let helper_process_id = child.id();
    let output = child.wait_with_output().map_err(|error| {
        PublicError::unexpected(format!("failed to read Unix process containment: {error}"))
    })?;
    if !output.status.success() {
        return Err(PublicError::unexpected(
            "failed to inspect Unix process containment",
        ));
    }
    let helper_process_id = i32::try_from(helper_process_id)
        .map_err(|_| PublicError::unexpected("Unix process scanner ID exceeded platform limits"))?;
    parse_process_parent_map(&output.stdout, helper_process_id)
}

#[cfg(all(unix, not(target_os = "linux")))]
fn parse_process_parent_map(
    output: &[u8],
    helper_process_id: i32,
) -> PublicResult<HashMap<i32, i32>> {
    let process_list = std::str::from_utf8(output)
        .map_err(|_| PublicError::unexpected("Unix process inspection returned non-UTF-8 data"))?;
    Ok(process_list
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let process_id = fields.next()?.parse::<i32>().ok()?;
            let parent_id = fields.next()?.parse::<i32>().ok()?;
            (process_id != helper_process_id).then_some((process_id, parent_id))
        })
        .collect())
}

#[cfg(unix)]
fn owned_processes(
    processes: &HashMap<i32, i32>,
    root_process_id: i32,
    owner_process_id: i32,
    baseline_children: &HashSet<i32>,
) -> HashSet<i32> {
    let mut owned = HashSet::new();
    owned.extend(processes.iter().filter_map(|(process_id, parent_id)| {
        (*parent_id == owner_process_id && !baseline_children.contains(process_id))
            .then_some(*process_id)
    }));
    if processes.get(&root_process_id) == Some(&owner_process_id) {
        owned.insert(root_process_id);
    }
    loop {
        let before = owned.len();
        let descendants = processes
            .iter()
            .filter_map(|(process_id, parent_id)| owned.contains(parent_id).then_some(*process_id))
            .collect::<Vec<_>>();
        owned.extend(descendants);
        if owned.len() == before {
            break;
        }
    }
    owned
}

#[cfg(target_os = "linux")]
fn reap_subreaper_descendants(processes: &HashSet<i32>, root_process_id: i32) {
    // Tokio owns the root Child handle, so never reap that PID here. Children
    // that escaped through daemonization are not known to Tokio; once SIGKILL
    // reparents them to this subreaper, reap them explicitly to avoid a zombie
    // leak across long-running service polls.
    let mut pending = processes
        .iter()
        .copied()
        .filter(|process_id| *process_id != root_process_id)
        .collect::<HashSet<_>>();
    for _ in 0..64 {
        pending.retain(|process_id| {
            let result = unsafe { libc::waitpid(*process_id, std::ptr::null_mut(), libc::WNOHANG) };
            result != *process_id
        });
        if pending.is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
}

#[cfg(unix)]
pub(crate) fn resume_process_tree(_child: &tokio::process::Child) -> PublicResult<()> {
    Ok(())
}

#[cfg(windows)]
pub(crate) fn resume_process_tree(child: &tokio::process::Child) -> PublicResult<()> {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, INVALID_HANDLE_VALUE},
        System::{
            Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First,
                Thread32Next,
            },
            Threading::{OpenThread, ResumeThread, THREAD_SUSPEND_RESUME},
        },
    };

    let process_id = child
        .id()
        .ok_or_else(|| PublicError::unexpected("Codex process ID was unavailable"))?;
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return Err(PublicError::unexpected(format!(
                "failed to enumerate suspended Codex threads: {}",
                std::io::Error::last_os_error()
            )));
        }
        let mut entry = THREADENTRY32 {
            dwSize: std::mem::size_of::<THREADENTRY32>() as u32,
            ..Default::default()
        };
        let mut found = false;
        let mut has_entry = Thread32First(snapshot, &mut entry) != 0;
        while has_entry {
            if entry.th32OwnerProcessID == process_id {
                let thread = OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32ThreadID);
                if thread.is_null() {
                    let error = std::io::Error::last_os_error();
                    CloseHandle(snapshot);
                    return Err(PublicError::unexpected(format!(
                        "failed to open suspended Codex thread: {error}"
                    )));
                }
                let resume_result = ResumeThread(thread);
                let resume_error = (resume_result == u32::MAX).then(std::io::Error::last_os_error);
                CloseHandle(thread);
                if let Some(error) = resume_error {
                    CloseHandle(snapshot);
                    return Err(PublicError::unexpected(format!(
                        "failed to resume Codex after job assignment: {error}"
                    )));
                }
                found = true;
            }
            has_entry = Thread32Next(snapshot, &mut entry) != 0;
        }
        CloseHandle(snapshot);
        if !found {
            return Err(PublicError::unexpected(
                "suspended Codex process had no resumable thread",
            ));
        }
    }
    Ok(())
}

pub(crate) fn apply_git_environment(command: &mut Command) {
    command.env_clear();
    for name in ["PATH", "TMPDIR", "LANG", "LC_ALL"] {
        if let Some(value) = env::var_os(name) {
            command.env(name, value);
        }
    }
}

async fn drain_bounded(mut stream: impl AsyncRead + Unpin) -> PublicResult<Vec<u8>> {
    let mut captured = VecDeque::with_capacity(MAX_HARNESS_STREAM_BYTES);
    let mut buffer = [0_u8; 8192];
    loop {
        let count = stream.read(&mut buffer).await.map_err(|error| {
            PublicError::unexpected(format!("failed to read harness output: {error}"))
        })?;
        if count == 0 {
            return Ok(captured.into());
        }
        let overflow = captured
            .len()
            .saturating_add(count)
            .saturating_sub(MAX_HARNESS_STREAM_BYTES);
        if overflow > 0 {
            captured.drain(..overflow.min(captured.len()));
        }
        let start = count.saturating_sub(MAX_HARNESS_STREAM_BYTES);
        captured.extend(&buffer[start..count]);
    }
}

fn compose_harness_summary(
    succeeded: bool,
    final_message: String,
    stdout: &[u8],
    stderr: &[u8],
) -> String {
    let mut summary = if final_message.trim().is_empty() {
        if succeeded {
            "Codex completed without a final summary.".to_string()
        } else {
            "Codex exited without a final summary.".to_string()
        }
    } else {
        final_message
    };
    if succeeded {
        return summary;
    }

    let diagnostic = sanitized_harness_diagnostic(stderr)
        .map(|diagnostic| ("stderr", diagnostic))
        .or_else(|| sanitized_harness_diagnostic(stdout).map(|diagnostic| ("stdout", diagnostic)));
    if let Some((stream, diagnostic)) = diagnostic {
        summary.push_str("\n\nCodex ");
        summary.push_str(stream);
        summary.push_str(" (sanitized, bounded tail):\n");
        summary.push_str(&diagnostic);
    }
    summary
}

fn sanitized_harness_diagnostic(stream: &[u8]) -> Option<String> {
    let start = stream.len().saturating_sub(MAX_HARNESS_DIAGNOSTIC_BYTES);
    let diagnostic = String::from_utf8_lossy(&stream[start..]);
    let mut sanitized = String::with_capacity(diagnostic.len());
    for character in diagnostic.chars() {
        match character {
            '\n' | '\t' => sanitized.push(character),
            '\r' => {}
            character if character.is_whitespace() => sanitized.push(' '),
            character if character.is_control() || is_bidi_control(character) => {
                sanitized.push('\u{fffd}');
            }
            character => sanitized.push(character),
        }
    }
    let sanitized = sanitized.trim();
    (!sanitized.is_empty()).then(|| sanitized.to_string())
}

fn is_bidi_control(character: char) -> bool {
    matches!(
        character,
        '\u{061c}'
            | '\u{200e}'..='\u{200f}'
            | '\u{202a}'..='\u{202e}'
            | '\u{2066}'..='\u{2069}'
    )
}

async fn create_private_output_file(path: &Path) -> PublicResult<()> {
    let mut options = fs::OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    {
        options.mode(0o600);
    }
    options.open(path).await.map_err(|error| {
        PublicError::unexpected(format!("failed to create private harness output: {error}"))
    })?;
    Ok(())
}

async fn secure_private_file(path: &Path) -> PublicResult<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .await
            .map_err(|error| {
                PublicError::unexpected(format!("failed to secure harness output: {error}"))
            })?;
    }
    Ok(())
}

async fn read_bounded_final_message(path: &Path) -> PublicResult<String> {
    let metadata = fs::metadata(path).await.map_err(|error| {
        PublicError::unexpected(format!("failed to inspect Codex final message: {error}"))
    })?;
    if metadata.len() > MAX_FINAL_MESSAGE_BYTES {
        return Err(PublicError::validation(
            "Codex final message exceeded the supported size",
        ));
    }
    fs::read_to_string(path).await.map_err(|error| {
        PublicError::unexpected(format!("failed to read Codex final message: {error}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn child_shell_environment_is_explicitly_allowlisted() {
        assert!(CODEX_SUBPROCESS_ENV_ALLOWLIST.contains("PATH"));
        assert!(!CODEX_SUBPROCESS_ENV_ALLOWLIST.contains("SEALTASK"));
        assert!(!CODEX_SUBPROCESS_ENV_ALLOWLIST.contains("TOKEN"));
    }

    #[test]
    fn executable_path_is_kept_as_an_os_value() {
        let harness = CodexHarness::new(PathBuf::from(std::ffi::OsStr::new("codex")));
        assert_eq!(harness.executable, PathBuf::from("codex"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn harness_drains_early_output_before_writing_a_large_prompt() {
        use std::os::unix::fs::PermissionsExt as _;

        let temporary = tempfile::tempdir().expect("harness I/O fixture directory");
        let executable = temporary.path().join("early-output-codex");
        let block = "x".repeat(1024);
        let script = format!(
            "#!/bin/sh\n\
             output_file=\n\
             while [ \"$#\" -gt 0 ]; do\n\
               if [ \"$1\" = \"--output-last-message\" ]; then\n\
                 shift\n\
                 output_file=$1\n\
               fi\n\
               shift\n\
             done\n\
             i=0\n\
             while [ \"$i\" -lt 256 ]; do\n\
               printf '%s' '{block}' >&2\n\
               i=$((i + 1))\n\
             done\n\
             /bin/cat >/dev/null\n\
             printf '%s' 'drained' >\"$output_file\"\n"
        );
        std::fs::write(&executable, script).expect("write harness I/O fixture");
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700))
            .expect("make harness I/O fixture executable");
        let harness = CodexHarness::new(executable);
        let prompt = "p".repeat(MAX_HARNESS_STREAM_BYTES * 2);

        let output = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            harness.run_codex(temporary.path(), temporary.path(), &prompt),
        )
        .await
        .expect("harness must not deadlock on opposing full pipes")
        .expect("run harness I/O fixture");

        assert!(output.succeeded, "{}", output.summary);
        assert_eq!(output.summary, "drained");
    }

    #[test]
    fn failed_harness_summary_includes_only_sanitized_bounded_diagnostics() {
        let mut stderr = b"discarded-prefix".to_vec();
        stderr.extend(std::iter::repeat_n(b'x', MAX_HARNESS_DIAGNOSTIC_BYTES + 32));
        stderr.extend_from_slice("\nuseful failure\x1b[31m\u{202e}\r\n".as_bytes());

        let summary = compose_harness_summary(false, String::new(), b"less useful stdout", &stderr);

        assert!(summary.contains("Codex stderr (sanitized, bounded tail):"));
        assert!(summary.contains("useful failure"));
        assert!(!summary.contains("discarded-prefix"));
        assert!(!summary.contains("less useful stdout"));
        assert!(!summary.contains('\x1b'));
        assert!(!summary.contains('\u{202e}'));
    }

    #[cfg(all(unix, not(target_os = "linux")))]
    #[test]
    fn unix_process_parser_excludes_its_scanner_helper() {
        let processes =
            parse_process_parent_map(b"10 1\n20 10\n30 10\n", 20).expect("parse process fixture");

        assert_eq!(processes.get(&10), Some(&1));
        assert!(!processes.contains_key(&20));
        assert_eq!(processes.get(&30), Some(&10));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn process_tree_guard_terminates_the_owned_process_group() {
        let mut command = Command::new("sh");
        command
            .arg("-c")
            .arg("sleep 60 & wait")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        let configuration = configure_process_tree(&mut command).expect("configure process tree");
        let mut child = command.spawn().expect("spawn process tree fixture");
        let process_group_id = i32::try_from(child.id().expect("fixture process ID"))
            .expect("fixture process ID fits platform");
        let mut guard =
            ProcessTreeGuard::new(&child, configuration).expect("own fixture process tree");
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        guard.terminate();
        tokio::time::timeout(std::time::Duration::from_secs(5), child.wait())
            .await
            .expect("process tree terminates promptly")
            .expect("wait for fixture process");

        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            let exists = unsafe { libc::kill(-process_group_id, 0) } == 0;
            if !exists {
                break;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "a descendant survived process-tree termination"
            );
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn process_tree_guard_terminates_the_owned_group_after_its_leader_exits() {
        let mut command = Command::new("/bin/sh");
        command
            .arg("-c")
            .arg("trap '' HUP; sleep 60 &")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        let configuration = configure_process_tree(&mut command).expect("configure process tree");
        let mut child = command.spawn().expect("spawn process tree fixture");
        let process_group_id = i32::try_from(child.id().expect("fixture process ID"))
            .expect("fixture process ID fits platform");
        let mut guard =
            ProcessTreeGuard::new(&child, configuration).expect("own fixture process tree");

        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            wait_for_child_exit_without_reaping(&child),
        )
        .await
        .expect("fixture leader exits promptly")
        .expect("observe fixture leader without reaping it");
        assert_eq!(
            unsafe { libc::kill(-process_group_id, 0) },
            0,
            "fixture descendant must keep the process group alive"
        );

        guard.terminate();
        let status = child.wait().await.expect("reap fixture leader");
        assert!(status.success());
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            if unsafe { libc::kill(-process_group_id, 0) } != 0 {
                break;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "a descendant survived cleanup after its group leader exited"
            );
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn exit_observation_keeps_the_process_identity_reserved_until_cleanup() {
        let mut command = Command::new("/bin/sh");
        command
            .arg("-c")
            .arg("exit 0")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        let configuration = configure_process_tree(&mut command).expect("configure process tree");
        let mut child = command.spawn().expect("spawn process identity fixture");
        let process_id = i32::try_from(child.id().expect("fixture process ID"))
            .expect("fixture process ID fits platform");
        let mut guard =
            ProcessTreeGuard::new(&child, configuration).expect("own fixture process identity");

        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            wait_for_child_exit_without_reaping(&child),
        )
        .await
        .expect("fixture exits promptly")
        .expect("observe fixture exit without reaping it");
        assert_eq!(
            unsafe { libc::kill(process_id, 0) },
            0,
            "the waitable leader must continue reserving its PID and process-group ID"
        );

        guard.terminate();
        let status = child.wait().await.expect("reap process identity fixture");
        assert!(status.success());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn escaped_process_fixture() {
        use std::ffi::CString;

        let Ok(marker) = std::env::var("SEALTASK_TEST_SETSID_MARKER") else {
            return;
        };
        let marker = CString::new(marker).expect("marker path has no NUL bytes");
        unsafe {
            let first_child = libc::fork();
            assert!(first_child >= 0, "fork escaped fixture");
            if first_child > 0 {
                libc::_exit(0);
            }
            assert!(libc::setsid() >= 0, "create escaped fixture session");
            let daemon = libc::fork();
            assert!(daemon >= 0, "fork daemon fixture");
            if daemon > 0 {
                libc::_exit(0);
            }

            let descriptor = libc::open(
                marker.as_ptr(),
                libc::O_CREAT | libc::O_WRONLY | libc::O_TRUNC,
                0o600,
            );
            assert!(descriptor >= 0, "open escaped fixture marker");
            let mut digits = [0_u8; 20];
            let mut process_id = libc::getpid() as u32;
            let mut start = digits.len();
            loop {
                start -= 1;
                digits[start] = b'0' + u8::try_from(process_id % 10).expect("PID digit");
                process_id /= 10;
                if process_id == 0 {
                    break;
                }
            }
            assert_eq!(
                libc::write(
                    descriptor,
                    digits[start..].as_ptr().cast(),
                    digits.len() - start,
                ),
                isize::try_from(digits.len() - start).expect("PID marker length"),
                "write escaped fixture process ID",
            );
            libc::close(descriptor);
            loop {
                libc::pause();
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn escaped_session_fixture() {
        let Ok(marker) = std::env::var("SEALTASK_TEST_SETSID_MARKER") else {
            return;
        };
        assert!(
            unsafe { libc::setsid() } >= 0,
            "create escaped fixture session"
        );
        std::fs::write(marker, std::process::id().to_string())
            .expect("write escaped fixture process ID");
        std::thread::sleep(std::time::Duration::from_secs(60));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn process_tree_guard_terminates_a_descendant_that_calls_setsid() {
        assert_escaped_fixture_is_terminated("harness::tests::escaped_session_fixture").await;
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn process_tree_guard_terminates_a_daemonized_descendant_that_calls_setsid() {
        assert_escaped_fixture_is_terminated("harness::tests::escaped_process_fixture").await;
    }

    #[cfg(unix)]
    async fn assert_escaped_fixture_is_terminated(test_name: &str) {
        let temporary = tempfile::tempdir().expect("process fixture directory");
        let marker = temporary.path().join("escaped.pid");
        let test_binary = std::env::current_exe().expect("current test executable");
        let mut command = Command::new("/bin/sh");
        command
            .arg("-c")
            .arg("\"$1\" --exact \"$2\" --nocapture & wait")
            .arg("sh")
            .arg(test_binary)
            .arg(test_name)
            .env("SEALTASK_TEST_SETSID_MARKER", &marker)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        let configuration = configure_process_tree(&mut command).expect("configure escaped tree");
        let mut child = command.spawn().expect("spawn escaped process tree fixture");
        let mut guard =
            ProcessTreeGuard::new(&child, configuration).expect("own escaped process tree");
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while !marker.exists() {
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("setsid fixture starts");
        let escaped_process_id = std::fs::read_to_string(&marker)
            .expect("read escaped fixture process ID")
            .parse::<i32>()
            .expect("escaped fixture process ID");
        assert_eq!(unsafe { libc::kill(escaped_process_id, 0) }, 0);

        guard.terminate();
        tokio::time::timeout(std::time::Duration::from_secs(5), child.wait())
            .await
            .expect("escaped process tree terminates promptly")
            .expect("wait for escaped fixture root");
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            if unsafe { libc::kill(escaped_process_id, 0) } != 0 {
                break;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "setsid descendant survived process-tree termination"
            );
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    }

    #[cfg(windows)]
    #[test]
    fn process_tree_guard_is_send() {
        fn assert_send<T: Send>() {}

        assert_send::<ProcessTreeGuard>();
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn process_is_assigned_to_its_job_before_it_can_create_descendants() {
        let temporary = tempfile::tempdir().expect("process fixture directory");
        let marker = temporary.path().join("started.txt");
        let mut command = Command::new("cmd.exe");
        command
            .arg("/C")
            .arg(format!(
                "(echo started)>\"{}\" & ping -n 60 127.0.0.1 >NUL",
                marker.display()
            ))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        let configuration = configure_process_tree(&mut command).expect("configure process job");
        let mut child = command.spawn().expect("spawn suspended process fixture");
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        assert!(
            !marker.exists(),
            "suspended process executed before job assignment"
        );

        let mut guard =
            ProcessTreeGuard::new(&child, configuration).expect("assign fixture process job");
        assert!(
            !marker.exists(),
            "job assignment unexpectedly resumed the process"
        );
        resume_process_tree(&child).expect("resume fixture after job assignment");
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while !marker.exists() {
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("resumed fixture starts");

        guard.terminate();
        tokio::time::timeout(std::time::Duration::from_secs(5), child.wait())
            .await
            .expect("job terminates fixture tree promptly")
            .expect("wait for Windows fixture");
    }
}
