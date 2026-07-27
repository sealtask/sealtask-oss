use crate::args::{Cli, OutputArg};
use crate::terminal::{self, StyleRole};
use sealtask_client_core::{PublicError, PublicResult};
use serde::Serialize;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::io::{self, Write};

pub(crate) type CliResult<T> = Result<T, CliError>;

#[derive(Debug)]
pub(crate) enum CliError {
    BrokenPipe,
    BatchStatus {
        code: &'static str,
        message: String,
        exit_code: i32,
    },
    Interrupted {
        message: String,
        warnings: Vec<WarningResult>,
        outcome_ambiguous: bool,
        hint: Option<&'static str>,
    },
    Public(PublicError),
    PublicWithWarnings {
        error: PublicError,
        warnings: Vec<WarningResult>,
    },
}

impl std::error::Error for CliError {}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BrokenPipe => write!(f, "broken pipe"),
            Self::BatchStatus { message, .. } => message.fmt(f),
            Self::Interrupted { message, .. } => message.fmt(f),
            Self::Public(error) | Self::PublicWithWarnings { error, .. } => error.fmt(f),
        }
    }
}

impl From<PublicError> for CliError {
    fn from(value: PublicError) -> Self {
        Self::Public(value)
    }
}

impl CliError {
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::BrokenPipe => "broken_pipe",
            Self::BatchStatus { code, .. } => code,
            Self::Interrupted { .. } => "interrupted",
            Self::Public(error) | Self::PublicWithWarnings { error, .. } => error.code(),
        }
    }

    fn warnings(&self) -> &[WarningResult] {
        match self {
            Self::Interrupted { warnings, .. } => warnings,
            Self::PublicWithWarnings { warnings, .. } => warnings,
            _ => &[],
        }
    }

    pub(crate) fn with_warnings(error: PublicError, warnings: &[WarningResult]) -> Self {
        Self::PublicWithWarnings {
            error,
            warnings: warnings.to_vec(),
        }
    }

    pub(crate) fn interrupted(message: impl Into<String>, warnings: &[WarningResult]) -> Self {
        Self::Interrupted {
            message: message.into(),
            warnings: warnings.to_vec(),
            outcome_ambiguous: false,
            hint: None,
        }
    }

    pub(crate) fn interrupted_ambiguous(
        message: impl Into<String>,
        warnings: &[WarningResult],
    ) -> Self {
        Self::Interrupted {
            message: message.into(),
            warnings: warnings.to_vec(),
            outcome_ambiguous: true,
            hint: None,
        }
    }

    pub(crate) fn interrupted_session_ambiguous(
        message: impl Into<String>,
        warnings: &[WarningResult],
    ) -> Self {
        Self::Interrupted {
            message: message.into(),
            warnings: warnings.to_vec(),
            outcome_ambiguous: true,
            hint: Some(
                "The server may have rotated the session; run 'sealtask auth login' if authentication no longer works.",
            ),
        }
    }

    pub(crate) fn batch_partial_failure(message: impl Into<String>) -> Self {
        Self::BatchStatus {
            code: "batch_partial_failure",
            message: message.into(),
            exit_code: 3,
        }
    }

    pub(crate) fn checkpoint_conflict(message: impl Into<String>) -> Self {
        Self::BatchStatus {
            code: "checkpoint_conflict",
            message: message.into(),
            exit_code: 4,
        }
    }

    pub(crate) fn checkpoint_io(message: impl Into<String>) -> Self {
        Self::BatchStatus {
            code: "checkpoint_io",
            message: message.into(),
            exit_code: 4,
        }
    }

    pub(crate) fn exit_code(&self) -> i32 {
        match self {
            Self::Interrupted { .. } => 130,
            Self::BatchStatus { exit_code, .. } => *exit_code,
            _ => 1,
        }
    }

    fn error_result(&self) -> ErrorResult {
        let mut result = ErrorResult {
            code: self.code(),
            message: self.to_string(),
            retryable: false,
            retry_after_seconds: None,
            backend_code: None,
            http_status: None,
            outcome: None,
            hint: error_hint(self.code()),
        };
        match self {
            Self::Public(error) | Self::PublicWithWarnings { error, .. } => {
                result.retryable = public_error_is_retryable(error);
                result.retry_after_seconds = error.retry_after().map(|delay| delay.as_secs());
                result.backend_code = error.backend_error_code().map(str::to_owned);
                result.http_status = error.http_status();
                result.outcome = public_error_outcome(error);
            }
            Self::Interrupted {
                outcome_ambiguous,
                hint,
                ..
            } => {
                if *outcome_ambiguous {
                    result.outcome = Some("ambiguous");
                    result.hint = hint.or(Some(
                        "Inspect the resource before retrying; the mutation may have committed.",
                    ));
                } else {
                    result.outcome = Some("interrupted");
                }
            }
            Self::BatchStatus { .. } => {}
            Self::BrokenPipe => {}
        }
        result
    }
}

macro_rules! print {
    () => {
        $crate::output::write_stdout(format_args!(""))?
    };
    ($($arg:tt)*) => {
        $crate::output::write_stdout(format_args!($($arg)*))?
    };
}

macro_rules! println {
    () => {
        $crate::output::write_stdout_line(format_args!(""))?
    };
    ($($arg:tt)*) => {
        $crate::output::write_stdout_line(format_args!($($arg)*))?
    };
}

pub(crate) fn write_stdout(args: fmt::Arguments<'_>) -> CliResult<()> {
    if terminal::write_buffered_stdout(args, false)? {
        return Ok(());
    }
    write_to_stream(io::stdout().lock(), args, "print to", "stdout", true)
}

pub(crate) fn write_stdout_line(args: fmt::Arguments<'_>) -> CliResult<()> {
    if terminal::write_buffered_stdout(args, true)? {
        return Ok(());
    }
    write_line_to_stream(io::stdout().lock(), args, "print to", "stdout", true)
}

pub(crate) fn write_stdout_line_flushed(args: fmt::Arguments<'_>) -> CliResult<()> {
    let mut stdout = io::stdout().lock();
    write_line_to_stream(&mut stdout, args, "print to", "stdout", true)?;
    stdout
        .flush()
        .map_err(|err| map_stream_error(err, "flush", "stdout", true))
}

pub(crate) fn write_stdout_flushed(args: fmt::Arguments<'_>) -> CliResult<()> {
    let mut stdout = io::stdout().lock();
    write_to_stream(&mut stdout, args, "print to", "stdout", true)?;
    stdout
        .flush()
        .map_err(|err| map_stream_error(err, "flush", "stdout", true))
}

pub(crate) fn write_stderr_line(args: fmt::Arguments<'_>) -> CliResult<()> {
    terminal::clear_active_progress();
    write_line_to_stream(io::stderr().lock(), args, "print to", "stderr", false)
}

pub(crate) fn write_stderr(args: fmt::Arguments<'_>) -> CliResult<()> {
    terminal::clear_active_progress();
    write_to_stream(io::stderr().lock(), args, "print to", "stderr", false)
}

pub(crate) fn write_to_stream<W: Write>(
    mut stream: W,
    args: fmt::Arguments<'_>,
    action: &str,
    stream_name: &str,
    broken_pipe_is_success: bool,
) -> CliResult<()> {
    stream
        .write_fmt(args)
        .map_err(|err| map_stream_error(err, action, stream_name, broken_pipe_is_success))
}

fn write_line_to_stream<W: Write>(
    mut stream: W,
    args: fmt::Arguments<'_>,
    action: &str,
    stream_name: &str,
    broken_pipe_is_success: bool,
) -> CliResult<()> {
    stream
        .write_fmt(args)
        .map_err(|err| map_stream_error(err, action, stream_name, broken_pipe_is_success))?;
    stream
        .write_all(b"\n")
        .map_err(|err| map_stream_error(err, action, stream_name, broken_pipe_is_success))
}

fn map_stream_error(
    err: io::Error,
    action: &str,
    stream_name: &str,
    broken_pipe_is_success: bool,
) -> CliError {
    if broken_pipe_is_success && err.kind() == io::ErrorKind::BrokenPipe {
        CliError::BrokenPipe
    } else {
        CliError::Public(PublicError::unexpected(format!(
            "failed to {action} {stream_name}: {err}"
        )))
    }
}

pub(crate) fn print_json<T: Serialize + ?Sized>(
    value: &T,
    format: OutputFormat,
    context: &str,
) -> CliResult<()> {
    let output = if format.pretty_json() {
        serde_json::to_string_pretty(value)
    } else {
        serde_json::to_string(value)
    }
    .expect(context);
    println!("{output}");
    Ok(())
}

pub(crate) fn print_jsonl<T: Serialize + ?Sized>(value: &T, context: &str) -> CliResult<()> {
    let output = serde_json::to_string(value).expect(context);
    write_stdout_line_flushed(format_args!("{output}"))
}

fn print_json_stderr<T: Serialize + ?Sized>(
    value: &T,
    format: OutputFormat,
    context: &str,
) -> CliResult<()> {
    let output = if format.pretty_json() {
        serde_json::to_string_pretty(value)
    } else {
        serde_json::to_string(value)
    }
    .expect(context);
    write_stderr_line(format_args!("{output}"))
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum OutputFormat {
    Table,
    Json,
    JsonPretty,
    Jsonl,
}

impl OutputFormat {
    #[must_use]
    pub(crate) fn from_raw_args(args: &[OsString]) -> Self {
        let mut detected = Self::Table;
        for (index, arg) in args.iter().enumerate() {
            if arg == OsStr::new("--json") {
                return Self::Json;
            }
            if arg == OsStr::new("--format")
                && let Some(value) = args.get(index + 1).and_then(|value| value.to_str())
            {
                detected = Self::from_output_value(value);
            }
            if let Some(value) = arg.to_str().and_then(|arg| arg.strip_prefix("--format=")) {
                detected = Self::from_output_value(value);
            }
        }
        detected
    }

    #[must_use]
    pub(crate) fn from_cli(cli: &Cli) -> Self {
        if cli.json {
            return Self::Json;
        }
        match cli.format {
            Some(OutputArg::Json) => Self::Json,
            Some(OutputArg::JsonPretty) => Self::JsonPretty,
            Some(OutputArg::Jsonl) => Self::Jsonl,
            Some(OutputArg::Table) | None => Self::Table,
        }
    }

    #[must_use]
    pub(crate) const fn is_json(self) -> bool {
        matches!(self, Self::Json | Self::JsonPretty | Self::Jsonl)
    }

    const fn pretty_json(self) -> bool {
        matches!(self, Self::JsonPretty)
    }

    fn from_output_value(value: &str) -> Self {
        match value {
            "json" => Self::Json,
            "json-pretty" => Self::JsonPretty,
            "jsonl" => Self::Jsonl,
            _ => Self::Table,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorResult {
    code: &'static str,
    message: String,
    retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    retry_after_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    backend_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    http_status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    outcome: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hint: Option<&'static str>,
}

#[derive(Debug, Serialize)]
struct StderrEnvelope<'a> {
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    warnings: &'a [WarningResult],
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<ErrorResult>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WarningResult {
    code: &'static str,
    message: String,
}

impl WarningResult {
    #[cfg(test)]
    pub(crate) fn code(&self) -> &'static str {
        self.code
    }

    #[cfg(test)]
    pub(crate) fn message(&self) -> &str {
        &self.message
    }
}

pub(crate) fn print_cli_error(err: &CliError, format: OutputFormat) -> CliResult<()> {
    match format {
        OutputFormat::Table => {
            print_warnings(format, err.warnings())?;
            write_table_error(io::stderr().lock(), err)
        }
        OutputFormat::Json | OutputFormat::JsonPretty | OutputFormat::Jsonl => {
            print_json_stderr_envelope(
                format,
                err.warnings(),
                Some(err.error_result()),
                "serializing CLI error should succeed",
            )
        }
    }
}

fn write_table_error<W: Write>(mut stream: W, err: &CliError) -> CliResult<()> {
    let result = err.error_result();
    let label = terminal::style_stderr("error", StyleRole::Error);
    write_line_to_stream(
        &mut stream,
        format_args!(
            "{label} [{}]: {}",
            result.code,
            terminal_line(&result.message)
        ),
        "print to",
        "stderr",
        false,
    )?;
    if let Some(retry_after_seconds) = result.retry_after_seconds {
        write_line_to_stream(
            &mut stream,
            format_args!("retry after: {retry_after_seconds}s"),
            "print to",
            "stderr",
            false,
        )?;
    }
    if let Some(outcome) = result.outcome {
        write_line_to_stream(
            &mut stream,
            format_args!("outcome: {}", terminal_line(outcome)),
            "print to",
            "stderr",
            false,
        )?;
    }
    if let Some(hint) = result.hint {
        write_line_to_stream(
            stream,
            format_args!("hint: {}", terminal_line(hint)),
            "print to",
            "stderr",
            false,
        )?;
    }
    Ok(())
}

pub(crate) fn print_clap_error(err: &clap::Error, format: OutputFormat) -> CliResult<()> {
    print_json_error(
        format,
        "validation",
        err.to_string().trim_end().to_string(),
        "serializing clap parse error should succeed",
    )
}

fn print_json_error(
    format: OutputFormat,
    code: &'static str,
    message: String,
    context: &str,
) -> CliResult<()> {
    print_json_stderr_envelope(
        format,
        &[],
        Some(ErrorResult {
            code,
            message,
            retryable: false,
            retry_after_seconds: None,
            backend_code: None,
            http_status: None,
            outcome: None,
            hint: error_hint(code),
        }),
        context,
    )
}

fn print_warnings(format: OutputFormat, warnings: &[WarningResult]) -> CliResult<()> {
    write_warnings(io::stderr().lock(), format, warnings)
}

pub(crate) fn emit_warnings_best_effort(format: OutputFormat, warnings: &[WarningResult]) {
    let _ = print_warnings(format, warnings);
}

pub(crate) fn finish_with_warnings<T>(
    format: OutputFormat,
    warnings: &[WarningResult],
    result: CliResult<T>,
) -> CliResult<T> {
    match result {
        Ok(value) => {
            emit_warnings_best_effort(format, warnings);
            Ok(value)
        }
        Err(CliError::BrokenPipe) => Err(CliError::BrokenPipe),
        Err(error @ CliError::BatchStatus { .. }) => Err(error),
        Err(CliError::Interrupted {
            message,
            warnings: existing,
            outcome_ambiguous,
            hint,
        }) => {
            let mut combined = warnings.to_vec();
            combined.extend(existing);
            Err(CliError::Interrupted {
                message,
                warnings: combined,
                outcome_ambiguous,
                hint,
            })
        }
        Err(CliError::Public(error)) => Err(CliError::with_warnings(error, warnings)),
        Err(CliError::PublicWithWarnings {
            error,
            warnings: existing,
        }) => {
            let mut combined = warnings.to_vec();
            combined.extend(existing);
            Err(CliError::PublicWithWarnings {
                error,
                warnings: combined,
            })
        }
    }
}

#[cfg(test)]
fn emit_warnings_best_effort_to<W: Write>(
    stream: W,
    format: OutputFormat,
    warnings: &[WarningResult],
) {
    let _ = write_warnings(stream, format, warnings);
}

fn write_warnings<W: Write>(
    mut stream: W,
    format: OutputFormat,
    warnings: &[WarningResult],
) -> CliResult<()> {
    if warnings.is_empty() {
        return Ok(());
    }

    match format {
        OutputFormat::Table => {
            for warning in warnings {
                let label = terminal::style_stderr("warning", StyleRole::Warning);
                write_line_to_stream(
                    &mut stream,
                    format_args!("{label}: {}", terminal_line(&warning.message)),
                    "print to",
                    "stderr",
                    false,
                )?;
            }
            Ok(())
        }
        OutputFormat::Json | OutputFormat::JsonPretty | OutputFormat::Jsonl => {
            let envelope = StderrEnvelope {
                warnings,
                error: None,
            };
            let output = if format.pretty_json() {
                serde_json::to_string_pretty(&envelope)
            } else {
                serde_json::to_string(&envelope)
            }
            .expect("serializing CLI warnings should succeed");
            write_line_to_stream(
                stream,
                format_args!("{output}"),
                "print to",
                "stderr",
                false,
            )
        }
    }
}

pub(crate) fn terminal_line(value: &str) -> String {
    sanitize_terminal_text(value, false)
}

pub(crate) fn terminal_block(value: &str) -> String {
    sanitize_terminal_text(value, true)
}

fn sanitize_terminal_text(value: &str, preserve_newlines: bool) -> String {
    value
        .chars()
        .filter_map(|ch| match ch {
            '\n' if preserve_newlines => Some('\n'),
            '\r' if preserve_newlines => None,
            ch if ch.is_whitespace() => Some(' '),
            ch if ch.is_control() || is_bidi_control(ch) => None,
            ch => Some(ch),
        })
        .collect()
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

fn print_json_stderr_envelope(
    format: OutputFormat,
    warnings: &[WarningResult],
    error: Option<ErrorResult>,
    context: &str,
) -> CliResult<()> {
    print_json_stderr(&StderrEnvelope { warnings, error }, format, context)
}

pub(crate) fn public_result_with_warnings<T>(
    result: PublicResult<T>,
    warnings: &[WarningResult],
) -> CliResult<T> {
    result.map_err(|err| CliError::with_warnings(err, warnings))
}

pub(crate) fn warning_result(code: &'static str, message: String) -> WarningResult {
    WarningResult { code, message }
}

pub(crate) fn print_simple_result<T: Serialize + ?Sized>(
    format: OutputFormat,
    payload: &T,
    context: &str,
    table_message: &str,
) -> CliResult<()> {
    match format {
        OutputFormat::Json | OutputFormat::JsonPretty | OutputFormat::Jsonl => {
            print_json(payload, format, context)
        }
        OutputFormat::Table => {
            if !terminal::quiet() {
                println!(
                    "{}",
                    terminal::style_stdout(table_message, StyleRole::Success)
                );
            }
            Ok(())
        }
    }
}

pub(crate) fn mutation_output_enabled(format: OutputFormat) -> bool {
    format.is_json() || !terminal::quiet()
}

fn public_error_is_retryable(error: &PublicError) -> bool {
    matches!(
        error.code(),
        "rate_limited"
            | "request_timeout"
            | "transport_timeout"
            | "transport_connect"
            | "transport_body"
            | "transport_other"
            | "response_body_read"
            | "response_body_truncated"
            | "response_transport"
            | "http_server_error"
    )
}

fn public_error_outcome(error: &PublicError) -> Option<&'static str> {
    match error {
        PublicError::CompensationFailed { .. } => Some("cleanup_failed"),
        PublicError::OutcomeAmbiguous { .. } => Some("ambiguous"),
        PublicError::CommittedButLocalProcessingFailed { .. } => Some("committed"),
        PublicError::Cancelled(_) => Some("cancelled"),
        _ => None,
    }
}

fn error_hint(code: &str) -> Option<&'static str> {
    match code {
        "authentication" => Some("Run 'sealtask auth login' and retry."),
        "mfa_input_required" => {
            Some("Provide the authenticator or backup code on login stdin line 2.")
        }
        "conflict" => Some("Re-read the resource, reconcile the latest state, and retry."),
        "rate_limited" => Some("Wait for retryAfterSeconds when present before retrying."),
        "request_timeout" => Some("Retry the command after the blocking condition clears."),
        "outcome_ambiguous" => {
            Some("Inspect the resource before retrying; the mutation may have committed.")
        }
        "committed_but_local_processing_failed" => {
            Some("Fetch the committed resource instead of repeating the mutation.")
        }
        "checkpoint_conflict" => Some(
            "Use the exact original input and checkpoint; inspect ownership and permissions before changing either file.",
        ),
        "checkpoint_io" => Some(
            "Inspect checkpoint disk space and permissions; do not delete it until in-flight mutation state is understood.",
        ),
        "batch_partial_failure" => Some(
            "Inspect streamed operation failures, then resume with the exact same input and checkpoint.",
        ),
        "validation" => Some("Review command help and the rejected input field."),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct AlwaysFailWriter;

    #[test]
    fn raw_json_detection_survives_an_invalid_or_conflicting_format_flag() {
        let invalid_then_json = [
            OsString::from("sealtask"),
            OsString::from("--format"),
            OsString::from("invalid"),
            OsString::from("--json"),
        ];
        let pretty_then_json = [
            OsString::from("sealtask"),
            OsString::from("--format=json-pretty"),
            OsString::from("--json"),
        ];

        assert_eq!(
            OutputFormat::from_raw_args(&invalid_then_json),
            OutputFormat::Json
        );
        assert_eq!(
            OutputFormat::from_raw_args(&pretty_then_json),
            OutputFormat::Json
        );
    }

    #[test]
    fn test_should_delegate_public_error_classification_to_core() {
        let errors = [
            PublicError::validation("message"),
            PublicError::conflict("message"),
            PublicError::entitlement("message"),
            PublicError::payload_too_large("message"),
            PublicError::rate_limited("message"),
            PublicError::request_timeout("retry the request"),
            PublicError::crypto("message"),
            PublicError::unexpected("message"),
            PublicError::cancelled("message"),
            PublicError::compensation_failed("operation", "primary", "cleanup"),
            PublicError::outcome_ambiguous("operation", "details"),
            PublicError::mfa_required_use_begin_login(),
            PublicError::mfa_input_required(),
        ];

        for error in errors {
            let expected = error.code();
            assert_eq!(CliError::from(error).code(), expected);
        }
    }

    #[test]
    fn request_timeout_keeps_stable_json_code_and_retry_guidance() {
        let error = CliError::from(PublicError::request_timeout(
            "request body timed out before execution; retry the request",
        ));
        let result = error.error_result();

        assert_eq!(result.code, "request_timeout");
        assert_eq!(
            result.message,
            "request body timed out before execution; retry the request"
        );
        assert!(result.retryable);
        assert!(result.hint.is_some());
    }

    #[test]
    fn machine_errors_expose_retry_transport_and_outcome_metadata() {
        let rate_limited = CliError::from(PublicError::rate_limited_with_retry_after(
            "retry later",
            std::time::Duration::from_secs(17),
        ))
        .error_result();
        assert!(rate_limited.retryable);
        assert_eq!(rate_limited.retry_after_seconds, Some(17));
        assert_eq!(
            rate_limited.hint,
            Some("Wait for retryAfterSeconds when present before retrying.")
        );

        let http = CliError::from(PublicError::http(
            409,
            Some("revision_mismatch".to_string()),
            None,
        ))
        .error_result();
        assert_eq!(http.http_status, Some(409));
        assert_eq!(http.backend_code.as_deref(), Some("revision_mismatch"));
        assert!(!http.retryable);

        let server = CliError::from(PublicError::http(503, None, None)).error_result();
        assert_eq!(server.http_status, Some(503));
        assert!(server.retryable);

        let ambiguous = CliError::from(PublicError::outcome_ambiguous(
            "create task",
            "unknown commit",
        ))
        .error_result();
        assert_eq!(ambiguous.outcome, Some("ambiguous"));
        assert!(!ambiguous.retryable);
    }

    #[test]
    fn exhausted_replay_safe_transient_failures_remain_retryable_in_output() {
        for kind in [
            sealtask_client_core::ResponseFailureKind::BodyRead,
            sealtask_client_core::ResponseFailureKind::BodyTruncated,
            sealtask_client_core::ResponseFailureKind::Transport,
        ] {
            let result =
                CliError::from(PublicError::response(kind, "response failed")).error_result();
            assert!(result.retryable, "{} should be retryable", result.code);
        }
        assert!(
            CliError::from(PublicError::http(503, None, None))
                .error_result()
                .retryable
        );
    }

    impl Write for AlwaysFailWriter {
        fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "stderr closed"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_should_classify_broken_pipe_stdout_errors_separately() {
        let error = io::Error::new(io::ErrorKind::BrokenPipe, "stdout closed");
        assert!(matches!(
            map_stream_error(error, "print to", "stdout", true),
            CliError::BrokenPipe
        ));
    }

    #[test]
    fn test_should_map_non_broken_pipe_stdout_errors_to_public_errors() {
        let error = io::Error::other("disk exploded");
        assert!(matches!(
            map_stream_error(error, "print to", "stdout", true),
            CliError::Public(PublicError::Unexpected(message))
                if message.contains("failed to print to stdout: disk exploded")
        ));
    }

    #[test]
    fn test_should_keep_broken_pipe_stderr_errors_as_failures() {
        let error = io::Error::new(io::ErrorKind::BrokenPipe, "stderr closed");
        assert!(matches!(
            map_stream_error(error, "print to", "stderr", false),
            CliError::Public(PublicError::Unexpected(message))
                if message.contains("failed to print to stderr: stderr closed")
        ));
    }

    #[test]
    fn test_should_ignore_stderr_failures_during_best_effort_warning_emission() {
        let warnings = [warning_result(
            "logout_revoke_failed",
            "failed to revoke token on server: stderr closed".to_string(),
        )];

        assert!(write_warnings(AlwaysFailWriter, OutputFormat::Json, &warnings).is_err());
        emit_warnings_best_effort_to(AlwaysFailWriter, OutputFormat::Json, &warnings);
    }

    #[test]
    fn test_should_attach_deferred_warnings_to_stdout_failures() {
        let warnings = [warning_result(
            "logout_revoke_failed",
            "failed to revoke token on server".to_string(),
        )];
        let print_result: CliResult<()> =
            Err(PublicError::unexpected("failed to print to stdout").into());

        let error = finish_with_warnings(OutputFormat::Json, &warnings, print_result)
            .expect_err("stdout failure must retain deferred warnings");

        assert!(matches!(
            error,
            CliError::PublicWithWarnings { warnings, .. }
                if warnings.len() == 1 && warnings[0].code == "logout_revoke_failed"
        ));
    }

    #[test]
    fn test_should_sanitize_table_errors_and_warnings_at_the_output_boundary() {
        let message =
            "attachment '\u{1b}]8;;https://example.test\u{7}name\u{1b}]8;;\u{7}'\nnext row";
        let error = CliError::from(PublicError::validation(message));
        let mut error_output = Vec::new();
        write_table_error(&mut error_output, &error).expect("write table error");
        let error_output = String::from_utf8(error_output).expect("error output UTF-8");
        assert_eq!(
            error_output,
            concat!(
                "error [validation]: attachment ']8;;https://example.testname]8;;' next row\n",
                "hint: Review command help and the rejected input field.\n"
            )
        );

        let warnings = [warning_result("unsafe_warning", message.to_string())];
        let mut warning_output = Vec::new();
        write_warnings(&mut warning_output, OutputFormat::Table, &warnings)
            .expect("write table warning");
        let warning_output = String::from_utf8(warning_output).expect("warning output UTF-8");
        assert_eq!(
            warning_output,
            "warning: attachment ']8;;https://example.testname]8;;' next row\n"
        );
    }

    #[test]
    fn test_should_preserve_warning_text_exactly_in_json_output() {
        let message = "unsafe\u{1b}]8;;url\u{7}label\nnext row";
        let warnings = [warning_result("unsafe_warning", message.to_string())];
        let mut output = Vec::new();
        write_warnings(&mut output, OutputFormat::Json, &warnings).expect("write JSON warning");
        let document: serde_json::Value =
            serde_json::from_slice(&output).expect("parse JSON warning output");
        assert_eq!(document["warnings"][0]["message"], message);
    }

    #[test]
    fn interrupted_outcome_uses_exit_130_and_json_error_with_warnings() {
        let warnings = [warning_result(
            "attachment_upload_cancellation_timed_out",
            "cleanup timed out".to_string(),
        )];
        let error = CliError::interrupted("attachment upload interrupted", &warnings);

        assert_eq!(error.exit_code(), 130);
        assert_eq!(error.code(), "interrupted");
        let document = serde_json::to_value(StderrEnvelope {
            warnings: error.warnings(),
            error: Some(error.error_result()),
        })
        .expect("JSON error envelope");
        assert_eq!(document["error"]["code"], "interrupted");
        assert_eq!(document["error"]["outcome"], "interrupted");
        assert_eq!(
            document["warnings"][0]["code"],
            "attachment_upload_cancellation_timed_out"
        );
    }

    #[test]
    fn forced_mutation_interruption_reports_an_ambiguous_outcome() {
        let error =
            CliError::interrupted_ambiguous("stopped before a definitive mutation response", &[]);
        let result = error.error_result();

        assert_eq!(error.exit_code(), 130);
        assert_eq!(result.code, "interrupted");
        assert_eq!(result.outcome, Some("ambiguous"));
        assert!(
            result
                .hint
                .is_some_and(|hint| hint.contains("may have committed"))
        );
    }

    #[test]
    fn forced_credential_refresh_interruption_uses_session_recovery_guidance() {
        let error =
            CliError::interrupted_session_ambiguous("session rotation may be ambiguous", &[]);
        let result = error.error_result();

        assert_eq!(result.outcome, Some("ambiguous"));
        assert!(result.hint.is_some_and(|hint| hint.contains("auth login")));
        assert!(
            result
                .hint
                .is_none_or(|hint| !hint.contains("resource") && !hint.contains("mutation"))
        );
    }

    #[test]
    fn test_should_prevent_terminal_lines_from_injecting_controls_or_extra_rows() {
        assert_eq!(
            terminal_line(
                "safe\nnext\rrow\tcell\u{2028}more\u{1b}[2J\u{009b}31m\u{202e}spoof\u{202c}\u{2066}isolate\u{2069}"
            ),
            "safe next row cell more[2J31mspoofisolate"
        );
    }

    #[test]
    fn test_should_preserve_block_line_feeds_while_stripping_terminal_controls() {
        assert_eq!(
            terminal_block("first\r\nsecond\tcell\u{1b}[31m"),
            "first\nsecond cell[31m"
        );
    }
}
