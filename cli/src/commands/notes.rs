use crate::args::{NoteCreateArgsCli, NoteDeleteArgsCli, NoteUpdateArgsCli, NotesCommand};
use crate::input::{resolve_delete_input, resolve_note_create_input, resolve_note_update_input};
use crate::output::{CliResult, OutputFormat};
use crate::render::{print_delete_result, print_empty_collection, print_note, print_notes};
use sealtask_client_api::DeleteNoteRequest;
use sealtask_client_runtime::{CreateNoteArgs, DeleteNoteArgs, RuntimeClient, UpdateNoteArgs};
use serde_json::json;

pub(crate) async fn run_notes(
    runtime: &RuntimeClient,
    format: OutputFormat,
    command: NotesCommand,
) -> CliResult<()> {
    match command {
        NotesCommand::List {
            work_list_id,
            password_stdin,
        } => {
            let notes = runtime.list_notes(work_list_id, password_stdin).await?;
            if notes.is_empty() {
                return print_empty_collection(format, "No notes found in this work list.");
            }
            print_notes(&notes, format)
        }
        NotesCommand::Get {
            work_list_id,
            note_id,
            password_stdin,
        } => {
            let note = runtime
                .get_note(work_list_id, note_id, password_stdin)
                .await?;
            print_note(&note, format)
        }
        NotesCommand::Create(args) => create_note(runtime, format, args).await,
        NotesCommand::Update(args) => update_note(runtime, format, args).await,
        NotesCommand::Delete(args) => delete_note(runtime, format, args).await,
    }
}

async fn create_note(
    runtime: &RuntimeClient,
    format: OutputFormat,
    args: NoteCreateArgsCli,
) -> CliResult<()> {
    let input = resolve_note_create_input(&args)?;
    let note = runtime
        .create_note(CreateNoteArgs {
            work_list_id: args.work_list_id,
            input,
            password_stdin: args.password_stdin,
        })
        .await?;
    print_note(&note, format)
}

async fn update_note(
    runtime: &RuntimeClient,
    format: OutputFormat,
    args: NoteUpdateArgsCli,
) -> CliResult<()> {
    let input = resolve_note_update_input(&args)?;
    let note = runtime
        .update_note(UpdateNoteArgs {
            work_list_id: args.work_list_id,
            note_id: args.note_id,
            input,
            password_stdin: args.password_stdin,
        })
        .await?;
    print_note(&note, format)
}

async fn delete_note(
    runtime: &RuntimeClient,
    format: OutputFormat,
    args: NoteDeleteArgsCli,
) -> CliResult<()> {
    let input =
        resolve_delete_input::<DeleteNoteRequest>(args.input_file.as_deref(), args.input_stdin)?;
    runtime
        .delete_note(DeleteNoteArgs {
            work_list_id: args.work_list_id,
            note_id: args.note_id,
            input,
        })
        .await?;
    print_delete_result(
        format,
        "note",
        &json!({
            "deleted": true,
            "workListId": args.work_list_id,
            "noteId": args.note_id,
        }),
        &format!("Deleted note {}.", args.note_id),
    )
}
