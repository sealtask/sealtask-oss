use crate::args::{NoteCreateArgsCli, NoteUpdateArgsCli, TaskCreateArgsCli, TaskUpdateArgsCli};
use crate::interaction::write_interaction;
use crate::output::CliResult;
use sealtask_client_core::{PublicError, PublicResult};
use sealtask_client_runtime::{
    CommentInput, NoteCreateInput, NoteUpdateInput, TaskCreateInput, TaskFieldPatch,
    TaskUpdateInput,
};
use serde::de::DeserializeOwned;
use std::fs;
use std::io::{self, IsTerminal, Read};
use std::path::Path;
use zeroize::Zeroizing;

#[derive(Debug, Clone, Copy)]
enum JsonInputSource<'a> {
    File(&'a Path),
    Stdin,
}

impl<'a> JsonInputSource<'a> {
    fn label(self) -> &'a str {
        match self {
            Self::File(_) => "file",
            Self::Stdin => "stdin",
        }
    }
}

pub(crate) fn resolve_task_create_input(args: &TaskCreateArgsCli) -> PublicResult<TaskCreateInput> {
    if let Some(input) = load_structured_input::<TaskCreateInput>(
        args.input_file.as_deref(),
        args.input_stdin,
        args.password_stdin,
    )? {
        return Ok(input);
    }

    let title = args
        .title
        .as_deref()
        .map(str::to_owned)
        .ok_or_else(|| PublicError::validation("title is required"))?;
    Ok(TaskCreateInput {
        title,
        body: args.body.as_deref().map(str::to_owned),
        checklist: None,
        priority: args.priority,
        due_at: args.due_at,
        start_at: args.start_at,
        section_id: args.section_id,
        idempotency_key: args.idempotency_key.clone(),
    })
}

pub(crate) fn resolve_task_update_input(args: &TaskUpdateArgsCli) -> PublicResult<TaskUpdateInput> {
    if let Some(input) = load_structured_input::<TaskUpdateInput>(
        args.input_file.as_deref(),
        args.input_stdin,
        args.password_stdin,
    )? {
        return Ok(input);
    }

    Ok(TaskUpdateInput {
        title: args.title.as_deref().map(str::to_owned),
        body: task_field_patch(args.body.clone(), args.clear_body),
        checklist: TaskFieldPatch::Unchanged,
        priority: task_field_patch(args.priority, args.clear_priority),
        due_at: task_field_patch(args.due_at, args.clear_due_at),
        start_at: task_field_patch(args.start_at, args.clear_start_at),
        section_id: task_field_patch(args.section_id, args.clear_section),
    })
}

fn task_field_patch<T>(value: Option<T>, clear: bool) -> TaskFieldPatch<T> {
    match (value, clear) {
        (Some(value), false) => TaskFieldPatch::Set(value),
        (None, true) => TaskFieldPatch::Clear,
        (None, false) => TaskFieldPatch::Unchanged,
        (Some(_), true) => unreachable!("clap rejects a set value with its clear flag"),
    }
}

pub(crate) fn resolve_comment_input(
    body: Option<&str>,
    input_file: Option<&Path>,
    input_stdin: bool,
    password_stdin: bool,
) -> PublicResult<CommentInput> {
    if let Some(input) =
        load_structured_input::<CommentInput>(input_file, input_stdin, password_stdin)?
    {
        return Ok(input);
    }

    let body = body
        .map(str::to_owned)
        .ok_or_else(|| PublicError::validation("comment body is required"))?;
    Ok(CommentInput { body })
}

pub(crate) fn resolve_note_create_input(args: &NoteCreateArgsCli) -> PublicResult<NoteCreateInput> {
    if let Some(input) = load_structured_input::<NoteCreateInput>(
        args.input_file.as_deref(),
        args.input_stdin,
        args.password_stdin,
    )? {
        return Ok(input);
    }

    let title = args
        .title
        .clone()
        .ok_or_else(|| PublicError::validation("note title is required"))?;
    let idempotency_key = args.idempotency_key.clone().ok_or_else(|| {
        PublicError::validation(
            "--idempotency-key is required so note creation can be retried safely after process loss",
        )
    })?;
    Ok(NoteCreateInput {
        title,
        body: args.body.clone().unwrap_or_default(),
        is_private: args.is_private,
        idempotency_key,
    })
}

pub(crate) fn resolve_note_update_input(args: &NoteUpdateArgsCli) -> PublicResult<NoteUpdateInput> {
    if let Some(input) = load_structured_input::<NoteUpdateInput>(
        args.input_file.as_deref(),
        args.input_stdin,
        args.password_stdin,
    )? {
        return Ok(input);
    }

    Ok(NoteUpdateInput {
        title: args.title.clone(),
        body: args.body.clone(),
    })
}

pub(crate) fn resolve_delete_input<T: Default + DeserializeOwned>(
    input_file: Option<&Path>,
    input_stdin: bool,
) -> PublicResult<T> {
    load_structured_input(input_file, input_stdin, false).map(|input| input.unwrap_or_default())
}

fn load_structured_input<T: DeserializeOwned>(
    input_file: Option<&Path>,
    input_stdin: bool,
    password_stdin: bool,
) -> PublicResult<Option<T>> {
    let source = select_json_input_source(input_file, input_stdin, password_stdin)?;
    let Some(source) = source else {
        return Ok(None);
    };

    let contents = read_json_input(source)?;
    parse_json_input(&contents, source.label()).map(Some)
}

fn select_json_input_source<'a>(
    input_file: Option<&'a Path>,
    input_stdin: bool,
    password_stdin: bool,
) -> PublicResult<Option<JsonInputSource<'a>>> {
    if input_file.is_some() && input_stdin {
        return Err(PublicError::validation(
            "use only one of --input-file or --input-stdin",
        ));
    }
    if input_stdin && password_stdin {
        return Err(PublicError::validation(
            "--input-stdin cannot be combined with --password-stdin",
        ));
    }

    Ok(match (input_file, input_stdin) {
        (Some(path), false) => Some(JsonInputSource::File(path)),
        (None, true) => Some(JsonInputSource::Stdin),
        (None, false) => None,
        (Some(_), true) => unreachable!("validated mutually exclusive input flags"),
    })
}

fn read_json_input(source: JsonInputSource<'_>) -> PublicResult<String> {
    match source {
        JsonInputSource::File(path) => fs::read_to_string(path).map_err(|err| {
            PublicError::unexpected(format!(
                "failed to read input file {}: {err}",
                path.display()
            ))
        }),
        JsonInputSource::Stdin => {
            let mut input = String::new();
            io::stdin().read_to_string(&mut input).map_err(|err| {
                PublicError::unexpected(format!("failed to read input from stdin: {err}"))
            })?;
            Ok(input)
        }
    }
}

fn parse_json_input<T: DeserializeOwned>(contents: &str, source: &str) -> PublicResult<T> {
    serde_json::from_str(contents)
        .map_err(|err| PublicError::validation(format!("invalid JSON input from {source}: {err}")))
}

pub(crate) fn prompt(format: crate::output::OutputFormat, label: &str) -> CliResult<String> {
    if !io::stdin().is_terminal() {
        let field = label.trim().trim_end_matches(':');
        return Err(PublicError::validation(format!(
            "cannot prompt for {field} because stdin is not a terminal; pass the value explicitly or use --non-interactive"
        ))
        .into());
    }
    write_interaction(format, format_args!("{label}"))?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|err| PublicError::unexpected(format!("failed to read input: {err}")))?;

    Ok(input.trim().to_string())
}

fn read_password_from_stdin() -> PublicResult<String> {
    read_password_from(io::stdin().lock())
}

fn read_password_from(mut reader: impl Read) -> PublicResult<String> {
    let mut input = Zeroizing::new(String::new());
    reader.read_to_string(&mut input).map_err(|err| {
        PublicError::unexpected(format!("failed to read password from stdin: {err}"))
    })?;
    Ok(input.trim().to_string())
}

pub(crate) fn read_required_password(
    password_stdin: bool,
    prompt_message: Option<&str>,
) -> CliResult<String> {
    let password = if password_stdin {
        read_password_from_stdin()?
    } else {
        if !io::stdin().is_terminal() {
            return Err(PublicError::validation(
                "cannot prompt for a password because stdin is not a terminal; use --password-stdin or --non-interactive",
            )
            .into());
        }
        let prompt = prompt_message
            .map(|message| format!("{message}\nPassword: "))
            .unwrap_or_else(|| "Password: ".to_string());
        rpassword::prompt_password(prompt)
            .map_err(|err| PublicError::unexpected(format!("failed to read password: {err}")))?
    };

    if password.is_empty() {
        return Err(PublicError::validation("password is required").into());
    }
    Ok(password)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_should_keep_whole_trimmed_non_login_password_stdin_contract() {
        let password = read_password_from("first line\nsecond line\n".as_bytes()).expect("read");
        assert_eq!(password, "first line\nsecond line");
    }

    #[test]
    fn note_create_requires_a_process_recoverable_idempotency_key() {
        let args = NoteCreateArgsCli {
            project: None,
            work_list_id: Some(Uuid::now_v7()),
            title: Some("Retry-safe note".to_string()),
            body: None,
            is_private: false,
            idempotency_key: None,
            input_file: None,
            input_stdin: false,
            password_stdin: false,
        };

        let error = resolve_note_create_input(&args)
            .expect_err("an ephemeral default cannot survive Ctrl-C or process loss");
        assert!(error.to_string().contains("--idempotency-key is required"));
    }

    #[test]
    fn structured_comment_input_rejects_unknown_fields() {
        let error =
            parse_json_input::<CommentInput>(r#"{"body":"hello","ignoredBefore":true}"#, "test")
                .expect_err("unknown fields must not be silently ignored");
        assert!(error.to_string().contains("unknown field"));
        assert!(error.to_string().contains("ignoredBefore"));
    }
}
