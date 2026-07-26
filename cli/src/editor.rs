use crate::output::{CliError, CliResult};
use crate::terminal;
use sealtask_client_core::PublicError;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
#[cfg(unix)]
use std::sync::atomic::{AtomicI32, AtomicUsize, Ordering};
#[cfg(unix)]
use std::thread;
#[cfg(unix)]
use std::time::{Duration, Instant};
use tempfile::{Builder, TempDir};
use zeroize::{Zeroize, Zeroizing};

#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

const DOCUMENT_FILE_NAME: &str = "document.md";
const EDITOR_VARIABLES: &[&str] = &["SEALTASK_EDITOR", "VISUAL", "EDITOR"];
#[cfg(unix)]
const EDITOR_CHILD_POLL_INTERVAL: Duration = Duration::from_millis(25);
#[cfg(unix)]
const EDITOR_TERMINATION_GRACE: Duration = Duration::from_secs(2);
const MAX_DOCUMENT_BYTES: usize = 4 * 1024 * 1024;
const TEMP_DIRECTORY_PREFIX: &str = "sealtask-edit-";

#[cfg(unix)]
const EDITOR_TERMINATION_SIGNALS: [libc::c_int; 4] =
    [libc::SIGINT, libc::SIGTERM, libc::SIGHUP, libc::SIGQUIT];
#[cfg(unix)]
static EDITOR_SIGNAL: AtomicI32 = AtomicI32::new(0);
#[cfg(unix)]
static EDITOR_SIGNAL_COUNT: AtomicUsize = AtomicUsize::new(0);

pub(crate) struct EditedDocument {
    pub(crate) title: String,
    pub(crate) body: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct EditorCommand {
    program: OsString,
    args: Vec<OsString>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct EditorExit {
    success: bool,
    code: Option<i32>,
}

struct ControllingTerminal {
    stdin: File,
    stdout: File,
    stderr: File,
}

struct ProcessEnvironment;

struct SystemEditorLauncher;

#[cfg(unix)]
struct EditorSignalGuard {
    previous_actions: Vec<(libc::c_int, libc::sigaction)>,
}

trait EditorEnvironment {
    fn value(&self, name: &str) -> Option<OsString>;
}

trait EditorLauncher {
    fn launch(&mut self, command: &EditorCommand, document_path: &Path) -> CliResult<EditorExit>;
}

impl fmt::Debug for EditedDocument {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EditedDocument")
            .field("title", &"<redacted>")
            .field("body", &"<redacted>")
            .finish()
    }
}

impl Drop for EditedDocument {
    fn drop(&mut self) {
        self.title.zeroize();
        self.body.zeroize();
    }
}

impl EditorEnvironment for ProcessEnvironment {
    fn value(&self, name: &str) -> Option<OsString> {
        env::var_os(name)
    }
}

impl EditorLauncher for SystemEditorLauncher {
    fn launch(&mut self, command: &EditorCommand, document_path: &Path) -> CliResult<EditorExit> {
        let terminal = ControllingTerminal::open()?;
        let mut child = Command::new(&command.program)
            .args(&command.args)
            .arg(document_path)
            .stdin(Stdio::from(terminal.stdin))
            .stdout(Stdio::from(terminal.stdout))
            .stderr(Stdio::from(terminal.stderr))
            .spawn()
            .map_err(|error| {
                PublicError::unexpected(format!(
                    "failed to start the configured editor: {error}; check SEALTASK_EDITOR, VISUAL, or EDITOR"
                ))
            })?;
        wait_for_editor_child(&mut child)
    }
}

impl<F> EditorLauncher for F
where
    F: FnMut(&EditorCommand, &Path) -> CliResult<EditorExit>,
{
    fn launch(&mut self, command: &EditorCommand, document_path: &Path) -> CliResult<EditorExit> {
        self(command, document_path)
    }
}

impl ControllingTerminal {
    #[cfg(unix)]
    fn open() -> CliResult<Self> {
        let stdin = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/tty")
            .map_err(controlling_terminal_error)?;
        let stdout = stdin.try_clone().map_err(controlling_terminal_error)?;
        let stderr = stdin.try_clone().map_err(controlling_terminal_error)?;
        Ok(Self {
            stdin,
            stdout,
            stderr,
        })
    }

    #[cfg(windows)]
    fn open() -> CliResult<Self> {
        let stdin = OpenOptions::new()
            .read(true)
            .open("CONIN$")
            .map_err(controlling_terminal_error)?;
        let stdout = OpenOptions::new()
            .write(true)
            .open("CONOUT$")
            .map_err(controlling_terminal_error)?;
        let stderr = stdout.try_clone().map_err(controlling_terminal_error)?;
        Ok(Self {
            stdin,
            stdout,
            stderr,
        })
    }

    #[cfg(not(any(unix, windows)))]
    fn open() -> CliResult<Self> {
        Err(
            PublicError::validation("interactive editor input is not supported on this platform")
                .into(),
        )
    }
}

#[cfg(unix)]
impl EditorSignalGuard {
    fn install() -> io::Result<Self> {
        EDITOR_SIGNAL.store(0, Ordering::SeqCst);
        EDITOR_SIGNAL_COUNT.store(0, Ordering::SeqCst);

        // SAFETY: `sigaction` is a plain C struct that is valid when zeroed, and
        // `sigemptyset` initializes its signal mask before it is registered.
        let mut action = unsafe { std::mem::zeroed::<libc::sigaction>() };
        action.sa_sigaction = capture_editor_signal as *const () as libc::sighandler_t;
        action.sa_flags = 0;
        // SAFETY: `action.sa_mask` is a valid, writable sigset owned by this function.
        if unsafe { libc::sigemptyset(&mut action.sa_mask) } != 0 {
            return Err(io::Error::last_os_error());
        }

        let mut guard = Self {
            previous_actions: Vec::with_capacity(EDITOR_TERMINATION_SIGNALS.len()),
        };
        for signal in EDITOR_TERMINATION_SIGNALS {
            // SAFETY: `previous` is initialized by `sigaction` on success. The
            // handler is a process-lifetime function and only performs atomic stores.
            let mut previous = unsafe { std::mem::zeroed::<libc::sigaction>() };
            // SAFETY: pointers refer to valid sigaction values for the duration
            // of the call, and `signal` is one of the supported termination signals.
            if unsafe { libc::sigaction(signal, &action, &mut previous) } != 0 {
                let error = io::Error::last_os_error();
                let _ = guard.restore_actions();
                return Err(error);
            }
            guard.previous_actions.push((signal, previous));
        }
        Ok(guard)
    }

    fn restore(mut self) -> io::Result<()> {
        self.restore_actions()
    }

    fn restore_actions(&mut self) -> io::Result<()> {
        let mut first_error = None;
        while let Some((signal, previous)) = self.previous_actions.pop() {
            // SAFETY: `previous` was returned by a successful `sigaction` call
            // for this exact signal and remains valid for the duration of the call.
            if unsafe { libc::sigaction(signal, &previous, std::ptr::null_mut()) } != 0
                && first_error.is_none()
            {
                first_error = Some(io::Error::last_os_error());
            }
        }
        first_error.map_or(Ok(()), Err)
    }
}

#[cfg(unix)]
impl Drop for EditorSignalGuard {
    fn drop(&mut self) {
        let _ = self.restore_actions();
    }
}

#[cfg(unix)]
extern "C" fn capture_editor_signal(signal: libc::c_int) {
    let _ = EDITOR_SIGNAL.compare_exchange(0, signal, Ordering::SeqCst, Ordering::SeqCst);
    EDITOR_SIGNAL_COUNT.fetch_add(1, Ordering::SeqCst);
}

#[cfg(unix)]
fn wait_for_editor_child(child: &mut Child) -> CliResult<EditorExit> {
    let mut forwarded_signal = None;
    let mut graceful_deadline = None;

    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(editor_exit_from_status(status)),
            Ok(None) => {}
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => {
                terminate_and_reap_editor(child);
                return Err(PublicError::unexpected(format!(
                    "failed while waiting for the configured editor: {error}"
                ))
                .into());
            }
        }

        if let Some(signal) = received_editor_signal() {
            if forwarded_signal.is_none() {
                forward_signal_to_editor(child, signal)?;
                forwarded_signal = Some(signal);
                graceful_deadline = Some(Instant::now() + EDITOR_TERMINATION_GRACE);
            }

            let force_termination = EDITOR_SIGNAL_COUNT.load(Ordering::SeqCst) > 1
                || graceful_deadline.is_some_and(|deadline| Instant::now() >= deadline);
            if force_termination {
                return force_terminate_editor(child);
            }
        }

        thread::sleep(EDITOR_CHILD_POLL_INTERVAL);
    }
}

#[cfg(not(unix))]
fn wait_for_editor_child(child: &mut Child) -> CliResult<EditorExit> {
    child.wait().map(editor_exit_from_status).map_err(|error| {
        PublicError::unexpected(format!(
            "failed while waiting for the configured editor: {error}"
        ))
        .into()
    })
}

fn editor_exit_from_status(status: ExitStatus) -> EditorExit {
    EditorExit {
        success: status.success(),
        code: status.code(),
    }
}

#[cfg(unix)]
fn forward_signal_to_editor(child: &mut Child, signal: libc::c_int) -> CliResult<()> {
    // SAFETY: the PID comes from this live `Child`; `kill` does not dereference
    // pointers, and the signal is one captured from the supported signal set.
    if unsafe { libc::kill(child.id() as libc::pid_t, signal) } == 0 {
        return Ok(());
    }

    let error = io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::ESRCH) {
        return Ok(());
    }
    terminate_and_reap_editor(child);
    Err(PublicError::unexpected(format!(
        "failed to forward {} to the configured editor: {error}",
        editor_signal_name(signal)
    ))
    .into())
}

#[cfg(unix)]
fn force_terminate_editor(child: &mut Child) -> CliResult<EditorExit> {
    match child.kill() {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::InvalidInput => {}
        Err(error) => {
            terminate_and_reap_editor(child);
            return Err(PublicError::unexpected(format!(
                "failed to terminate the configured editor after interruption: {error}"
            ))
            .into());
        }
    }
    child.wait().map(editor_exit_from_status).map_err(|error| {
        PublicError::unexpected(format!(
            "failed to reap the configured editor after interruption: {error}"
        ))
        .into()
    })
}

#[cfg(unix)]
fn terminate_and_reap_editor(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(unix)]
fn received_editor_signal() -> Option<libc::c_int> {
    match EDITOR_SIGNAL.load(Ordering::SeqCst) {
        0 => None,
        signal => Some(signal),
    }
}

#[cfg(unix)]
fn editor_signal_name(signal: libc::c_int) -> &'static str {
    match signal {
        libc::SIGINT => "SIGINT",
        libc::SIGTERM => "SIGTERM",
        libc::SIGHUP => "SIGHUP",
        libc::SIGQUIT => "SIGQUIT",
        _ => "a termination signal",
    }
}

#[cfg(unix)]
fn editor_signal_error(error: io::Error) -> CliError {
    PublicError::unexpected(format!(
        "failed to supervise editor interruption safely: {error}"
    ))
    .into()
}

pub(crate) fn ensure_editor_available() -> CliResult<()> {
    let _command = resolve_editor(&ProcessEnvironment)?;
    let _terminal = ControllingTerminal::open()?;
    Ok(())
}

pub(crate) fn edit_new_document(entity_kind: &str) -> CliResult<EditedDocument> {
    edit_document_with_system_editor(entity_kind, "", "")
}

pub(crate) fn edit_existing_document(
    entity_kind: &str,
    title: &str,
    body: &str,
) -> CliResult<EditedDocument> {
    edit_document_with_system_editor(entity_kind, title, body)
}

fn edit_document_with_system_editor(
    entity_kind: &str,
    title: &str,
    body: &str,
) -> CliResult<EditedDocument> {
    let mut launcher = SystemEditorLauncher;
    edit_document_with_signal_supervision(
        entity_kind,
        title,
        body,
        &ProcessEnvironment,
        &mut launcher,
    )
}

#[cfg(unix)]
fn edit_document_with_signal_supervision(
    entity_kind: &str,
    title: &str,
    body: &str,
    environment: &impl EditorEnvironment,
    launcher: &mut impl EditorLauncher,
) -> CliResult<EditedDocument> {
    let signals = EditorSignalGuard::install().map_err(editor_signal_error)?;
    let operation = edit_document(entity_kind, title, body, environment, launcher);
    let restore_result = signals.restore();
    let received_signal = received_editor_signal();

    if let Some(signal) = received_signal {
        let mut message = format!(
            "editor input interrupted by {}; the private editor workspace was removed and no changes were applied",
            editor_signal_name(signal)
        );
        if let Err(error) = restore_result {
            message.push_str(&format!(
                "; the previous signal handlers could not be fully restored: {error}"
            ));
        }
        if let Err(error) = &operation
            && error
                .to_string()
                .contains("temporary workspace could not be removed")
        {
            message.push_str(&format!("; {error}"));
        }
        return Err(CliError::interrupted(message, &[]));
    }

    restore_result.map_err(editor_signal_error)?;
    operation
}

#[cfg(not(unix))]
fn edit_document_with_signal_supervision(
    entity_kind: &str,
    title: &str,
    body: &str,
    environment: &impl EditorEnvironment,
    launcher: &mut impl EditorLauncher,
) -> CliResult<EditedDocument> {
    edit_document(entity_kind, title, body, environment, launcher)
}

fn edit_document(
    entity_kind: &str,
    title: &str,
    body: &str,
    environment: &impl EditorEnvironment,
    launcher: &mut impl EditorLauncher,
) -> CliResult<EditedDocument> {
    let command = resolve_editor(environment)?;
    let initial_document = render_document(title, body)?;
    let workspace = Builder::new()
        .prefix(TEMP_DIRECTORY_PREFIX)
        .tempdir()
        .map_err(|error| {
            PublicError::unexpected(format!(
                "failed to create the private editor workspace: {error}"
            ))
        })?;
    let workspace_path = workspace.path().to_path_buf();
    let operation = edit_document_in_workspace(
        entity_kind,
        &workspace,
        &command,
        initial_document.as_bytes(),
        launcher,
    );
    finish_workspace(workspace, &workspace_path, operation)
}

fn edit_document_in_workspace(
    entity_kind: &str,
    workspace: &TempDir,
    command: &EditorCommand,
    initial_document: &[u8],
    launcher: &mut impl EditorLauncher,
) -> CliResult<EditedDocument> {
    set_private_directory_permissions(workspace.path())?;
    let document_path = workspace.path().join(DOCUMENT_FILE_NAME);
    create_private_document(&document_path, initial_document)?;

    terminal::clear_active_progress();
    let status = launcher.launch(command, &document_path)?;
    if !status.success {
        return Err(editor_exit_error(status));
    }

    let document = read_validated_document(&document_path)?;
    parse_document(entity_kind, document)
}

fn resolve_editor(environment: &impl EditorEnvironment) -> CliResult<EditorCommand> {
    for variable in EDITOR_VARIABLES {
        let Some(value) = environment.value(variable) else {
            continue;
        };
        if value.is_empty() {
            continue;
        }
        return parse_editor_command(variable, &value);
    }
    Ok(default_editor_command())
}

fn parse_editor_command(variable: &str, value: &OsStr) -> CliResult<EditorCommand> {
    let value = value.to_str().ok_or_else(|| {
        PublicError::validation(format!(
            "{variable} must be valid UTF-8 so its command and arguments can be parsed safely"
        ))
    })?;
    let words = shlex::split(value).ok_or_else(|| {
        PublicError::validation(format!(
            "{variable} contains unmatched quoting; set it to a directly executable command such as 'vim' or 'code --wait'"
        ))
    })?;
    let mut words = words.into_iter();
    let program = words
        .next()
        .filter(|word| !word.is_empty())
        .ok_or_else(|| {
            PublicError::validation(format!(
                "{variable} must name an editor executable such as 'vim' or 'code --wait'"
            ))
        })?;
    Ok(EditorCommand {
        program: OsString::from(program),
        args: words.map(OsString::from).collect(),
    })
}

#[cfg(unix)]
fn default_editor_command() -> EditorCommand {
    EditorCommand {
        program: OsString::from("vi"),
        args: Vec::new(),
    }
}

#[cfg(windows)]
fn default_editor_command() -> EditorCommand {
    EditorCommand {
        program: OsString::from("notepad.exe"),
        args: Vec::new(),
    }
}

#[cfg(not(any(unix, windows)))]
fn default_editor_command() -> EditorCommand {
    EditorCommand {
        program: OsString::new(),
        args: Vec::new(),
    }
}

fn render_document(title: &str, body: &str) -> CliResult<Zeroizing<String>> {
    let normalized_title = normalize_line_endings(title);
    if normalized_title.contains('\n') || normalized_title.contains('\r') {
        return Err(PublicError::validation(
            "the existing title cannot be opened in the editor because it contains a line break",
        )
        .into());
    }
    let normalized_body = normalize_line_endings(body);
    let title = normalized_title.trim();
    let capacity = title
        .len()
        .saturating_add(normalized_body.len())
        .saturating_add(3);
    if capacity > MAX_DOCUMENT_BYTES {
        return Err(PublicError::validation(format!(
            "the editor document exceeds the {MAX_DOCUMENT_BYTES}-byte safety limit"
        ))
        .into());
    }

    let mut document = Zeroizing::new(String::with_capacity(capacity));
    document.push_str(title);
    document.push_str("\n\n");
    document.push_str(&normalized_body);
    if !normalized_body.is_empty() && !normalized_body.ends_with('\n') {
        document.push('\n');
    }
    if document.len() > MAX_DOCUMENT_BYTES {
        return Err(PublicError::validation(format!(
            "the editor document exceeds the {MAX_DOCUMENT_BYTES}-byte safety limit"
        ))
        .into());
    }
    Ok(document)
}

fn normalize_line_endings(value: &str) -> Zeroizing<String> {
    Zeroizing::new(value.replace("\r\n", "\n"))
}

fn create_private_document(path: &Path, contents: &[u8]) -> CliResult<()> {
    let mut options = OpenOptions::new();
    options.create_new(true).read(true).write(true);
    #[cfg(unix)]
    options.mode(0o600);

    let mut file = options.open(path).map_err(|error| {
        PublicError::unexpected(format!(
            "failed to create the private editor document: {error}"
        ))
    })?;
    set_private_file_permissions(&file)?;
    file.write_all(contents).map_err(|error| {
        PublicError::unexpected(format!(
            "failed to write the private editor document: {error}"
        ))
    })?;
    file.flush().map_err(|error| {
        PublicError::unexpected(format!(
            "failed to finish the private editor document: {error}"
        ))
    })?;
    let metadata = file.metadata().map_err(|error| {
        PublicError::unexpected(format!(
            "failed to inspect the private editor document: {error}"
        ))
    })?;
    validate_regular_document(&metadata)?;
    validate_private_file_permissions(&metadata)?;
    Ok(())
}

fn read_validated_document(path: &Path) -> CliResult<Zeroizing<Vec<u8>>> {
    inspect_document_path(path)?;

    let mut file = OpenOptions::new().read(true).open(path).map_err(|error| {
        PublicError::unexpected(format!(
            "failed to open the edited document safely: {error}"
        ))
    })?;
    inspect_document_path(path)?;
    set_private_file_permissions(&file)?;
    let open_metadata = file.metadata().map_err(|error| {
        PublicError::unexpected(format!(
            "failed to inspect the opened editor document: {error}"
        ))
    })?;
    validate_regular_document(&open_metadata)?;
    validate_private_file_permissions(&open_metadata)?;
    validate_document_size(&open_metadata)?;
    let secured_path_metadata = inspect_document_path(path)?;
    validate_private_file_permissions(&secured_path_metadata)?;

    let mut contents = Zeroizing::new(Vec::with_capacity(open_metadata.len() as usize));
    Read::by_ref(&mut file)
        .take((MAX_DOCUMENT_BYTES as u64) + 1)
        .read_to_end(&mut contents)
        .map_err(|error| {
            PublicError::unexpected(format!("failed to read the edited document: {error}"))
        })?;
    if contents.len() > MAX_DOCUMENT_BYTES {
        return Err(PublicError::validation(format!(
            "the edited document exceeds the {MAX_DOCUMENT_BYTES}-byte safety limit"
        ))
        .into());
    }

    let final_metadata = inspect_document_path(path)?;
    validate_private_file_permissions(&final_metadata)?;
    Ok(contents)
}

fn inspect_document_path(path: &Path) -> CliResult<fs::Metadata> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        PublicError::unexpected(format!("failed to inspect the edited document: {error}"))
    })?;
    if metadata.file_type().is_symlink() {
        return Err(PublicError::validation(
            "the editor replaced the private document with a symbolic link; the edit was rejected",
        )
        .into());
    }
    validate_regular_document(&metadata)?;
    validate_document_size(&metadata)?;
    Ok(metadata)
}

fn validate_regular_document(metadata: &fs::Metadata) -> CliResult<()> {
    if metadata.is_file() {
        Ok(())
    } else {
        Err(PublicError::validation(
            "the editor document is not a regular file; the edit was rejected",
        )
        .into())
    }
}

fn validate_document_size(metadata: &fs::Metadata) -> CliResult<()> {
    if metadata.len() <= MAX_DOCUMENT_BYTES as u64 {
        Ok(())
    } else {
        Err(PublicError::validation(format!(
            "the edited document exceeds the {MAX_DOCUMENT_BYTES}-byte safety limit"
        ))
        .into())
    }
}

fn parse_document(entity_kind: &str, mut bytes: Zeroizing<Vec<u8>>) -> CliResult<EditedDocument> {
    let raw_bytes = std::mem::take(&mut *bytes);
    let mut contents = match String::from_utf8(raw_bytes) {
        Ok(contents) => Zeroizing::new(contents),
        Err(error) => {
            let mut bytes = error.into_bytes();
            bytes.zeroize();
            return Err(PublicError::validation(
                "the edited document is not valid UTF-8; save it as UTF-8 and try again",
            )
            .into());
        }
    };
    if contents.contains("\r\n") {
        let normalized = contents.replace("\r\n", "\n");
        contents.zeroize();
        contents = Zeroizing::new(normalized);
    }

    let (title, body) = contents.find('\n').map_or_else(
        || (contents.as_str(), ""),
        |title_end| {
            let title = &contents[..title_end];
            let remainder = &contents[title_end + 1..];
            (title, remainder.strip_prefix('\n').unwrap_or(remainder))
        },
    );
    let title = title.trim();
    if title.is_empty() {
        return Err(PublicError::validation(format!(
            "the edited {} title is empty; put the title on the first line",
            safe_entity_kind(entity_kind)
        ))
        .into());
    }

    let mut body = body.to_owned();
    if body.ends_with('\n') {
        body.pop();
    }
    Ok(EditedDocument {
        title: title.to_owned(),
        body,
    })
}

fn safe_entity_kind(entity_kind: &str) -> &'static str {
    match entity_kind {
        "comment" => "comment",
        "note" => "note",
        "project" => "project",
        "task" => "task",
        _ => "item",
    }
}

fn editor_exit_error(status: EditorExit) -> CliError {
    match status.code {
        Some(code) => PublicError::unexpected(format!(
            "the configured editor exited unsuccessfully with code {code}; no changes were applied"
        ))
        .into(),
        None => PublicError::unexpected(
            "the configured editor was terminated before saving successfully; no changes were applied",
        )
        .into(),
    }
}

fn finish_workspace<T>(
    workspace: TempDir,
    workspace_path: &Path,
    operation: CliResult<T>,
) -> CliResult<T> {
    let cleanup = workspace.close();
    match (operation, cleanup) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(cleanup_error)) => Err(PublicError::unexpected(format!(
            "the edit finished, but its private temporary workspace could not be removed: {cleanup_error}; remove {} before continuing",
            workspace_path.display()
        ))
        .into()),
        (Err(operation_error), Err(cleanup_error)) => Err(PublicError::unexpected(format!(
            "{operation_error}; additionally, the private temporary workspace could not be removed: {cleanup_error}; remove {} before continuing",
            workspace_path.display()
        ))
        .into()),
    }
}

fn controlling_terminal_error(error: io::Error) -> CliError {
    PublicError::validation(format!(
        "opening an editor requires a controlling terminal: {error}; use explicit file or stdin input in non-interactive environments"
    ))
    .into()
}

#[cfg(unix)]
fn set_private_directory_permissions(path: &Path) -> CliResult<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|error| {
        PublicError::unexpected(format!(
            "failed to secure the private editor workspace: {error}"
        ))
        .into()
    })
}

#[cfg(not(unix))]
fn set_private_directory_permissions(_path: &Path) -> CliResult<()> {
    Ok(())
}

#[cfg(unix)]
fn set_private_file_permissions(file: &File) -> CliResult<()> {
    file.set_permissions(fs::Permissions::from_mode(0o600))
        .map_err(|error| {
            PublicError::unexpected(format!(
                "failed to secure the private editor document: {error}"
            ))
            .into()
        })
}

#[cfg(not(unix))]
fn set_private_file_permissions(_file: &File) -> CliResult<()> {
    Ok(())
}

#[cfg(unix)]
fn validate_private_file_permissions(metadata: &fs::Metadata) -> CliResult<()> {
    if metadata.permissions().mode() & 0o777 == 0o600 {
        Ok(())
    } else {
        Err(PublicError::validation(
            "the editor document permissions changed from mode 0600; the edit was rejected",
        )
        .into())
    }
}

#[cfg(not(unix))]
fn validate_private_file_permissions(_metadata: &fs::Metadata) -> CliResult<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::path::PathBuf;
    use std::rc::Rc;

    #[cfg(unix)]
    use std::os::unix::ffi::OsStringExt;
    #[cfg(unix)]
    use std::os::unix::fs::{PermissionsExt, symlink};
    #[cfg(unix)]
    use std::process::Stdio;

    #[derive(Default)]
    struct TestEnvironment {
        values: BTreeMap<String, OsString>,
    }

    impl EditorEnvironment for TestEnvironment {
        fn value(&self, name: &str) -> Option<OsString> {
            self.values.get(name).cloned()
        }
    }

    impl TestEnvironment {
        fn with(mut self, name: &str, value: impl Into<OsString>) -> Self {
            self.values.insert(name.to_owned(), value.into());
            self
        }
    }

    fn successful_exit() -> EditorExit {
        EditorExit {
            success: true,
            code: Some(0),
        }
    }

    fn failing_exit(code: i32) -> EditorExit {
        EditorExit {
            success: false,
            code: Some(code),
        }
    }

    fn write_editor_document(path: &Path, contents: &[u8]) -> CliResult<()> {
        fs::write(path, contents).map_err(|error| {
            PublicError::unexpected(format!("test editor failed to write document: {error}")).into()
        })
    }

    #[test]
    fn editor_precedence_prefers_sealtask_then_visual_then_editor() {
        let environment = TestEnvironment::default()
            .with("EDITOR", "nano")
            .with("VISUAL", "vim")
            .with("SEALTASK_EDITOR", "code --wait");

        let command = resolve_editor(&environment).expect("resolve configured editor");

        assert_eq!(command.program, "code");
        assert_eq!(command.args, [OsString::from("--wait")]);
    }

    #[test]
    fn empty_editor_variables_are_skipped() {
        let environment = TestEnvironment::default()
            .with("SEALTASK_EDITOR", "")
            .with("VISUAL", "helix");

        let command = resolve_editor(&environment).expect("resolve visual editor");

        assert_eq!(command.program, "helix");
    }

    #[test]
    fn configured_editor_is_split_without_a_shell() {
        let environment =
            TestEnvironment::default().with("SEALTASK_EDITOR", "code --wait '--reuse-window'");

        let command = resolve_editor(&environment).expect("parse configured editor");

        assert_eq!(command.program, "code");
        assert_eq!(
            command.args,
            [OsString::from("--wait"), OsString::from("--reuse-window")]
        );
    }

    #[test]
    fn unmatched_editor_quoting_is_actionable() {
        let environment = TestEnvironment::default().with("EDITOR", "vim '");

        let error = resolve_editor(&environment).expect_err("reject malformed editor");

        assert!(
            error
                .to_string()
                .contains("EDITOR contains unmatched quoting")
        );
        assert!(!error.to_string().contains("document"));
    }

    #[test]
    fn whitespace_only_editor_command_is_rejected() {
        let environment = TestEnvironment::default().with("EDITOR", "   ");

        let error = resolve_editor(&environment).expect_err("reject empty editor command");

        assert!(error.to_string().contains("EDITOR must name an editor"));
    }

    #[test]
    fn platform_default_editor_is_directly_executable() {
        let command = resolve_editor(&TestEnvironment::default()).expect("resolve default editor");

        #[cfg(unix)]
        assert_eq!(command.program, "vi");
        #[cfg(windows)]
        assert_eq!(command.program, "notepad.exe");
        assert!(command.args.is_empty());
    }

    #[test]
    fn document_roundtrip_normalizes_crlf_and_removes_editor_newline() {
        let mut rendered =
            render_document("  Release checklist  ", "First\r\nSecond").expect("render document");
        assert_eq!(rendered.as_str(), "Release checklist\n\nFirst\nSecond\n");
        let bytes = std::mem::take(&mut *rendered).into_bytes();
        let document =
            parse_document("task", Zeroizing::new(bytes)).expect("parse rendered document");

        assert_eq!(document.title, "Release checklist");
        assert_eq!(document.body, "First\nSecond");
    }

    #[test]
    fn body_whitespace_and_intentional_blank_lines_are_preserved() {
        let document = parse_document(
            "note",
            Zeroizing::new(b"Runbook\n\n  indented  \n\n\n".to_vec()),
        )
        .expect("parse document");

        assert_eq!(document.title, "Runbook");
        assert_eq!(document.body, "  indented  \n\n");
    }

    #[test]
    fn blank_separator_is_optional_and_title_only_documents_are_supported() {
        let without_separator = parse_document(
            "task",
            Zeroizing::new(b"Release checklist\nShip it\n".to_vec()),
        )
        .expect("parse body without separator");
        let title_only = parse_document("note", Zeroizing::new(b"Runbook".to_vec()))
            .expect("parse title-only document");

        assert_eq!(without_separator.title, "Release checklist");
        assert_eq!(without_separator.body, "Ship it");
        assert_eq!(title_only.title, "Runbook");
        assert!(title_only.body.is_empty());
    }

    #[test]
    fn empty_title_is_rejected_without_content_leaks() {
        let empty = parse_document("note", Zeroizing::new(b"   \n\nprivate body".to_vec()))
            .expect_err("require title");

        assert!(empty.to_string().contains("title is empty"));
        assert!(!empty.to_string().contains("private"));
    }

    #[test]
    fn edited_document_debug_is_redacted() {
        let document = EditedDocument {
            title: "secret title".to_owned(),
            body: "secret body".to_owned(),
        };

        let debug = format!("{document:?}");

        assert_eq!(
            debug,
            "EditedDocument { title: \"<redacted>\", body: \"<redacted>\" }"
        );
        assert!(!debug.contains("secret"));
    }

    #[test]
    fn successful_edit_uses_final_path_argument_and_cleans_workspace() {
        let environment = TestEnvironment::default().with("EDITOR", "code --wait");
        let workspace_path = Rc::new(RefCell::new(None::<PathBuf>));
        let captured_path = Rc::clone(&workspace_path);
        let mut launcher = move |command: &EditorCommand, path: &Path| {
            assert_eq!(command.program, "code");
            assert_eq!(command.args, [OsString::from("--wait")]);
            assert_eq!(path.file_name(), Some(OsStr::new(DOCUMENT_FILE_NAME)));
            *captured_path.borrow_mut() = path.parent().map(Path::to_path_buf);
            write_editor_document(path, b"Edited title\n\nEdited body\n")?;
            Ok(successful_exit())
        };

        let document = edit_document(
            "task",
            "Original",
            "Original body",
            &environment,
            &mut launcher,
        )
        .expect("edit document");

        assert_eq!(document.title, "Edited title");
        assert_eq!(document.body, "Edited body");
        let workspace_path = workspace_path
            .borrow()
            .clone()
            .expect("captured workspace path");
        assert!(!workspace_path.exists());
    }

    #[test]
    fn nonzero_editor_exit_is_reported_and_workspace_is_cleaned() {
        let workspace_path = Rc::new(RefCell::new(None::<PathBuf>));
        let captured_path = Rc::clone(&workspace_path);
        let mut launcher = move |_command: &EditorCommand, path: &Path| {
            *captured_path.borrow_mut() = path.parent().map(Path::to_path_buf);
            Ok(failing_exit(23))
        };

        let error = edit_document(
            "task",
            "Private title",
            "Private body",
            &TestEnvironment::default(),
            &mut launcher,
        )
        .expect_err("nonzero editor must fail");

        assert!(error.to_string().contains("code 23"));
        assert!(!error.to_string().contains("Private"));
        let workspace_path = workspace_path
            .borrow()
            .clone()
            .expect("captured workspace path");
        assert!(!workspace_path.exists());
    }

    #[test]
    fn invalid_utf8_is_rejected_and_workspace_is_cleaned() {
        let workspace_path = Rc::new(RefCell::new(None::<PathBuf>));
        let captured_path = Rc::clone(&workspace_path);
        let mut launcher = move |_command: &EditorCommand, path: &Path| {
            *captured_path.borrow_mut() = path.parent().map(Path::to_path_buf);
            write_editor_document(path, &[0xff, 0xfe])?;
            Ok(successful_exit())
        };

        let error = edit_document(
            "note",
            "Title",
            "",
            &TestEnvironment::default(),
            &mut launcher,
        )
        .expect_err("reject non-UTF-8 document");

        assert!(error.to_string().contains("not valid UTF-8"));
        let workspace_path = workspace_path
            .borrow()
            .clone()
            .expect("captured workspace path");
        assert!(!workspace_path.exists());
    }

    #[test]
    fn oversized_editor_document_is_rejected() {
        let mut launcher = |_command: &EditorCommand, path: &Path| {
            let file = OpenOptions::new()
                .write(true)
                .open(path)
                .map_err(|error| PublicError::unexpected(error.to_string()))?;
            file.set_len((MAX_DOCUMENT_BYTES as u64) + 1)
                .map_err(|error| PublicError::unexpected(error.to_string()))?;
            Ok(successful_exit())
        };

        let error = edit_document(
            "task",
            "Title",
            "",
            &TestEnvironment::default(),
            &mut launcher,
        )
        .expect_err("reject oversized document");

        assert!(error.to_string().contains("safety limit"));
    }

    #[test]
    fn atomic_regular_file_replacement_is_accepted_and_cleaned() {
        let workspace_path = Rc::new(RefCell::new(None::<PathBuf>));
        let captured_path = Rc::clone(&workspace_path);
        let mut launcher = move |_command: &EditorCommand, path: &Path| {
            *captured_path.borrow_mut() = path.parent().map(Path::to_path_buf);
            fs::remove_file(path).map_err(|error| PublicError::unexpected(error.to_string()))?;
            write_editor_document(path, b"Replacement\n\nBody\n")?;
            Ok(successful_exit())
        };

        let document = edit_document(
            "task",
            "Title",
            "",
            &TestEnvironment::default(),
            &mut launcher,
        )
        .expect("accept private atomic replacement");

        assert_eq!(document.title, "Replacement");
        assert_eq!(document.body, "Body");
        let workspace_path = workspace_path
            .borrow()
            .clone()
            .expect("captured workspace path");
        assert!(!workspace_path.exists());
    }

    #[test]
    fn nonregular_editor_document_is_rejected() {
        let mut launcher = |_command: &EditorCommand, path: &Path| {
            fs::remove_file(path).map_err(|error| PublicError::unexpected(error.to_string()))?;
            fs::create_dir(path).map_err(|error| PublicError::unexpected(error.to_string()))?;
            Ok(successful_exit())
        };

        let error = edit_document(
            "note",
            "Title",
            "",
            &TestEnvironment::default(),
            &mut launcher,
        )
        .expect_err("reject directory document");

        assert!(error.to_string().contains("not a regular file"));
    }

    #[cfg(unix)]
    #[test]
    fn symlink_editor_document_is_rejected_without_reading_target() {
        let outside = tempfile::tempdir().expect("outside temp dir");
        let target = outside.path().join("target.md");
        fs::write(&target, "Leaked title\n\nLeaked body\n").expect("write target");
        let mut launcher = |_command: &EditorCommand, path: &Path| {
            fs::remove_file(path).map_err(|error| PublicError::unexpected(error.to_string()))?;
            symlink(&target, path).map_err(|error| PublicError::unexpected(error.to_string()))?;
            Ok(successful_exit())
        };

        let error = edit_document(
            "note",
            "Title",
            "",
            &TestEnvironment::default(),
            &mut launcher,
        )
        .expect_err("reject symlink document");

        assert!(error.to_string().contains("symbolic link"));
        assert!(!error.to_string().contains("Leaked"));
    }

    #[cfg(unix)]
    #[test]
    fn workspace_and_document_have_owner_only_permissions() {
        let mut launcher = |_command: &EditorCommand, path: &Path| {
            let directory_mode = fs::metadata(path.parent().expect("workspace path"))
                .expect("workspace metadata")
                .permissions()
                .mode()
                & 0o777;
            let document_mode = fs::metadata(path)
                .expect("document metadata")
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(directory_mode, 0o700);
            assert_eq!(document_mode, 0o600);
            write_editor_document(path, b"Title\n\nBody\n")?;
            Ok(successful_exit())
        };

        edit_document("task", "", "", &TestEnvironment::default(), &mut launcher)
            .expect("edit secure document");
    }

    #[cfg(unix)]
    #[test]
    fn broadened_document_permissions_are_restored_before_reading() {
        let mut launcher = |_command: &EditorCommand, path: &Path| {
            fs::set_permissions(path, fs::Permissions::from_mode(0o644))
                .map_err(|error| PublicError::unexpected(error.to_string()))?;
            write_editor_document(path, b"Title\n\nBody\n")?;
            Ok(successful_exit())
        };

        let document = edit_document(
            "task",
            "Title",
            "",
            &TestEnvironment::default(),
            &mut launcher,
        )
        .expect("restore private permissions");

        assert_eq!(document.body, "Body");
    }

    #[test]
    fn initial_multiline_title_and_oversized_content_are_rejected_before_launch() {
        let launches = Rc::new(RefCell::new(0_u8));
        let launch_count = Rc::clone(&launches);
        let mut launcher = move |_command: &EditorCommand, _path: &Path| {
            *launch_count.borrow_mut() += 1;
            Ok(successful_exit())
        };

        let title_error = edit_document(
            "task",
            "private\nsecond",
            "",
            &TestEnvironment::default(),
            &mut launcher,
        )
        .expect_err("reject multiline title");
        let oversized = "x".repeat(MAX_DOCUMENT_BYTES);
        let size_error = edit_document(
            "note",
            "Title",
            &oversized,
            &TestEnvironment::default(),
            &mut launcher,
        )
        .expect_err("reject oversized initial content");

        assert!(title_error.to_string().contains("line break"));
        assert!(!title_error.to_string().contains("private"));
        assert!(size_error.to_string().contains("safety limit"));
        assert_eq!(*launches.borrow(), 0);
    }

    #[cfg(unix)]
    #[test]
    fn signal_cleanup_worker() {
        const COORDINATION_ENV: &str = "SEALTASK_EDITOR_SIGNAL_TEST_COORDINATION";

        let Some(coordination_path) = env::var_os(COORDINATION_ENV) else {
            return;
        };
        let previous_handlers = EDITOR_TERMINATION_SIGNALS.map(|signal| {
            let action = current_signal_action(signal);
            (action.sa_sigaction, action.sa_flags)
        });
        let coordination_path = PathBuf::from(coordination_path);
        let mut launcher = move |_command: &EditorCommand, document_path: &Path| {
            let workspace_path = document_path
                .parent()
                .expect("editor document must have workspace");
            fs::write(
                &coordination_path,
                workspace_path.as_os_str().as_encoded_bytes(),
            )
            .map_err(|error| PublicError::unexpected(error.to_string()))?;
            let mut child = Command::new("sleep")
                .arg("30")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|error| PublicError::unexpected(error.to_string()))?;
            wait_for_editor_child(&mut child)
        };

        let error = edit_document_with_signal_supervision(
            "task",
            "Private title",
            "Private body",
            &TestEnvironment::default(),
            &mut launcher,
        )
        .expect_err("SIGINT should interrupt editor input");

        assert!(matches!(error, CliError::Interrupted { .. }));
        assert!(error.to_string().contains("SIGINT"));
        assert!(!error.to_string().contains("Private"));
        let restored_handlers = EDITOR_TERMINATION_SIGNALS.map(|signal| {
            let action = current_signal_action(signal);
            (action.sa_sigaction, action.sa_flags)
        });
        assert_eq!(restored_handlers, previous_handlers);
    }

    #[cfg(unix)]
    #[test]
    fn sigint_in_subprocess_removes_plaintext_workspace_before_exit() {
        const COORDINATION_ENV: &str = "SEALTASK_EDITOR_SIGNAL_TEST_COORDINATION";
        const WORKER_TEST: &str = "editor::tests::signal_cleanup_worker";

        let coordination_directory = tempfile::tempdir().expect("coordination temp dir");
        let coordination_path = coordination_directory.path().join("workspace-path");
        let mut worker = Command::new(env::current_exe().expect("current test executable"))
            .arg("--exact")
            .arg(WORKER_TEST)
            .arg("--nocapture")
            .env(COORDINATION_ENV, &coordination_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn signal cleanup worker");

        let readiness_deadline = Instant::now() + Duration::from_secs(10);
        let workspace_path = loop {
            if let Ok(bytes) = fs::read(&coordination_path)
                && !bytes.is_empty()
            {
                let candidate = PathBuf::from(OsString::from_vec(bytes));
                if candidate.exists() {
                    break candidate;
                }
            }
            if let Some(status) = worker.try_wait().expect("poll signal cleanup worker") {
                panic!("signal cleanup worker exited before readiness: {status}");
            }
            if Instant::now() >= readiness_deadline {
                terminate_and_reap_editor(&mut worker);
                panic!("signal cleanup worker did not become ready");
            }
            thread::sleep(Duration::from_millis(10));
        };

        // SAFETY: the PID belongs to the live helper subprocess and SIGINT is a
        // supported termination signal caught by its scoped editor supervisor.
        assert_eq!(
            unsafe { libc::kill(worker.id() as libc::pid_t, libc::SIGINT) },
            0
        );

        let exit_deadline = Instant::now() + Duration::from_secs(10);
        let status = loop {
            if let Some(status) = worker.try_wait().expect("poll interrupted worker") {
                break status;
            }
            if Instant::now() >= exit_deadline {
                terminate_and_reap_editor(&mut worker);
                panic!("signal cleanup worker did not exit");
            }
            thread::sleep(Duration::from_millis(10));
        };

        assert!(status.success(), "signal cleanup worker failed: {status}");
        assert!(
            !workspace_path.exists(),
            "plaintext workspace survived interruption: {}",
            workspace_path.display()
        );
    }

    #[cfg(unix)]
    fn current_signal_action(signal: libc::c_int) -> libc::sigaction {
        // SAFETY: the null action pointer requests a read-only query and
        // `current` is a valid writable output value.
        let mut current = unsafe { std::mem::zeroed::<libc::sigaction>() };
        // SAFETY: `signal` is from the supported signal set and `current`
        // remains writable for the duration of the call.
        let result = unsafe { libc::sigaction(signal, std::ptr::null(), &mut current) };
        assert_eq!(result, 0, "query current action for signal {signal}");
        current
    }
}
