use serde::Serialize;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::io::{self, Write};
use worklist_client_core::{PublicError, PublicResult};

pub(crate) type CliResult<T> = Result<T, CliError>;

#[derive(Debug)]
pub(crate) enum CliError {
    BrokenPipe,
    Public(PublicError),
    PublicWithWarnings {
        error: PublicError,
        warnings: Vec<WarningResult>,
    },
}

impl std::error::Error for CliError {}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(err) = self.public_error() {
            err.fmt(f)
        } else {
            write!(f, "broken pipe")
        }
    }
}

impl From<PublicError> for CliError {
    fn from(value: PublicError) -> Self {
        Self::Public(value)
    }
}

impl CliError {
    fn public_error(&self) -> Option<&PublicError> {
        match self {
            Self::BrokenPipe => None,
            Self::Public(err) | Self::PublicWithWarnings { error: err, .. } => Some(err),
        }
    }

    fn code(&self) -> &'static str {
        match self.public_error() {
            None => "broken_pipe",
            Some(PublicError::Validation(_)) => "validation",
            Some(PublicError::Conflict(_)) => "conflict",
            Some(PublicError::Crypto(_)) => "crypto",
            Some(PublicError::Unexpected(_)) => "unexpected",
            Some(PublicError::MfaRequiredUseBeginLogin) => "mfa_required_use_begin_login",
            Some(PublicError::MfaInputRequired) => "mfa_input_required",
        }
    }

    fn warnings(&self) -> &[WarningResult] {
        match self {
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

    fn error_result(&self) -> ErrorResult {
        ErrorResult {
            code: self.code(),
            message: self.to_string(),
        }
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
    write_to_stream(io::stdout().lock(), args, "print to", "stdout", true)
}

pub(crate) fn write_stdout_line(args: fmt::Arguments<'_>) -> CliResult<()> {
    write_line_to_stream(io::stdout().lock(), args, "print to", "stdout", true)
}

pub(crate) fn write_stderr_line(args: fmt::Arguments<'_>) -> CliResult<()> {
    write_line_to_stream(io::stderr().lock(), args, "print to", "stderr", false)
}

pub(crate) fn flush_stdout() -> CliResult<()> {
    io::stdout()
        .lock()
        .flush()
        .map_err(|err| map_stream_error(err, "flush", "stdout", true))
}

fn write_to_stream<W: Write>(
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

pub(crate) fn print_pretty_json<T: Serialize + ?Sized>(value: &T, context: &str) -> CliResult<()> {
    let output = serde_json::to_string_pretty(value).expect(context);
    println!("{output}");
    Ok(())
}

fn print_pretty_json_stderr<T: Serialize + ?Sized>(value: &T, context: &str) -> CliResult<()> {
    let output = serde_json::to_string_pretty(value).expect(context);
    write_stderr_line(format_args!("{output}"))
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum OutputFormat {
    Table,
    Json,
}

impl OutputFormat {
    #[must_use]
    pub(crate) fn from_raw_args(args: &[OsString]) -> Self {
        if args.iter().any(|arg| arg == OsStr::new("--json")) {
            Self::Json
        } else {
            Self::Table
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorResult {
    code: &'static str,
    message: String,
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

pub(crate) fn print_cli_error(err: &CliError, format: OutputFormat) -> CliResult<()> {
    match format {
        OutputFormat::Table => {
            print_warnings(format, err.warnings())?;
            write_table_error(io::stderr().lock(), err)
        }
        OutputFormat::Json => print_json_stderr(
            err.warnings(),
            Some(err.error_result()),
            "serializing CLI error should succeed",
        ),
    }
}

fn write_table_error<W: Write>(mut stream: W, err: &CliError) -> CliResult<()> {
    write_line_to_stream(
        &mut stream,
        format_args!("error: {}", terminal_line(&err.to_string())),
        "print to",
        "stderr",
        false,
    )
}

pub(crate) fn print_clap_error(err: &clap::Error) -> CliResult<()> {
    print_json_error(
        "validation",
        err.to_string().trim_end().to_string(),
        "serializing clap parse error should succeed",
    )
}

fn print_json_error(code: &'static str, message: String, context: &str) -> CliResult<()> {
    print_json_stderr(&[], Some(ErrorResult { code, message }), context)
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
                write_line_to_stream(
                    &mut stream,
                    format_args!("warning: {}", terminal_line(&warning.message)),
                    "print to",
                    "stderr",
                    false,
                )?;
            }
            Ok(())
        }
        OutputFormat::Json => {
            let output = serde_json::to_string_pretty(&StderrEnvelope {
                warnings,
                error: None,
            })
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
            ch if ch.is_control() => None,
            ch => Some(ch),
        })
        .collect()
}

fn print_json_stderr(
    warnings: &[WarningResult],
    error: Option<ErrorResult>,
    context: &str,
) -> CliResult<()> {
    print_pretty_json_stderr(&StderrEnvelope { warnings, error }, context)
}

pub(crate) fn require_password_stdin_for_json_command(
    format: OutputFormat,
    password_stdin: bool,
    command_name: &str,
) -> CliResult<()> {
    if format == OutputFormat::Json && !password_stdin {
        return Err(PublicError::validation(format!(
            "--json {command_name} requires --password-stdin"
        ))
        .into());
    }

    Ok(())
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
        OutputFormat::Json => print_pretty_json(payload, context),
        OutputFormat::Table => {
            println!("{table_message}");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct AlwaysFailWriter;

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
            "error: attachment ']8;;https://example.testname]8;;' next row\n"
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
    fn test_should_prevent_terminal_lines_from_injecting_controls_or_extra_rows() {
        assert_eq!(
            terminal_line("safe\nnext\rrow\tcell\u{2028}more\u{1b}[2J\u{009b}31m"),
            "safe next row cell more[2J31m"
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
