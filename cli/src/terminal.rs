use crate::args::{ColorArg, PagerArg, ProgressArg};
use crate::output::{CliError, CliResult, OutputFormat, terminal_line, write_to_stream};
use anstyle::{AnsiColor, Effects, Style};
use sealtask_client_core::PublicError;
use std::ffi::{OsStr, OsString};
use std::future::Future;
use std::io::{self, IsTerminal, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock, Weak};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use terminal_size::{Height, Width, terminal_size};
use unicode_width::UnicodeWidthStr;
use zeroize::Zeroize;

const DEFAULT_TERMINAL_HEIGHT: usize = 24;
const DEFAULT_TERMINAL_WIDTH: usize = 100;
const PROGRESS_DELAY: Duration = Duration::from_millis(200);
const PROGRESS_INTERVAL: Duration = Duration::from_millis(80);
const PROGRESS_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const CLEAR_LINE: &str = "\r\x1b[2K";

static RUNTIME: OnceLock<Arc<TerminalRuntime>> = OnceLock::new();
static ACTIVE_PROGRESS: Mutex<Option<Weak<ProgressState>>> = Mutex::new(None);
static PROGRESS_IO: Mutex<()> = Mutex::new(());

#[derive(Clone, Copy, Debug)]
pub(crate) struct TerminalOptions {
    pub(crate) color: ColorArg,
    pub(crate) pager: PagerArg,
    pub(crate) pager_explicit: bool,
    pub(crate) no_pager: bool,
    pub(crate) progress: ProgressArg,
    pub(crate) progress_explicit: bool,
    pub(crate) quiet: bool,
    pub(crate) format: OutputFormat,
    pub(crate) pager_allowed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StyleRole {
    Heading,
    Success,
    Warning,
    Error,
    Muted,
    Active,
    Done,
    Archived,
    PriorityHigh,
    PriorityMedium,
    PriorityLow,
    Private,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PagerCommand {
    program: OsString,
    args: Vec<OsString>,
}

#[derive(Clone, Debug)]
struct TerminalPolicy {
    stdout_color: bool,
    stderr_color: bool,
    pager_mode: PagerArg,
    pager_command: Option<PagerCommand>,
    pager_configuration_error: Option<String>,
    pager_allowed: bool,
    stdout_is_terminal: bool,
    terminal_height: usize,
    terminal_width: usize,
    progress_enabled: bool,
    quiet: bool,
    format: OutputFormat,
}

struct TerminalRuntime {
    policy: TerminalPolicy,
    stdout: Mutex<String>,
    capture_stdout: bool,
}

pub(crate) struct TerminalSession {
    runtime: Arc<TerminalRuntime>,
}

pub(crate) struct ProgressGuard {
    state: Option<Arc<ProgressState>>,
    worker: Option<JoinHandle<()>>,
}

struct ProgressState {
    stopped: Mutex<bool>,
    wake: Condvar,
    visible: AtomicBool,
    message: Mutex<String>,
}

#[derive(Clone, Debug)]
struct Environment {
    no_color: bool,
    clicolor_force: bool,
    clicolor_disabled: bool,
    term_is_dumb: bool,
    pager: PagerEnvironment,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum PagerEnvironment {
    Unset,
    Disabled(&'static str),
    Command {
        variable: &'static str,
        value: OsString,
    },
}

#[derive(Clone, Copy, Debug)]
struct TerminalSnapshot {
    stdout_is_terminal: bool,
    stderr_is_terminal: bool,
    terminal_height: usize,
    terminal_width: usize,
}

impl TerminalSession {
    pub(crate) fn start(options: TerminalOptions) -> CliResult<Self> {
        let environment = Environment::read();
        let snapshot = TerminalSnapshot::read();
        let policy = match TerminalPolicy::resolve(options, &environment, snapshot) {
            Ok(policy) => policy,
            Err(error) => {
                let human = options.format == OutputFormat::Table;
                let policy = TerminalPolicy {
                    stdout_color: human
                        && resolve_color(options.color, &environment, snapshot.stdout_is_terminal),
                    stderr_color: human
                        && resolve_color(options.color, &environment, snapshot.stderr_is_terminal),
                    pager_mode: PagerArg::Never,
                    pager_command: None,
                    pager_configuration_error: None,
                    pager_allowed: false,
                    stdout_is_terminal: snapshot.stdout_is_terminal,
                    terminal_height: snapshot.terminal_height,
                    terminal_width: snapshot.terminal_width,
                    progress_enabled: false,
                    quiet: options.quiet,
                    format: options.format,
                };
                let _ = RUNTIME.set(Arc::new(TerminalRuntime {
                    policy,
                    stdout: Mutex::new(String::new()),
                    capture_stdout: false,
                }));
                return Err(error);
            }
        };
        let capture_stdout = policy.format == OutputFormat::Table
            && policy.pager_allowed
            && policy.stdout_is_terminal
            && policy.pager_mode != PagerArg::Never
            && (policy.pager_command.is_some() || policy.pager_configuration_error.is_some());
        let runtime = Arc::new(TerminalRuntime {
            policy,
            stdout: Mutex::new(String::new()),
            capture_stdout,
        });
        RUNTIME.set(Arc::clone(&runtime)).map_err(|_| {
            PublicError::unexpected("terminal output policy was configured more than once")
        })?;
        Ok(Self { runtime })
    }

    pub(crate) fn finish(self) -> CliResult<()> {
        clear_active_progress();
        if !self.runtime.capture_stdout {
            return Ok(());
        }
        let mut output = {
            let mut stdout = lock(&self.runtime.stdout, "terminal stdout buffer")?;
            std::mem::take(&mut *stdout)
        };
        if output.is_empty() {
            return Ok(());
        }
        let result = if self.runtime.policy.should_page(&output) {
            self.runtime.policy.page(&output)
        } else {
            write_direct_stdout(&output)
        };
        output.zeroize();
        result
    }
}

impl TerminalRuntime {
    fn write_stdout(&self, args: std::fmt::Arguments<'_>, newline: bool) -> CliResult<bool> {
        if !self.capture_stdout {
            return Ok(false);
        }
        let mut output = lock(&self.stdout, "terminal stdout buffer")?;
        std::fmt::write(&mut *output, args).map_err(|_| {
            CliError::Public(PublicError::unexpected(
                "failed to format terminal stdout output",
            ))
        })?;
        if newline {
            output.push('\n');
        }
        Ok(true)
    }
}

impl TerminalPolicy {
    fn resolve(
        options: TerminalOptions,
        environment: &Environment,
        snapshot: TerminalSnapshot,
    ) -> CliResult<Self> {
        let human = options.format == OutputFormat::Table;
        let requested_pager_mode = if options.no_pager {
            PagerArg::Never
        } else {
            options.pager
        };
        if options.quiet && requested_pager_mode == PagerArg::Always && options.pager_explicit {
            return Err(
                PublicError::validation("--quiet cannot be combined with --pager always").into(),
            );
        }
        if options.format.is_json()
            && requested_pager_mode == PagerArg::Always
            && options.pager_explicit
        {
            return Err(PublicError::validation(
                "--pager always cannot be used with JSON output; JSON is always written directly",
            )
            .into());
        }
        let pager_mode = if options.quiet
            || options.format.is_json()
            || (!options.pager_allowed && !options.pager_explicit)
        {
            PagerArg::Never
        } else {
            requested_pager_mode
        };
        if options.format.is_json()
            && options.progress == ProgressArg::Always
            && options.progress_explicit
        {
            return Err(PublicError::validation(
                "--progress always cannot be used with JSON output; machine stderr must remain structured",
            )
            .into());
        }
        if options.quiet && options.progress == ProgressArg::Always && options.progress_explicit {
            return Err(PublicError::validation(
                "--quiet cannot be combined with --progress always",
            )
            .into());
        }
        if pager_mode == PagerArg::Always && !options.pager_allowed {
            return Err(PublicError::validation(
                "paging is unavailable for this raw-output command",
            )
            .into());
        }
        if pager_mode == PagerArg::Always && !snapshot.stdout_is_terminal {
            return Err(PublicError::validation(
                "--pager always requires stdout to be a terminal; omit it or use --pager never when redirecting output",
            )
            .into());
        }
        if human
            && !options.quiet
            && options.progress == ProgressArg::Always
            && !snapshot.stderr_is_terminal
        {
            return Err(PublicError::validation(
                "--progress always requires stderr to be a terminal",
            )
            .into());
        }
        let stdout_color =
            human && resolve_color(options.color, environment, snapshot.stdout_is_terminal);
        let stderr_color =
            human && resolve_color(options.color, environment, snapshot.stderr_is_terminal);
        let (pager_command, pager_configuration_error) = if pager_mode == PagerArg::Never
            || !options.pager_allowed
            || !snapshot.stdout_is_terminal
            || !human
        {
            (None, None)
        } else {
            match resolve_pager_command(&environment.pager) {
                Ok(command) => (command, None),
                Err(error) if pager_mode == PagerArg::Auto => (None, Some(error.to_string())),
                Err(error) => return Err(error),
            }
        };
        if pager_mode == PagerArg::Always && pager_command.is_none() {
            let source = match &environment.pager {
                PagerEnvironment::Disabled(variable) => *variable,
                PagerEnvironment::Unset | PagerEnvironment::Command { .. } => {
                    "SEALTASK_PAGER or PAGER"
                }
            };
            return Err(PublicError::validation(format!(
                "--pager always was requested, but paging is disabled by the empty {source} value"
            ))
            .into());
        }
        let progress_enabled = human
            && !options.quiet
            && !environment.term_is_dumb
            && snapshot.stderr_is_terminal
            && match options.progress {
                ProgressArg::Auto => snapshot.stdout_is_terminal,
                ProgressArg::Always => true,
                ProgressArg::Never => false,
            };

        Ok(Self {
            stdout_color,
            stderr_color,
            pager_mode,
            pager_command,
            pager_configuration_error,
            pager_allowed: options.pager_allowed,
            stdout_is_terminal: snapshot.stdout_is_terminal,
            terminal_height: snapshot.terminal_height,
            terminal_width: snapshot.terminal_width,
            progress_enabled,
            quiet: options.quiet,
            format: options.format,
        })
    }

    fn should_page(&self, output: &str) -> bool {
        if self.format != OutputFormat::Table
            || !self.pager_allowed
            || !self.stdout_is_terminal
            || (self.pager_command.is_none() && self.pager_configuration_error.is_none())
        {
            return false;
        }
        match self.pager_mode {
            PagerArg::Always => true,
            PagerArg::Auto => {
                rendered_terminal_rows(output, self.terminal_width)
                    > self.terminal_height.saturating_sub(1)
            }
            PagerArg::Never => false,
        }
    }

    fn page(&self, output: &str) -> CliResult<()> {
        if let Some(error) = self.pager_configuration_error.as_deref() {
            emit_pager_fallback_warning(&format!("{error}; writing directly to stdout"));
            return write_direct_stdout(output);
        }
        let command = self
            .pager_command
            .as_ref()
            .expect("paging eligibility requires a pager command");
        match spawn_pager(command) {
            Ok(mut child) => write_pager_input_and_wait(&mut child, output),
            Err(error) if self.pager_mode == PagerArg::Auto => {
                emit_pager_fallback_warning(&format!(
                    "failed to start pager {}: {error}; writing directly to stdout",
                    command.program.to_string_lossy()
                ));
                write_direct_stdout(output)
            }
            Err(error) => Err(PublicError::unexpected(format!(
                "failed to start pager {}: {error}",
                command.program.to_string_lossy()
            ))
            .into()),
        }
    }
}

impl Environment {
    fn read() -> Self {
        let no_color = nonempty_env("NO_COLOR");
        let clicolor_force = std::env::var("CLICOLOR_FORCE")
            .ok()
            .is_some_and(|value| !value.is_empty() && value != "0");
        let clicolor_disabled = std::env::var("CLICOLOR").ok().as_deref() == Some("0");
        let term_is_dumb = std::env::var("TERM")
            .ok()
            .is_some_and(|value| value.eq_ignore_ascii_case("dumb"));
        let pager = match std::env::var_os("SEALTASK_PAGER") {
            Some(value) if value.is_empty() => PagerEnvironment::Disabled("SEALTASK_PAGER"),
            Some(value) => PagerEnvironment::Command {
                variable: "SEALTASK_PAGER",
                value,
            },
            None => match std::env::var_os("PAGER") {
                Some(value) if value.is_empty() => PagerEnvironment::Disabled("PAGER"),
                Some(value) => PagerEnvironment::Command {
                    variable: "PAGER",
                    value,
                },
                None => PagerEnvironment::Unset,
            },
        };
        Self {
            no_color,
            clicolor_force,
            clicolor_disabled,
            term_is_dumb,
            pager,
        }
    }
}

impl TerminalSnapshot {
    fn read() -> Self {
        let detected = terminal_size();
        Self {
            stdout_is_terminal: io::stdout().is_terminal(),
            stderr_is_terminal: io::stderr().is_terminal(),
            terminal_height: positive_env_usize("LINES")
                .or_else(|| detected.map(|(_, Height(height))| usize::from(height)))
                .filter(|height| *height > 0)
                .unwrap_or(DEFAULT_TERMINAL_HEIGHT),
            terminal_width: positive_env_usize("COLUMNS")
                .or_else(|| detected.map(|(Width(width), _)| usize::from(width)))
                .filter(|width| *width > 0)
                .unwrap_or(DEFAULT_TERMINAL_WIDTH),
        }
    }
}

impl ProgressGuard {
    pub(crate) fn start(label: &'static str) -> Self {
        if !progress_enabled() {
            return Self {
                state: None,
                worker: None,
            };
        }
        clear_active_progress();
        let state = Arc::new(ProgressState {
            stopped: Mutex::new(false),
            wake: Condvar::new(),
            visible: AtomicBool::new(false),
            message: Mutex::new(label.to_string()),
        });
        if let Ok(mut active) = ACTIVE_PROGRESS.lock() {
            *active = Some(Arc::downgrade(&state));
        }
        let worker_state = Arc::clone(&state);
        let worker = thread::Builder::new()
            .name("sealtask-progress".to_string())
            .spawn(move || progress_worker(worker_state))
            .ok();
        Self {
            state: Some(state),
            worker,
        }
    }

    pub(crate) fn set_message(&self, message: &'static str) {
        let Some(state) = self.state.as_ref() else {
            return;
        };
        if let Ok(mut current) = state.message.lock() {
            *current = message.to_string();
        }
        state.wake.notify_all();
    }
}

impl Drop for ProgressGuard {
    fn drop(&mut self) {
        if let Some(state) = self.state.as_ref() {
            stop_and_clear_progress(state);
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        if let Ok(mut active) = ACTIVE_PROGRESS.lock()
            && active
                .as_ref()
                .and_then(Weak::upgrade)
                .is_none_or(|candidate| {
                    self.state
                        .as_ref()
                        .is_some_and(|state| Arc::ptr_eq(&candidate, state))
                })
        {
            *active = None;
        }
    }
}

pub(crate) async fn with_progress<F>(label: &'static str, future: F) -> F::Output
where
    F: Future,
{
    let progress = ProgressGuard::start(label);
    let result = future.await;
    drop(progress);
    result
}

pub(crate) fn clear_active_progress() {
    let state = ACTIVE_PROGRESS
        .lock()
        .ok()
        .and_then(|active| active.as_ref().and_then(Weak::upgrade));
    if let Some(state) = state {
        stop_and_clear_progress(&state);
    }
}

pub(crate) fn stdout_is_terminal() -> bool {
    RUNTIME.get().map_or_else(
        || io::stdout().is_terminal(),
        |runtime| runtime.policy.stdout_is_terminal,
    )
}

pub(crate) fn write_buffered_stdout(
    args: std::fmt::Arguments<'_>,
    newline: bool,
) -> CliResult<bool> {
    RUNTIME
        .get()
        .map_or(Ok(false), |runtime| runtime.write_stdout(args, newline))
}

pub(crate) fn quiet() -> bool {
    RUNTIME.get().is_some_and(|runtime| runtime.policy.quiet)
}

pub(crate) fn style_stdout(value: &str, role: StyleRole) -> String {
    style_text(value, role, stdout_color_enabled())
}

pub(crate) fn style_stdout_explicit(value: &str, role: StyleRole, enabled: bool) -> String {
    style_text(value, role, enabled)
}

pub(crate) fn stdout_color_enabled() -> bool {
    RUNTIME
        .get()
        .is_some_and(|runtime| runtime.policy.stdout_color)
}

pub(crate) fn style_stderr(value: &str, role: StyleRole) -> String {
    style_text(value, role, stderr_color_enabled())
}

pub(crate) fn clap_color_choice(args: &[OsString], format: OutputFormat) -> clap::ColorChoice {
    if format.is_json() {
        return clap::ColorChoice::Never;
    }
    let requested =
        raw_color_arg(args).or_else(|| match std::env::var("SEALTASK_COLOR").as_deref() {
            Ok("always") => Some(ColorArg::Always),
            Ok("never") => Some(ColorArg::Never),
            Ok("auto") => Some(ColorArg::Auto),
            _ => None,
        });
    match requested {
        Some(ColorArg::Always) => clap::ColorChoice::Always,
        Some(ColorArg::Never) => clap::ColorChoice::Never,
        Some(ColorArg::Auto) | None if nonempty_env("NO_COLOR") => clap::ColorChoice::Never,
        Some(ColorArg::Auto) | None => clap::ColorChoice::Auto,
    }
}

pub(crate) fn clap_stdout_color_choice() -> clap::ColorChoice {
    if stdout_color_enabled() {
        clap::ColorChoice::Always
    } else {
        clap::ColorChoice::Never
    }
}

fn raw_color_arg(args: &[OsString]) -> Option<ColorArg> {
    for (index, argument) in args.iter().enumerate().skip(1) {
        if argument == OsStr::new("--") {
            break;
        }
        let value = if argument == OsStr::new("--color") {
            args.get(index + 1).and_then(|value| value.to_str())
        } else {
            argument
                .to_str()
                .and_then(|argument| argument.strip_prefix("--color="))
        };
        match value {
            Some("always") => return Some(ColorArg::Always),
            Some("never") => return Some(ColorArg::Never),
            Some("auto") => return Some(ColorArg::Auto),
            _ => {}
        }
    }
    None
}

fn resolve_color(mode: ColorArg, environment: &Environment, stream_is_terminal: bool) -> bool {
    match mode {
        ColorArg::Always => true,
        ColorArg::Never => false,
        ColorArg::Auto if environment.no_color => false,
        ColorArg::Auto if environment.clicolor_force => true,
        ColorArg::Auto if environment.clicolor_disabled || environment.term_is_dumb => false,
        ColorArg::Auto => stream_is_terminal,
    }
}

fn rendered_terminal_rows(output: &str, terminal_width: usize) -> usize {
    let terminal_width = terminal_width.max(1);
    output
        .lines()
        .map(|line| {
            let plain = strip_csi_sequences(line);
            UnicodeWidthStr::width(plain.as_str())
                .max(1)
                .div_ceil(terminal_width)
        })
        .sum()
}

fn strip_csi_sequences(value: &str) -> String {
    let mut plain = String::with_capacity(value.len());
    let mut characters = value.chars().peekable();
    while let Some(character) = characters.next() {
        if character == '\u{1b}' && characters.peek() == Some(&'[') {
            characters.next();
            for control in characters.by_ref() {
                if ('@'..='~').contains(&control) {
                    break;
                }
            }
        } else {
            plain.push(character);
        }
    }
    plain
}

fn resolve_pager_command(environment: &PagerEnvironment) -> CliResult<Option<PagerCommand>> {
    match environment {
        PagerEnvironment::Disabled(_) => Ok(None),
        PagerEnvironment::Command { variable, value } => {
            let value = value.to_str().ok_or_else(|| {
                PublicError::validation(format!(
                    "{variable} must be valid UTF-8 so its command and arguments can be parsed safely"
                ))
            })?;
            parse_pager_command(value, variable).map(Some)
        }
        PagerEnvironment::Unset => Ok(Some(default_pager_command())),
    }
}

fn parse_pager_command(value: &str, variable: &str) -> CliResult<PagerCommand> {
    let words = shlex::split(value).ok_or_else(|| {
        PublicError::validation(format!(
            "{variable} contains unmatched quoting; set it to a directly executable command such as 'less -R'"
        ))
    })?;
    let mut words = words.into_iter();
    let program = words
        .next()
        .filter(|word| !word.is_empty())
        .ok_or_else(|| {
            PublicError::validation(format!(
                "{variable} must name a pager executable or be empty to disable paging"
            ))
        })?;
    Ok(PagerCommand {
        program: OsString::from(program),
        args: words.map(OsString::from).collect(),
    })
}

#[cfg(unix)]
fn default_pager_command() -> PagerCommand {
    PagerCommand {
        program: OsString::from("less"),
        args: vec![OsString::from("-R")],
    }
}

#[cfg(windows)]
fn default_pager_command() -> PagerCommand {
    PagerCommand {
        program: OsString::from("more.com"),
        args: Vec::new(),
    }
}

fn spawn_pager(command: &PagerCommand) -> io::Result<Child> {
    Command::new(&command.program)
        .args(&command.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
}

fn write_pager_input_and_wait(child: &mut Child, output: &str) -> CliResult<()> {
    let write_result = child.stdin.take().map_or(Ok(()), |mut stdin| {
        stdin
            .write_all(output.as_bytes())
            .and_then(|()| stdin.flush())
    });
    let wait_result = child.wait();
    if let Err(error) = write_result
        && error.kind() != io::ErrorKind::BrokenPipe
    {
        return Err(PublicError::unexpected(format!(
            "failed to write human output to pager stdin: {error}"
        ))
        .into());
    }
    let status = wait_result.map_err(|error| {
        PublicError::unexpected(format!("failed to wait for pager process: {error}"))
    })?;
    if status.success() {
        Ok(())
    } else {
        Err(PublicError::unexpected(format!("pager exited unsuccessfully with {status}")).into())
    }
}

fn write_direct_stdout(output: &str) -> CliResult<()> {
    write_to_stream(
        io::stdout().lock(),
        format_args!("{output}"),
        "print to",
        "stdout",
        true,
    )
}

fn emit_pager_fallback_warning(message: &str) {
    let warning = style_stderr("warning", StyleRole::Warning);
    let detail = terminal_line(message);
    let _ = write_to_stream(
        io::stderr().lock(),
        format_args!("{warning}: {detail}\n"),
        "print to",
        "stderr",
        false,
    );
}

fn progress_enabled() -> bool {
    RUNTIME
        .get()
        .is_some_and(|runtime| runtime.policy.progress_enabled)
}

fn progress_worker(state: Arc<ProgressState>) {
    let Ok(stopped) = state.stopped.lock() else {
        return;
    };
    let Ok((mut stopped, _)) = state
        .wake
        .wait_timeout_while(stopped, PROGRESS_DELAY, |stopped| !*stopped)
    else {
        return;
    };
    if *stopped {
        return;
    }

    let mut frame = 0;
    loop {
        draw_progress(&state, frame);
        frame = (frame + 1) % PROGRESS_FRAMES.len();
        let Ok((next_stopped, _)) = state.wake.wait_timeout(stopped, PROGRESS_INTERVAL) else {
            return;
        };
        stopped = next_stopped;
        if *stopped {
            return;
        }
    }
}

fn draw_progress(state: &ProgressState, frame: usize) {
    let Ok(_io) = PROGRESS_IO.lock() else {
        return;
    };
    let message = state
        .message
        .lock()
        .map(|message| message.clone())
        .unwrap_or_else(|_| "Working…".to_string());
    let spinner = style_stderr(PROGRESS_FRAMES[frame], StyleRole::Muted);
    let mut stderr = io::stderr().lock();
    if write!(stderr, "{CLEAR_LINE}{spinner} {message}")
        .and_then(|()| stderr.flush())
        .is_ok()
    {
        state.visible.store(true, Ordering::Release);
    }
}

fn stop_and_clear_progress(state: &ProgressState) {
    if let Ok(mut stopped) = state.stopped.lock() {
        *stopped = true;
        state.wake.notify_all();
    }
    let Ok(_io) = PROGRESS_IO.lock() else {
        return;
    };
    if state.visible.swap(false, Ordering::AcqRel) {
        let mut stderr = io::stderr().lock();
        let _ = write!(stderr, "{CLEAR_LINE}").and_then(|()| stderr.flush());
    }
}

fn stderr_color_enabled() -> bool {
    RUNTIME
        .get()
        .is_some_and(|runtime| runtime.policy.stderr_color)
}

fn style_text(value: &str, role: StyleRole, enabled: bool) -> String {
    style_text_with(value, style_for(role), enabled)
}

fn style_text_with(value: &str, style: Style, enabled: bool) -> String {
    if !enabled {
        return value.to_string();
    }
    format!("{}{value}{}", style.render(), style.render_reset())
}

fn style_for(role: StyleRole) -> Style {
    match role {
        StyleRole::Heading => Style::new().effects(Effects::BOLD),
        StyleRole::Success | StyleRole::Active => AnsiColor::Green.on_default(),
        StyleRole::Warning | StyleRole::PriorityMedium => AnsiColor::Yellow.on_default(),
        StyleRole::Error | StyleRole::PriorityHigh => {
            AnsiColor::Red.on_default().effects(Effects::BOLD)
        }
        StyleRole::Muted | StyleRole::Archived => Style::new().effects(Effects::DIMMED),
        StyleRole::Done => AnsiColor::Cyan.on_default(),
        StyleRole::PriorityLow => AnsiColor::Blue.on_default(),
        StyleRole::Private => AnsiColor::Magenta.on_default(),
    }
}

fn nonempty_env(name: &str) -> bool {
    std::env::var_os(name).is_some_and(|value| !value.is_empty())
}

fn positive_env_usize(name: &str) -> Option<usize> {
    std::env::var(name)
        .ok()?
        .parse()
        .ok()
        .filter(|value| *value > 0)
}

fn lock<'a, T>(mutex: &'a Mutex<T>, name: &str) -> CliResult<std::sync::MutexGuard<'a, T>> {
    mutex
        .lock()
        .map_err(|_| CliError::Public(PublicError::unexpected(format!("{name} lock was poisoned"))))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn environment() -> Environment {
        Environment {
            no_color: false,
            clicolor_force: false,
            clicolor_disabled: false,
            term_is_dumb: false,
            pager: PagerEnvironment::Unset,
        }
    }

    fn options() -> TerminalOptions {
        TerminalOptions {
            color: ColorArg::Auto,
            pager: PagerArg::Auto,
            pager_explicit: false,
            no_pager: false,
            progress: ProgressArg::Auto,
            progress_explicit: false,
            quiet: false,
            format: OutputFormat::Table,
            pager_allowed: true,
        }
    }

    fn snapshot(stdout_is_terminal: bool, stderr_is_terminal: bool) -> TerminalSnapshot {
        TerminalSnapshot {
            stdout_is_terminal,
            stderr_is_terminal,
            terminal_height: 24,
            terminal_width: 80,
        }
    }

    #[test]
    fn automatic_color_honors_terminal_environment_and_explicit_overrides() {
        let mut env = environment();
        assert!(resolve_color(ColorArg::Auto, &env, true));
        assert!(!resolve_color(ColorArg::Auto, &env, false));

        env.no_color = true;
        assert!(!resolve_color(ColorArg::Auto, &env, true));
        assert!(resolve_color(ColorArg::Always, &env, false));
        assert!(!resolve_color(ColorArg::Never, &env, true));

        env.no_color = false;
        env.clicolor_force = true;
        assert!(resolve_color(ColorArg::Auto, &env, false));
    }

    #[test]
    fn json_policy_disables_all_terminal_decoration() {
        let mut options = options();
        options.format = OutputFormat::Json;
        options.color = ColorArg::Always;
        options.pager = PagerArg::Always;
        options.progress = ProgressArg::Always;
        let policy = TerminalPolicy::resolve(options, &environment(), snapshot(true, true))
            .expect("resolve JSON policy");
        assert!(!policy.stdout_color);
        assert!(!policy.stderr_color);
        assert!(!policy.progress_enabled);
        assert!(!policy.should_page("line\n".repeat(100).as_str()));
    }

    #[test]
    fn explicit_forced_terminal_modes_are_rejected_for_json() {
        let mut pager = options();
        pager.format = OutputFormat::Json;
        pager.pager = PagerArg::Always;
        pager.pager_explicit = true;
        assert!(
            TerminalPolicy::resolve(pager, &environment(), snapshot(true, true))
                .expect_err("reject forced JSON pager")
                .to_string()
                .contains("--pager always")
        );

        let mut progress = options();
        progress.format = OutputFormat::Json;
        progress.progress = ProgressArg::Always;
        progress.progress_explicit = true;
        assert!(
            TerminalPolicy::resolve(progress, &environment(), snapshot(true, true))
                .expect_err("reject forced JSON progress")
                .to_string()
                .contains("--progress always")
        );
    }

    #[test]
    fn automatic_progress_requires_human_output_and_two_terminals() {
        let enabled = TerminalPolicy::resolve(options(), &environment(), snapshot(true, true))
            .expect("resolve enabled policy");
        assert!(enabled.progress_enabled);

        for snapshot in [snapshot(false, true), snapshot(true, false)] {
            let disabled = TerminalPolicy::resolve(options(), &environment(), snapshot)
                .expect("resolve disabled policy");
            assert!(!disabled.progress_enabled);
        }
    }

    #[test]
    fn quiet_suppresses_progress_but_not_primary_output_policy() {
        let mut options = options();
        options.quiet = true;
        let policy = TerminalPolicy::resolve(options, &environment(), snapshot(true, true))
            .expect("resolve quiet policy");
        assert!(!policy.progress_enabled);
        assert!(policy.stdout_color);
    }

    #[test]
    fn automatic_pager_requires_long_human_output_on_a_terminal() {
        let mut policy = TerminalPolicy::resolve(options(), &environment(), snapshot(true, true))
            .expect("resolve pager policy");
        policy.terminal_height = 4;
        policy.terminal_width = 20;
        assert!(!policy.should_page("one\ntwo\nthree\n"));
        assert!(policy.should_page("one\ntwo\nthree\nfour\n"));
        assert!(policy.should_page(&format!("{}\n", "wide".repeat(20))));

        policy.stdout_is_terminal = false;
        assert!(!policy.should_page("one\ntwo\nthree\nfour\n"));
    }

    #[test]
    fn empty_pager_environment_disables_auto_and_explains_forced_paging() {
        let mut env = environment();
        env.pager = PagerEnvironment::Disabled("SEALTASK_PAGER");
        let automatic = TerminalPolicy::resolve(options(), &env, snapshot(true, true))
            .expect("resolve disabled automatic pager");
        assert!(automatic.pager_command.is_none());
        assert!(!automatic.should_page("line\n".repeat(100).as_str()));

        let mut forced = options();
        forced.pager = PagerArg::Always;
        forced.pager_explicit = true;
        let error = TerminalPolicy::resolve(forced, &env, snapshot(true, true))
            .expect_err("forced pager should reject an empty pager command");
        assert!(error.to_string().contains("empty SEALTASK_PAGER"));
    }

    #[test]
    fn raw_composable_output_ignores_environment_paging_but_rejects_explicit_forcing() {
        let mut inherited = options();
        inherited.pager = PagerArg::Always;
        inherited.pager_allowed = false;
        let policy = TerminalPolicy::resolve(inherited, &environment(), snapshot(false, true))
            .expect("environment defaults must not break raw composition");
        assert_eq!(policy.pager_mode, PagerArg::Never);

        let mut explicit = inherited;
        explicit.pager_explicit = true;
        let error = TerminalPolicy::resolve(explicit, &environment(), snapshot(true, true))
            .expect_err("explicit paging must be rejected for raw output");
        assert!(error.to_string().contains("paging is unavailable"));
    }

    #[test]
    fn pager_command_parsing_preserves_argv_without_a_shell() {
        let parsed = parse_pager_command(
            r#""/Applications/My Pager" --flag "two words" ';rm' | evil"#,
            "PAGER",
        )
        .expect("parse pager");
        assert_eq!(parsed.program, OsString::from("/Applications/My Pager"));
        assert_eq!(
            parsed.args,
            ["--flag", "two words", ";rm", "|", "evil",].map(OsString::from)
        );
    }

    #[test]
    fn pager_command_rejects_unmatched_quotes() {
        let error =
            parse_pager_command("less 'unterminated", "PAGER").expect_err("reject malformed pager");
        assert!(error.to_string().contains("unmatched quoting"));
    }

    #[test]
    fn malformed_automatic_pager_is_deferred_until_output_needs_paging() {
        let mut env = environment();
        env.pager = PagerEnvironment::Command {
            variable: "PAGER",
            value: OsString::from("less 'unterminated"),
        };
        let mut policy = TerminalPolicy::resolve(options(), &env, snapshot(true, true))
            .expect("auto pager configuration should be deferred");
        policy.terminal_height = 3;
        assert!(policy.pager_command.is_none());
        assert!(policy.pager_configuration_error.is_some());
        assert!(!policy.should_page("one\ntwo\n"));
        assert!(policy.should_page("one\ntwo\nthree\n"));

        let mut forced = options();
        forced.pager = PagerArg::Always;
        forced.pager_explicit = true;
        assert!(
            TerminalPolicy::resolve(forced, &env, snapshot(true, true))
                .expect_err("forced malformed pager should fail")
                .to_string()
                .contains("unmatched quoting")
        );
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_automatic_pager_is_deferred() {
        use std::os::unix::ffi::OsStringExt;

        let mut env = environment();
        env.pager = PagerEnvironment::Command {
            variable: "PAGER",
            value: OsString::from_vec(vec![0xff]),
        };
        let policy = TerminalPolicy::resolve(options(), &env, snapshot(true, true))
            .expect("non-UTF-8 auto pager should be deferred");
        assert!(policy.pager_command.is_none());
        assert!(policy.pager_configuration_error.is_some());
    }

    #[test]
    fn styled_text_has_an_exact_plain_equivalent() {
        let styled = style_text_with("Active", style_for(StyleRole::Active), true);
        assert!(styled.starts_with("\u{1b}["));
        assert!(styled.ends_with("\u{1b}[0m"));
        assert_eq!(
            style_text_with("Active", style_for(StyleRole::Active), false),
            "Active"
        );
    }

    #[test]
    fn progress_worker_delays_first_draw_and_clears_after_stop() {
        let fast = Arc::new(ProgressState {
            stopped: Mutex::new(false),
            wake: Condvar::new(),
            visible: AtomicBool::new(false),
            message: Mutex::new("Fast operation…".to_string()),
        });
        let fast_worker = {
            let state = Arc::clone(&fast);
            thread::spawn(move || progress_worker(state))
        };
        thread::sleep(Duration::from_millis(30));
        stop_and_clear_progress(&fast);
        fast_worker.join().expect("join fast progress worker");
        assert!(!fast.visible.load(Ordering::Acquire));

        let slow = Arc::new(ProgressState {
            stopped: Mutex::new(false),
            wake: Condvar::new(),
            visible: AtomicBool::new(false),
            message: Mutex::new("Slow operation…".to_string()),
        });
        let slow_worker = {
            let state = Arc::clone(&slow);
            thread::spawn(move || progress_worker(state))
        };
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while !slow.visible.load(Ordering::Acquire) && std::time::Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(slow.visible.load(Ordering::Acquire));
        stop_and_clear_progress(&slow);
        slow_worker.join().expect("join slow progress worker");
        assert!(!slow.visible.load(Ordering::Acquire));
    }

    #[cfg(unix)]
    #[test]
    fn pager_process_receives_content_only_on_stdin() {
        let directory = tempfile::tempdir().expect("pager temp directory");
        let output_path = directory.path().join("pager output.txt");
        let command = PagerCommand {
            program: OsString::from("tee"),
            args: vec![output_path.as_os_str().to_owned()],
        };
        let mut child = spawn_pager(&command).expect("spawn test pager");
        write_pager_input_and_wait(&mut child, "decrypted content\n").expect("write pager input");
        assert_eq!(
            std::fs::read_to_string(output_path).expect("read pager output"),
            "decrypted content\n"
        );
    }
}
