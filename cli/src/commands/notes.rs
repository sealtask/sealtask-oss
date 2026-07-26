use crate::args::{
    NoteCreateArgsCli, NoteDeleteArgsCli, NoteEditArgsCli, NoteUpdateArgsCli, NotesCommand,
};
use crate::editor;
use crate::input::{resolve_delete_input, resolve_note_create_input, resolve_note_update_input};
use crate::interaction::require_confirmation;
use crate::output::{CliResult, OutputFormat, mutation_output_enabled};
use crate::render::{print_delete_result, print_empty_collection, print_note, print_notes};
use crate::resolver::{ProjectLifecycle, resolve_note, resolve_project};
use crate::terminal::with_progress;
use sealtask_client_api::DeleteNoteRequest;
use sealtask_client_runtime::{CreateNoteArgs, DeleteNoteArgs, RuntimeClient, UpdateNoteArgs};
use serde_json::json;

pub(crate) async fn run_notes(
    runtime: &RuntimeClient,
    format: OutputFormat,
    non_interactive: bool,
    command: NotesCommand,
) -> CliResult<()> {
    match command {
        NotesCommand::List {
            project,
            work_list_id,
            password_stdin,
        } => {
            let project = resolve_project(
                runtime,
                project.as_ref(),
                work_list_id,
                password_stdin,
                ProjectLifecycle::Any,
            )
            .await?;
            let notes = with_progress(
                "Loading and decrypting notes…",
                runtime.list_notes(project.id, password_stdin),
            )
            .await?;
            if notes.is_empty() {
                return print_empty_collection(format, "No notes found in this project.");
            }
            print_notes(&notes, format)
        }
        NotesCommand::Get {
            note,
            note_id,
            project,
            work_list_id,
            password_stdin,
        } => {
            let project = resolve_project(
                runtime,
                project.as_ref(),
                work_list_id,
                password_stdin,
                ProjectLifecycle::Any,
            )
            .await?;
            let note =
                resolve_note(runtime, project.id, note.as_ref(), note_id, password_stdin).await?;
            let note = with_progress(
                "Loading and decrypting note…",
                runtime.get_note(project.id, note.id, password_stdin),
            )
            .await?;
            print_note(&note, format)
        }
        NotesCommand::Create(args) => create_note(runtime, format, args).await,
        NotesCommand::Edit(args) => edit_note(runtime, format, non_interactive, args).await,
        NotesCommand::Update(args) => update_note(runtime, format, args).await,
        NotesCommand::Delete(args) => delete_note(runtime, format, non_interactive, args).await,
    }
}

async fn create_note(
    runtime: &RuntimeClient,
    format: OutputFormat,
    args: NoteCreateArgsCli,
) -> CliResult<()> {
    let project = resolve_project(
        runtime,
        args.project.as_ref(),
        args.work_list_id,
        args.password_stdin,
        ProjectLifecycle::Any,
    )
    .await?;
    let input = resolve_note_create_input(&args)?;
    let note = with_progress(
        "Creating note…",
        runtime.create_note(CreateNoteArgs {
            work_list_id: project.id,
            input,
            password_stdin: args.password_stdin,
        }),
    )
    .await?;
    if mutation_output_enabled(format) {
        print_note(&note, format)
    } else {
        Ok(())
    }
}

async fn update_note(
    runtime: &RuntimeClient,
    format: OutputFormat,
    args: NoteUpdateArgsCli,
) -> CliResult<()> {
    let project = resolve_project(
        runtime,
        args.project.as_ref(),
        args.work_list_id,
        args.password_stdin,
        ProjectLifecycle::Any,
    )
    .await?;
    let note = resolve_note(
        runtime,
        project.id,
        args.note.as_ref(),
        args.note_id,
        args.password_stdin,
    )
    .await?;
    let input = resolve_note_update_input(&args)?;
    let note = with_progress(
        "Updating note…",
        runtime.update_note(UpdateNoteArgs {
            work_list_id: project.id,
            note_id: note.id,
            input,
            password_stdin: args.password_stdin,
        }),
    )
    .await?;
    if mutation_output_enabled(format) {
        print_note(&note, format)
    } else {
        Ok(())
    }
}

async fn edit_note(
    runtime: &RuntimeClient,
    format: OutputFormat,
    non_interactive: bool,
    args: NoteEditArgsCli,
) -> CliResult<()> {
    if non_interactive {
        return Err(sealtask_client_core::PublicError::validation(
            "'sealtask notes edit' requires an interactive controlling terminal and cannot be combined with --non-interactive",
        )
        .into());
    }
    let project = resolve_project(
        runtime,
        args.project.as_ref(),
        args.work_list_id,
        args.password_stdin,
        ProjectLifecycle::Any,
    )
    .await?;
    let resolved = resolve_note(
        runtime,
        project.id,
        args.note.as_ref(),
        args.note_id,
        args.password_stdin,
    )
    .await?;
    let current = with_progress(
        "Loading and decrypting note to edit…",
        runtime.get_note(project.id, resolved.id, args.password_stdin),
    )
    .await?;
    if current.read_error.is_some() {
        return Err(sealtask_client_core::PublicError::validation(
            "the note cannot be edited because its encrypted content is unreadable; inspect it with 'sealtask notes get' and resolve the read error first",
        )
        .into());
    }
    let current_title = current.title.as_deref().ok_or_else(|| {
        sealtask_client_core::PublicError::validation(
            "the note cannot be edited because its decrypted title is unavailable",
        )
    })?;
    let current_body = current.body_markdown.as_deref().unwrap_or_default();
    let edited = editor::edit_existing_document("note", current_title, current_body)?;
    let input = sealtask_client_runtime::NoteUpdateInput {
        title: (edited.title.trim() != current_title.trim()).then(|| edited.title.clone()),
        body: (edited.body.trim() != current_body.trim()).then(|| edited.body.clone()),
    };
    if input.title.is_none() && input.body.is_none() {
        return if mutation_output_enabled(format) {
            print_note(&current, format)
        } else {
            Ok(())
        };
    }

    let note = with_progress(
        "Updating note…",
        runtime.update_note_if_unchanged(
            UpdateNoteArgs {
                work_list_id: project.id,
                note_id: resolved.id,
                input,
                password_stdin: args.password_stdin,
            },
            current.updated_at,
        ),
    )
    .await?;
    if mutation_output_enabled(format) {
        print_note(&note, format)
    } else {
        Ok(())
    }
}

async fn delete_note(
    runtime: &RuntimeClient,
    format: OutputFormat,
    non_interactive: bool,
    args: NoteDeleteArgsCli,
) -> CliResult<()> {
    let project = resolve_project(
        runtime,
        args.project.as_ref(),
        args.work_list_id,
        args.password_stdin,
        ProjectLifecycle::Any,
    )
    .await?;
    let note = resolve_note(
        runtime,
        project.id,
        args.note.as_ref(),
        args.note_id,
        args.password_stdin,
    )
    .await?;
    let target = note.name.as_deref().map_or_else(
        || format!("note {} in project {}", note.id, project.id),
        |name| format!("note \"{name}\" ({}) in project {}", note.id, project.id),
    );
    require_confirmation(
        format,
        non_interactive,
        args.yes,
        args.input_stdin || args.password_stdin,
        &target,
    )?;
    let input =
        resolve_delete_input::<DeleteNoteRequest>(args.input_file.as_deref(), args.input_stdin)?;
    with_progress(
        "Deleting note…",
        runtime.delete_note(DeleteNoteArgs {
            work_list_id: project.id,
            note_id: note.id,
            input,
        }),
    )
    .await?;
    print_delete_result(
        format,
        "note",
        &json!({
            "deleted": true,
            "workListId": project.id,
            "noteId": note.id,
        }),
        &format!("Deleted note {}.", note.id),
    )
}
