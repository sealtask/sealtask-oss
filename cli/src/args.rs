use chrono::{DateTime, Utc};
use clap::{Args, Parser, Subcommand};
use std::fmt;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(
    name = "sealtask",
    version,
    about = "CLI for working with SealTask tasks, comments, notes, attachments, and decrypted workspace data"
)]
pub(crate) struct Cli {
    #[arg(
        long,
        env = "SEALTASK_API_URL",
        default_value = "https://sealtask.com",
        global = true
    )]
    pub(crate) api_url: String,

    #[arg(
        long,
        env = "SEALTASK_STORAGE_ORIGINS",
        value_delimiter = ',',
        global = true,
        help = "Trusted origin for presigned attachment transfers (repeatable)"
    )]
    pub(crate) storage_origin: Vec<String>,

    #[arg(long, global = true)]
    pub(crate) json: bool,

    #[arg(long, hide = true)]
    pub(crate) serve_unlock_daemon: Option<PathBuf>,

    #[command(subcommand)]
    pub(crate) command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Command {
    Info,
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },
    Me,
    Lists {
        #[arg(long)]
        verbose: bool,
        #[arg(long)]
        include_archived: bool,
        #[arg(long)]
        password_stdin: bool,
        #[arg(long, hide = true)]
        raw: bool,
        #[command(subcommand)]
        command: Option<ListsCommand>,
    },
    Tasks {
        #[command(subcommand)]
        command: TasksCommand,
    },
    Stats,
    #[command(hide = true)]
    Inspect {
        work_list_id: Uuid,
        #[arg(long)]
        password_stdin: bool,
    },
    Comments {
        #[command(subcommand)]
        command: CommentsCommand,
    },
    Notes {
        #[command(subcommand)]
        command: NotesCommand,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum ListsCommand {
    Get {
        work_list_id: Uuid,
        #[arg(long)]
        password_stdin: bool,
        #[arg(long, hide = true)]
        raw: bool,
    },
    Archive {
        work_list_id: Uuid,
        #[arg(long)]
        password_stdin: bool,
        #[arg(long, hide = true)]
        raw: bool,
    },
    Unarchive {
        work_list_id: Uuid,
        #[arg(long)]
        password_stdin: bool,
        #[arg(long, hide = true)]
        raw: bool,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum TasksCommand {
    List {
        #[arg(long)]
        work_list_id: Option<Uuid>,
        #[arg(long)]
        include_completed: bool,
        #[arg(long)]
        all: bool,
        #[arg(long)]
        password_stdin: bool,
        #[arg(long, hide = true)]
        raw: bool,
    },
    Get {
        #[arg(long)]
        work_list_id: Uuid,
        #[arg(long)]
        task_id: Uuid,
        #[arg(long)]
        password_stdin: bool,
        #[arg(long, hide = true)]
        raw: bool,
    },
    Create(TaskCreateArgsCli),
    Update(TaskUpdateArgsCli),
    Move(TaskMoveArgsCli),
    Complete(TaskCompletionArgsCli),
    Reopen(TaskCompletionArgsCli),
    Archive(TaskArchiveArgsCli),
    Unarchive(TaskUnarchiveArgsCli),
    Delete(TaskDeleteArgsCli),
    Attachments {
        #[command(subcommand)]
        command: TaskAttachmentsCommand,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum TaskAttachmentsCommand {
    Upload(TaskAttachmentUploadArgsCli),
    Delete(TaskAttachmentDeleteArgsCli),
    Read(TaskAttachmentReadArgsCli),
    Download(TaskAttachmentDownloadArgsCli),
}

#[derive(Subcommand, Debug)]
pub(crate) enum NotesCommand {
    List {
        #[arg(long)]
        work_list_id: Uuid,
        #[arg(long)]
        password_stdin: bool,
    },
    Get {
        #[arg(long)]
        work_list_id: Uuid,
        #[arg(long)]
        note_id: Uuid,
        #[arg(long)]
        password_stdin: bool,
    },
    Create(NoteCreateArgsCli),
    Update(NoteUpdateArgsCli),
    Delete(NoteDeleteArgsCli),
}

#[derive(Subcommand, Debug)]
pub(crate) enum CommentsCommand {
    List {
        #[arg(long)]
        work_list_id: Uuid,
        #[arg(long)]
        task_id: Uuid,
        #[arg(long)]
        password_stdin: bool,
    },
    Create(CommentCreateArgsCli),
    Update(CommentUpdateArgsCli),
    Delete(CommentDeleteArgsCli),
}

#[derive(Args, Debug)]
pub(crate) struct TaskCreateArgsCli {
    #[arg(long)]
    pub(crate) work_list_id: Uuid,
    #[arg(long)]
    pub(crate) title: Option<String>,
    #[arg(long)]
    pub(crate) body: Option<String>,
    #[arg(long)]
    pub(crate) priority: Option<i8>,
    #[arg(long)]
    pub(crate) due_at: Option<DateTime<Utc>>,
    #[arg(long)]
    pub(crate) start_at: Option<DateTime<Utc>>,
    #[arg(long)]
    pub(crate) section_id: Option<Uuid>,
    #[arg(long)]
    pub(crate) idempotency_key: Option<String>,
    #[arg(long)]
    pub(crate) input_file: Option<PathBuf>,
    #[arg(long)]
    pub(crate) input_stdin: bool,
    #[arg(long)]
    pub(crate) password_stdin: bool,
}

#[derive(Args, Debug)]
pub(crate) struct TaskUpdateArgsCli {
    #[arg(long)]
    pub(crate) work_list_id: Uuid,
    #[arg(long)]
    pub(crate) task_id: Uuid,
    #[arg(long)]
    pub(crate) title: Option<String>,
    #[arg(long, conflicts_with = "clear_body")]
    pub(crate) body: Option<String>,
    #[arg(long)]
    pub(crate) clear_body: bool,
    #[arg(long, conflicts_with = "clear_priority")]
    pub(crate) priority: Option<i8>,
    #[arg(long)]
    pub(crate) clear_priority: bool,
    #[arg(long, conflicts_with = "clear_due_at")]
    pub(crate) due_at: Option<DateTime<Utc>>,
    #[arg(long)]
    pub(crate) clear_due_at: bool,
    #[arg(long, conflicts_with = "clear_start_at")]
    pub(crate) start_at: Option<DateTime<Utc>>,
    #[arg(long)]
    pub(crate) clear_start_at: bool,
    #[arg(long, conflicts_with = "clear_section")]
    pub(crate) section_id: Option<Uuid>,
    #[arg(long)]
    pub(crate) clear_section: bool,
    #[arg(long)]
    pub(crate) input_file: Option<PathBuf>,
    #[arg(long)]
    pub(crate) input_stdin: bool,
    #[arg(long)]
    pub(crate) password_stdin: bool,
}

#[derive(Args, Debug)]
pub(crate) struct TaskMoveArgsCli {
    #[arg(long)]
    pub(crate) work_list_id: Uuid,
    #[arg(long)]
    pub(crate) task_id: Uuid,
    #[arg(long)]
    pub(crate) section_id: Option<Uuid>,
    #[arg(long)]
    pub(crate) insert_before_task_id: Option<Uuid>,
    #[arg(long)]
    pub(crate) password_stdin: bool,
}

#[derive(Args, Debug)]
pub(crate) struct TaskCompletionArgsCli {
    #[arg(long)]
    pub(crate) work_list_id: Uuid,
    #[arg(long)]
    pub(crate) task_id: Uuid,
    #[arg(long)]
    pub(crate) password_stdin: bool,
}

#[derive(Args, Debug)]
pub(crate) struct TaskArchiveArgsCli {
    #[arg(long)]
    pub(crate) work_list_id: Uuid,
    #[arg(long)]
    pub(crate) task_id: Uuid,
    #[arg(long)]
    pub(crate) password_stdin: bool,
}

#[derive(Args, Debug)]
pub(crate) struct TaskUnarchiveArgsCli {
    #[arg(long)]
    pub(crate) work_list_id: Uuid,
    #[arg(long)]
    pub(crate) task_id: Uuid,
    #[arg(long)]
    pub(crate) password_stdin: bool,
}

#[derive(Args, Debug)]
pub(crate) struct TaskDeleteArgsCli {
    #[arg(long)]
    pub(crate) work_list_id: Uuid,
    #[arg(long)]
    pub(crate) task_id: Uuid,
    #[arg(long)]
    pub(crate) input_file: Option<PathBuf>,
    #[arg(long)]
    pub(crate) input_stdin: bool,
}

#[derive(Args, Debug)]
pub(crate) struct CommentCreateArgsCli {
    #[arg(long)]
    pub(crate) work_list_id: Uuid,
    #[arg(long)]
    pub(crate) task_id: Uuid,
    #[arg(long)]
    pub(crate) body: Option<String>,
    #[arg(long)]
    pub(crate) input_file: Option<PathBuf>,
    #[arg(long)]
    pub(crate) input_stdin: bool,
    #[arg(long)]
    pub(crate) password_stdin: bool,
}

#[derive(Args, Debug)]
pub(crate) struct CommentUpdateArgsCli {
    #[arg(long)]
    pub(crate) work_list_id: Uuid,
    #[arg(long)]
    pub(crate) task_id: Uuid,
    #[arg(long)]
    pub(crate) comment_id: Uuid,
    #[arg(long)]
    pub(crate) body: Option<String>,
    #[arg(long)]
    pub(crate) input_file: Option<PathBuf>,
    #[arg(long)]
    pub(crate) input_stdin: bool,
    #[arg(long)]
    pub(crate) password_stdin: bool,
}

#[derive(Args, Debug)]
pub(crate) struct CommentDeleteArgsCli {
    #[arg(long)]
    pub(crate) work_list_id: Uuid,
    #[arg(long)]
    pub(crate) task_id: Uuid,
    #[arg(long)]
    pub(crate) comment_id: Uuid,
    #[arg(long)]
    pub(crate) input_file: Option<PathBuf>,
    #[arg(long)]
    pub(crate) input_stdin: bool,
}

#[derive(Args)]
pub(crate) struct NoteCreateArgsCli {
    #[arg(long)]
    pub(crate) work_list_id: Uuid,
    #[arg(long)]
    pub(crate) title: Option<String>,
    #[arg(long)]
    pub(crate) body: Option<String>,
    #[arg(long = "private")]
    pub(crate) is_private: bool,
    #[arg(long)]
    pub(crate) idempotency_key: Option<String>,
    #[arg(long)]
    pub(crate) input_file: Option<PathBuf>,
    #[arg(long)]
    pub(crate) input_stdin: bool,
    #[arg(long)]
    pub(crate) password_stdin: bool,
}

impl fmt::Debug for NoteCreateArgsCli {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NoteCreateArgsCli")
            .field("work_list_id", &self.work_list_id)
            .field("title_present", &self.title.is_some())
            .field("body_present", &self.body.is_some())
            .field("is_private", &self.is_private)
            .field("idempotency_key", &self.idempotency_key)
            .field("input_file_present", &self.input_file.is_some())
            .field("input_stdin", &self.input_stdin)
            .field("password_stdin", &self.password_stdin)
            .finish()
    }
}

#[derive(Args)]
pub(crate) struct NoteUpdateArgsCli {
    #[arg(long)]
    pub(crate) work_list_id: Uuid,
    #[arg(long)]
    pub(crate) note_id: Uuid,
    #[arg(long)]
    pub(crate) title: Option<String>,
    #[arg(long)]
    pub(crate) body: Option<String>,
    #[arg(long)]
    pub(crate) input_file: Option<PathBuf>,
    #[arg(long)]
    pub(crate) input_stdin: bool,
    #[arg(long)]
    pub(crate) password_stdin: bool,
}

impl fmt::Debug for NoteUpdateArgsCli {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NoteUpdateArgsCli")
            .field("work_list_id", &self.work_list_id)
            .field("note_id", &self.note_id)
            .field("title_present", &self.title.is_some())
            .field("body_present", &self.body.is_some())
            .field("input_file_present", &self.input_file.is_some())
            .field("input_stdin", &self.input_stdin)
            .field("password_stdin", &self.password_stdin)
            .finish()
    }
}

#[derive(Args, Debug)]
pub(crate) struct NoteDeleteArgsCli {
    #[arg(long)]
    pub(crate) work_list_id: Uuid,
    #[arg(long)]
    pub(crate) note_id: Uuid,
    #[arg(long)]
    pub(crate) input_file: Option<PathBuf>,
    #[arg(long)]
    pub(crate) input_stdin: bool,
}

#[derive(Args, Debug)]
pub(crate) struct TaskAttachmentUploadArgsCli {
    #[arg(long)]
    pub(crate) work_list_id: Uuid,
    #[arg(long)]
    pub(crate) task_id: Uuid,
    #[arg(
        long,
        help = "Current-working-directory-relative regular file to upload (absolute paths, parent traversal, and symlinks are rejected)"
    )]
    pub(crate) file: PathBuf,
    #[arg(long)]
    pub(crate) file_name: Option<String>,
    #[arg(long)]
    pub(crate) content_type: Option<String>,
    #[arg(long)]
    pub(crate) password_stdin: bool,
}

#[derive(Args, Debug)]
pub(crate) struct TaskAttachmentDeleteArgsCli {
    #[arg(long)]
    pub(crate) work_list_id: Uuid,
    #[arg(long)]
    pub(crate) task_id: Uuid,
    #[arg(long)]
    pub(crate) attachment_id: Uuid,
    #[arg(long)]
    pub(crate) password_stdin: bool,
}

#[derive(Args, Debug)]
pub(crate) struct TaskAttachmentReadArgsCli {
    #[arg(long)]
    pub(crate) work_list_id: Uuid,
    #[arg(long)]
    pub(crate) task_id: Uuid,
    #[arg(long)]
    pub(crate) attachment_id: Uuid,
    #[arg(long)]
    pub(crate) password_stdin: bool,
}

#[derive(Args, Debug)]
pub(crate) struct TaskAttachmentDownloadArgsCli {
    #[arg(long)]
    pub(crate) work_list_id: Uuid,
    #[arg(long)]
    pub(crate) task_id: Uuid,
    #[arg(long)]
    pub(crate) attachment_id: Uuid,
    #[arg(
        long,
        help = "Current-working-directory-relative output path (absolute paths, parent traversal, and symlinks are rejected)"
    )]
    pub(crate) output: Option<PathBuf>,
    #[arg(long)]
    pub(crate) force: bool,
    #[arg(long)]
    pub(crate) password_stdin: bool,
}

#[derive(Subcommand, Debug)]
pub(crate) enum AuthCommand {
    Login {
        #[arg(long)]
        email: Option<String>,
        #[arg(
            long,
            help = "Read login input from stdin: trimmed password on line 1 and optional exact authenticator or backup code on line 2"
        )]
        password_stdin: bool,
    },
    Unlock {
        #[arg(long, default_value_t = 8 * 60 * 60)]
        ttl_seconds: u64,
        #[arg(long)]
        password_stdin: bool,
    },
    Lock,
    Keychain {
        #[command(subcommand)]
        command: KeychainCommand,
    },
    Logout,
    Status,
}

#[derive(Subcommand, Debug)]
pub(crate) enum KeychainCommand {
    Store {
        #[arg(long)]
        password_stdin: bool,
    },
    Clear,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_level_cli_debug_redacts_note_plaintext() {
        let create_title = "canary-create-title-secret";
        let create_body = "canary-create-body-secret";
        let create = Cli::try_parse_from([
            "sealtask",
            "notes",
            "create",
            "--work-list-id",
            "018f4a76-c9f2-7f38-a09a-2ac748db8ee8",
            "--title",
            create_title,
            "--body",
            create_body,
            "--private",
        ])
        .expect("parse note create");
        let create_debug = format!("{create:?}");
        assert!(!create_debug.contains(create_title));
        assert!(!create_debug.contains(create_body));
        assert!(create_debug.contains("title_present: true"));
        assert!(create_debug.contains("body_present: true"));

        let update_title = "canary-update-title-secret";
        let update_body = "canary-update-body-secret";
        let update = Cli::try_parse_from([
            "sealtask",
            "notes",
            "update",
            "--work-list-id",
            "018f4a76-c9f2-7f38-a09a-2ac748db8ee8",
            "--note-id",
            "018f4a76-c9f2-7f38-a09a-2ac748db8ee9",
            "--title",
            update_title,
            "--body",
            update_body,
        ])
        .expect("parse note update");
        let update_debug = format!("{update:?}");
        assert!(!update_debug.contains(update_title));
        assert!(!update_debug.contains(update_body));
        assert!(update_debug.contains("title_present: true"));
        assert!(update_debug.contains("body_present: true"));
    }
}
