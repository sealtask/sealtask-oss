use crate::output::{CliResult, OutputFormat, write_stderr, write_stderr_line, write_to_stream};
use crate::table::sanitize_cell;
use sealtask_client_core::PublicError;
use std::fs::OpenOptions;
use std::io::{self, BufRead, IsTerminal, Write};

#[cfg(unix)]
const CONTROLLING_TERMINAL: &str = "/dev/tty";
#[cfg(windows)]
const CONTROLLING_TERMINAL: &str = "CONOUT$";

pub(crate) fn write_interaction(
    format: OutputFormat,
    args: std::fmt::Arguments<'_>,
) -> CliResult<()> {
    if format.is_json() {
        return write_controlling_terminal(args, false);
    }
    write_stderr(args)
}

pub(crate) fn write_interaction_line(
    format: OutputFormat,
    args: std::fmt::Arguments<'_>,
) -> CliResult<()> {
    if format.is_json() {
        return write_controlling_terminal(args, true);
    }
    write_stderr_line(args)
}

fn write_controlling_terminal(
    args: std::fmt::Arguments<'_>,
    trailing_newline: bool,
) -> CliResult<()> {
    let terminal = OpenOptions::new()
        .write(true)
        .open(CONTROLLING_TERMINAL)
        .map_err(|err| {
            PublicError::validation(format!(
                "interactive input requires a controlling terminal: {err}; use --non-interactive and the command's explicit stdin flags"
            ))
        })?;
    write_interaction_to(terminal, args, trailing_newline)
}

fn write_interaction_to(
    mut stream: impl Write,
    args: std::fmt::Arguments<'_>,
    trailing_newline: bool,
) -> CliResult<()> {
    write_to_stream(
        &mut stream,
        args,
        "print interactive prompt to",
        "controlling terminal",
        false,
    )?;
    if trailing_newline {
        write_to_stream(
            stream,
            format_args!("\n"),
            "print interactive prompt to",
            "controlling terminal",
            false,
        )?;
    }
    Ok(())
}

pub(crate) fn require_confirmation(
    format: OutputFormat,
    non_interactive: bool,
    confirmed: bool,
    stdin_reserved: bool,
    target: &str,
) -> CliResult<()> {
    if confirmed {
        return Ok(());
    }

    let target = sanitize_cell(target);
    if non_interactive || format.is_json() || stdin_reserved || !io::stdin().is_terminal() {
        return Err(PublicError::validation(format!(
            "permanently deleting {target} requires --yes when prompting is unavailable"
        ))
        .into());
    }

    confirm_from(io::stdin().lock(), io::stderr().lock(), &target)
}

fn confirm_from(mut reader: impl BufRead, writer: impl Write, target: &str) -> CliResult<()> {
    let target = sanitize_cell(target);
    write_to_stream(
        writer,
        format_args!("Permanently delete {target}? [y/N] "),
        "print confirmation prompt to",
        "stderr",
        false,
    )?;
    let mut response = String::new();
    reader
        .read_line(&mut response)
        .map_err(|err| PublicError::unexpected(format!("failed to read confirmation: {err}")))?;
    if matches!(response.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
        return Ok(());
    }

    Err(PublicError::cancelled(format!("permanent deletion of {target} was cancelled")).into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interaction_writer_keeps_terminal_message_contiguous() {
        let mut output = Vec::new();
        write_interaction_to(&mut output, format_args!("Retry in {}s", 3), true)
            .expect("write interaction");
        assert_eq!(output, b"Retry in 3s\n");
    }

    #[test]
    fn explicit_confirmation_never_needs_a_terminal() {
        require_confirmation(OutputFormat::Json, true, true, true, "task 123")
            .expect("explicit confirmation");
    }

    #[test]
    fn machine_deletion_requires_explicit_confirmation() {
        let error = require_confirmation(OutputFormat::Json, false, false, false, "task 123")
            .expect_err("JSON deletion must not prompt");
        assert!(error.to_string().contains("requires --yes"));
    }

    #[test]
    fn interactive_confirmation_accepts_yes_case_insensitively() {
        for response in ["y\n", "YES\n", " Yes \n"] {
            let mut prompt = Vec::new();
            confirm_from(response.as_bytes(), &mut prompt, "task 123")
                .expect("affirmative confirmation");
            assert_eq!(prompt, b"Permanently delete task 123? [y/N] ");
        }
    }

    #[test]
    fn interactive_confirmation_defaults_to_no() {
        for response in ["\n", "n\n", "anything else\n"] {
            let mut prompt = Vec::new();
            let error = confirm_from(response.as_bytes(), &mut prompt, "task 123")
                .expect_err("non-affirmative confirmation");
            assert_eq!(
                error.to_string(),
                "permanent deletion of task 123 was cancelled"
            );
            assert_eq!(prompt, b"Permanently delete task 123? [y/N] ");
        }
    }

    #[test]
    fn confirmation_sanitizes_untrusted_targets_in_prompts_and_cancellation_errors() {
        let target = "task safe\nforged\u{1b}[31m\u{202e}reversed\u{202c}";
        let mut prompt = Vec::new();

        let error = confirm_from(b"n\n".as_slice(), &mut prompt, target)
            .expect_err("confirmation should be declined");

        assert_eq!(
            prompt,
            b"Permanently delete task safe forged[31mreversed? [y/N] "
        );
        assert_eq!(
            error.to_string(),
            "permanent deletion of task safe forged[31mreversed was cancelled"
        );
    }
}
