use chrono::{DateTime, Utc};
use clap::{Args, Parser, Subcommand, ValueEnum};
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
    /// SealTask API base URL.
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

    /// Emit compact JSON instead of human-readable output.
    #[arg(long, global = true, conflicts_with = "format")]
    pub(crate) json: bool,

    /// Select human-readable, compact JSON, or pretty JSON output.
    #[arg(
        long = "format",
        global = true,
        value_enum,
        value_name = "FORMAT",
        conflicts_with = "json"
    )]
    pub(crate) format: Option<OutputArg>,

    /// Never prompt; fail with an actionable validation error when input is missing.
    #[arg(long, global = true)]
    pub(crate) non_interactive: bool,

    /// Isolate credentials and unlock state under a named profile.
    #[arg(long, env = "SEALTASK_PROFILE", global = true, value_name = "NAME")]
    pub(crate) profile: Option<String>,

    /// Override the base directory used for credentials and local unlock state.
    #[arg(long, env = "SEALTASK_CONFIG_DIR", global = true, value_name = "PATH")]
    pub(crate) config_dir: Option<PathBuf>,

    #[arg(long, hide = true)]
    pub(crate) serve_unlock_daemon: Option<PathBuf>,

    #[command(subcommand)]
    pub(crate) command: Option<Command>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum OutputArg {
    Table,
    Json,
    JsonPretty,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Command {
    /// Show machine-readable CLI capabilities and contract versions.
    Info,
    /// Describe commands and arguments as human help or versioned JSON.
    Schema {
        /// Optional nested command path, for example: tasks create.
        #[arg(value_name = "COMMAND", num_args = 0..)]
        command: Vec<String>,
    },
    /// Authenticate, inspect the session, and manage local unlock state.
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },
    /// Show the current authenticated user.
    Me,
    /// List, inspect, archive, or restore work lists.
    Lists {
        /// Print expanded human-readable work-list details.
        #[arg(long)]
        verbose: bool,
        /// Include archived work lists.
        #[arg(long)]
        include_archived: bool,
        /// Read the account password from stdin when no local unlock is available.
        #[arg(long)]
        password_stdin: bool,
        #[arg(long, hide = true)]
        raw: bool,
        #[command(subcommand)]
        command: Option<ListsCommand>,
    },
    /// List, inspect, create, update, move, or delete tasks.
    Tasks {
        #[command(subcommand)]
        command: TasksCommand,
    },
    /// Show current dashboard task counts.
    Stats,
    #[command(hide = true)]
    Inspect {
        work_list_id: Uuid,
        #[arg(long)]
        password_stdin: bool,
    },
    /// List, create, update, or delete task comments.
    Comments {
        #[command(subcommand)]
        command: CommentsCommand,
    },
    /// List, inspect, create, update, or delete encrypted notes.
    Notes {
        #[command(subcommand)]
        command: NotesCommand,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum ListsCommand {
    /// Show one decrypted work list.
    Get {
        /// Work-list UUID.
        work_list_id: Uuid,
        /// Read the account password from stdin when no local unlock is available.
        #[arg(long)]
        password_stdin: bool,
        #[arg(long, hide = true)]
        raw: bool,
    },
    /// Archive a work list and make it read-only.
    Archive {
        /// Work-list UUID.
        work_list_id: Uuid,
        /// Read the account password from stdin when no local unlock is available.
        #[arg(long)]
        password_stdin: bool,
        #[arg(long, hide = true)]
        raw: bool,
    },
    /// Restore an archived work list.
    Unarchive {
        /// Work-list UUID.
        work_list_id: Uuid,
        /// Read the account password from stdin when no local unlock is available.
        #[arg(long)]
        password_stdin: bool,
        #[arg(long, hide = true)]
        raw: bool,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum TasksCommand {
    /// List decrypted tasks assigned to the current user or in one work list.
    List {
        /// Restrict results to one work-list UUID.
        #[arg(long)]
        work_list_id: Option<Uuid>,
        /// Include tasks in completed sections.
        #[arg(long)]
        include_completed: bool,
        /// List assigned tasks across all accessible work lists.
        #[arg(long)]
        all: bool,
        /// Read the account password from stdin when no local unlock is available.
        #[arg(long)]
        password_stdin: bool,
        #[arg(long, hide = true)]
        raw: bool,
    },
    /// Show one decrypted task, including comments and attachment metadata.
    Get {
        /// Work-list UUID containing the task.
        #[arg(long)]
        work_list_id: Uuid,
        /// Task UUID.
        #[arg(long)]
        task_id: Uuid,
        /// Read the account password from stdin when no local unlock is available.
        #[arg(long)]
        password_stdin: bool,
        #[arg(long, hide = true)]
        raw: bool,
    },
    /// Create an encrypted task.
    Create(TaskCreateArgsCli),
    /// Patch an encrypted task; omitted fields remain unchanged.
    Update(TaskUpdateArgsCli),
    /// Move a task to a section or relative position.
    Move(TaskMoveArgsCli),
    /// Move a task to the final section.
    Complete(TaskCompletionArgsCli),
    /// Move a task to the first section.
    Reopen(TaskCompletionArgsCli),
    /// Archive a task.
    Archive(TaskArchiveArgsCli),
    /// Restore an archived task.
    Unarchive(TaskUnarchiveArgsCli),
    /// Permanently delete a task.
    Delete(TaskDeleteArgsCli),
    /// Upload, delete, read, or download encrypted task attachments.
    Attachments {
        #[command(subcommand)]
        command: TaskAttachmentsCommand,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum TaskAttachmentsCommand {
    /// Encrypt and upload a local regular file.
    Upload(TaskAttachmentUploadArgsCli),
    /// Remove an attachment reference and its encrypted object.
    Delete(TaskAttachmentDeleteArgsCli),
    /// Decrypt a text or DOCX attachment and print readable text.
    Read(TaskAttachmentReadArgsCli),
    /// Decrypt an attachment and save it beneath the current directory.
    Download(TaskAttachmentDownloadArgsCli),
}

#[derive(Subcommand, Debug)]
pub(crate) enum NotesCommand {
    /// List decrypted notes in a work list.
    List {
        /// Work-list UUID.
        #[arg(long)]
        work_list_id: Uuid,
        /// Read the account password from stdin when no local unlock is available.
        #[arg(long)]
        password_stdin: bool,
    },
    /// Show one decrypted note.
    Get {
        /// Work-list UUID.
        #[arg(long)]
        work_list_id: Uuid,
        /// Note UUID.
        #[arg(long)]
        note_id: Uuid,
        /// Read the account password from stdin when no local unlock is available.
        #[arg(long)]
        password_stdin: bool,
    },
    /// Create an encrypted shared or private note.
    Create(NoteCreateArgsCli),
    /// Patch an encrypted note.
    Update(NoteUpdateArgsCli),
    /// Permanently delete a note.
    Delete(NoteDeleteArgsCli),
}

#[derive(Subcommand, Debug)]
pub(crate) enum CommentsCommand {
    /// List decrypted comments on a task.
    List {
        /// Work-list UUID containing the task.
        #[arg(long)]
        work_list_id: Uuid,
        /// Task UUID.
        #[arg(long)]
        task_id: Uuid,
        /// Read the account password from stdin when no local unlock is available.
        #[arg(long)]
        password_stdin: bool,
    },
    /// Create an encrypted task comment.
    Create(CommentCreateArgsCli),
    /// Replace an encrypted task comment body.
    Update(CommentUpdateArgsCli),
    /// Permanently delete a task comment.
    Delete(CommentDeleteArgsCli),
}

#[derive(Args, Debug)]
pub(crate) struct TaskCreateArgsCli {
    /// Work-list UUID that will own the task.
    #[arg(long)]
    pub(crate) work_list_id: Uuid,
    /// Plaintext task title.
    #[arg(long)]
    pub(crate) title: Option<String>,
    /// Plaintext Markdown task body.
    #[arg(long)]
    pub(crate) body: Option<String>,
    /// Task priority: 1 (low), 3 (medium), 5 (high), or 8 (urgent).
    #[arg(long, value_parser = parse_priority)]
    pub(crate) priority: Option<i8>,
    /// Due time as an RFC 3339 timestamp.
    #[arg(long)]
    pub(crate) due_at: Option<DateTime<Utc>>,
    /// Start time as an RFC 3339 timestamp.
    #[arg(long)]
    pub(crate) start_at: Option<DateTime<Utc>>,
    /// Initial section UUID.
    #[arg(long)]
    pub(crate) section_id: Option<Uuid>,
    /// Stable retry key containing at most 128 ASCII letters, digits, '.', '_', '-', or ':'.
    #[arg(long)]
    pub(crate) idempotency_key: Option<String>,
    /// Read the complete camelCase task input object from a UTF-8 JSON file.
    #[arg(
        long,
        value_name = "PATH",
        conflicts_with_all = [
            "input_stdin",
            "title",
            "body",
            "priority",
            "due_at",
            "start_at",
            "section_id",
            "idempotency_key"
        ]
    )]
    pub(crate) input_file: Option<PathBuf>,
    /// Read the complete camelCase task input object from stdin.
    #[arg(
        long,
        conflicts_with_all = [
            "input_file",
            "password_stdin",
            "title",
            "body",
            "priority",
            "due_at",
            "start_at",
            "section_id",
            "idempotency_key"
        ]
    )]
    pub(crate) input_stdin: bool,
    /// Read the account password from stdin when no local unlock is available.
    #[arg(long)]
    pub(crate) password_stdin: bool,
}

#[derive(Args, Debug)]
pub(crate) struct TaskUpdateArgsCli {
    /// Work-list UUID containing the task.
    #[arg(long)]
    pub(crate) work_list_id: Uuid,
    /// Task UUID.
    #[arg(long)]
    pub(crate) task_id: Uuid,
    /// Replace the task title.
    #[arg(long)]
    pub(crate) title: Option<String>,
    /// Replace the Markdown body.
    #[arg(long, conflicts_with = "clear_body")]
    pub(crate) body: Option<String>,
    /// Remove the task body.
    #[arg(long)]
    pub(crate) clear_body: bool,
    /// Set priority to 1 (low), 3 (medium), 5 (high), or 8 (urgent).
    #[arg(
        long,
        conflicts_with = "clear_priority",
        value_parser = parse_priority
    )]
    pub(crate) priority: Option<i8>,
    /// Remove the priority.
    #[arg(long)]
    pub(crate) clear_priority: bool,
    /// Set the due time as an RFC 3339 timestamp.
    #[arg(long, conflicts_with = "clear_due_at")]
    pub(crate) due_at: Option<DateTime<Utc>>,
    /// Remove the due time.
    #[arg(long)]
    pub(crate) clear_due_at: bool,
    /// Set the start time as an RFC 3339 timestamp.
    #[arg(long, conflicts_with = "clear_start_at")]
    pub(crate) start_at: Option<DateTime<Utc>>,
    /// Remove the start time.
    #[arg(long)]
    pub(crate) clear_start_at: bool,
    /// Move the task to this section UUID.
    #[arg(long, conflicts_with = "clear_section")]
    pub(crate) section_id: Option<Uuid>,
    /// Remove the explicit section assignment.
    #[arg(long)]
    pub(crate) clear_section: bool,
    /// Read the complete camelCase patch object from a UTF-8 JSON file.
    #[arg(
        long,
        value_name = "PATH",
        conflicts_with_all = [
            "input_stdin",
            "title",
            "body",
            "clear_body",
            "priority",
            "clear_priority",
            "due_at",
            "clear_due_at",
            "start_at",
            "clear_start_at",
            "section_id",
            "clear_section"
        ]
    )]
    pub(crate) input_file: Option<PathBuf>,
    /// Read the complete camelCase patch object from stdin.
    #[arg(
        long,
        conflicts_with_all = [
            "input_file",
            "password_stdin",
            "title",
            "body",
            "clear_body",
            "priority",
            "clear_priority",
            "due_at",
            "clear_due_at",
            "start_at",
            "clear_start_at",
            "section_id",
            "clear_section"
        ]
    )]
    pub(crate) input_stdin: bool,
    /// Read the account password from stdin when no local unlock is available.
    #[arg(long)]
    pub(crate) password_stdin: bool,
}

#[derive(Args, Debug)]
pub(crate) struct TaskMoveArgsCli {
    /// Work-list UUID containing the task.
    #[arg(long)]
    pub(crate) work_list_id: Uuid,
    /// Task UUID.
    #[arg(long)]
    pub(crate) task_id: Uuid,
    /// Destination section UUID.
    #[arg(long)]
    pub(crate) section_id: Option<Uuid>,
    /// Place the task immediately before this task UUID.
    #[arg(long)]
    pub(crate) insert_before_task_id: Option<Uuid>,
    /// Read the account password from stdin when no local unlock is available.
    #[arg(long)]
    pub(crate) password_stdin: bool,
}

#[derive(Args, Debug)]
pub(crate) struct TaskCompletionArgsCli {
    /// Work-list UUID containing the task.
    #[arg(long)]
    pub(crate) work_list_id: Uuid,
    /// Task UUID.
    #[arg(long)]
    pub(crate) task_id: Uuid,
    /// Read the account password from stdin when no local unlock is available.
    #[arg(long)]
    pub(crate) password_stdin: bool,
}

#[derive(Args, Debug)]
pub(crate) struct TaskArchiveArgsCli {
    /// Work-list UUID containing the task.
    #[arg(long)]
    pub(crate) work_list_id: Uuid,
    /// Task UUID.
    #[arg(long)]
    pub(crate) task_id: Uuid,
    /// Read the account password from stdin when no local unlock is available.
    #[arg(long)]
    pub(crate) password_stdin: bool,
}

#[derive(Args, Debug)]
pub(crate) struct TaskUnarchiveArgsCli {
    /// Work-list UUID containing the task.
    #[arg(long)]
    pub(crate) work_list_id: Uuid,
    /// Task UUID.
    #[arg(long)]
    pub(crate) task_id: Uuid,
    /// Read the account password from stdin when no local unlock is available.
    #[arg(long)]
    pub(crate) password_stdin: bool,
}

#[derive(Args, Debug)]
pub(crate) struct TaskDeleteArgsCli {
    /// Work-list UUID containing the task.
    #[arg(long)]
    pub(crate) work_list_id: Uuid,
    /// Task UUID.
    #[arg(long)]
    pub(crate) task_id: Uuid,
    /// Read an optional audit patch from a UTF-8 JSON file.
    #[arg(long, value_name = "PATH", conflicts_with = "input_stdin")]
    pub(crate) input_file: Option<PathBuf>,
    /// Read an optional audit patch from stdin.
    #[arg(long, conflicts_with = "input_file")]
    pub(crate) input_stdin: bool,
}

#[derive(Args, Debug)]
pub(crate) struct CommentCreateArgsCli {
    /// Work-list UUID containing the task.
    #[arg(long)]
    pub(crate) work_list_id: Uuid,
    /// Task UUID.
    #[arg(long)]
    pub(crate) task_id: Uuid,
    /// Plaintext Markdown comment body.
    #[arg(long)]
    pub(crate) body: Option<String>,
    /// Read the complete camelCase comment input object from a UTF-8 JSON file.
    #[arg(
        long,
        value_name = "PATH",
        conflicts_with_all = ["input_stdin", "body"]
    )]
    pub(crate) input_file: Option<PathBuf>,
    /// Read the complete camelCase comment input object from stdin.
    #[arg(
        long,
        conflicts_with_all = ["input_file", "password_stdin", "body"]
    )]
    pub(crate) input_stdin: bool,
    /// Read the account password from stdin when no local unlock is available.
    #[arg(long)]
    pub(crate) password_stdin: bool,
}

#[derive(Args, Debug)]
pub(crate) struct CommentUpdateArgsCli {
    /// Work-list UUID containing the task.
    #[arg(long)]
    pub(crate) work_list_id: Uuid,
    /// Task UUID.
    #[arg(long)]
    pub(crate) task_id: Uuid,
    /// Comment UUID.
    #[arg(long)]
    pub(crate) comment_id: Uuid,
    /// Replacement plaintext Markdown comment body.
    #[arg(long)]
    pub(crate) body: Option<String>,
    /// Read the complete camelCase comment input object from a UTF-8 JSON file.
    #[arg(
        long,
        value_name = "PATH",
        conflicts_with_all = ["input_stdin", "body"]
    )]
    pub(crate) input_file: Option<PathBuf>,
    /// Read the complete camelCase comment input object from stdin.
    #[arg(
        long,
        conflicts_with_all = ["input_file", "password_stdin", "body"]
    )]
    pub(crate) input_stdin: bool,
    /// Read the account password from stdin when no local unlock is available.
    #[arg(long)]
    pub(crate) password_stdin: bool,
}

#[derive(Args, Debug)]
pub(crate) struct CommentDeleteArgsCli {
    /// Work-list UUID containing the task.
    #[arg(long)]
    pub(crate) work_list_id: Uuid,
    /// Task UUID.
    #[arg(long)]
    pub(crate) task_id: Uuid,
    /// Comment UUID.
    #[arg(long)]
    pub(crate) comment_id: Uuid,
    /// Read an optional audit patch from a UTF-8 JSON file.
    #[arg(long, value_name = "PATH", conflicts_with = "input_stdin")]
    pub(crate) input_file: Option<PathBuf>,
    /// Read an optional audit patch from stdin.
    #[arg(long, conflicts_with = "input_file")]
    pub(crate) input_stdin: bool,
}

#[derive(Args)]
pub(crate) struct NoteCreateArgsCli {
    /// Work-list UUID that will own the note.
    #[arg(long)]
    pub(crate) work_list_id: Uuid,
    /// Plaintext note title.
    #[arg(long)]
    pub(crate) title: Option<String>,
    /// Plaintext Markdown note body.
    #[arg(long)]
    pub(crate) body: Option<String>,
    /// Encrypt with a per-note key available only to the current user.
    #[arg(long = "private")]
    pub(crate) is_private: bool,
    /// Stable retry key containing at most 128 ASCII letters, digits, '.', '_', '-', or ':'.
    #[arg(long)]
    pub(crate) idempotency_key: Option<String>,
    /// Read the complete camelCase note input object from a UTF-8 JSON file.
    #[arg(
        long,
        value_name = "PATH",
        conflicts_with_all = [
            "input_stdin",
            "title",
            "body",
            "is_private",
            "idempotency_key"
        ]
    )]
    pub(crate) input_file: Option<PathBuf>,
    /// Read the complete camelCase note input object from stdin.
    #[arg(
        long,
        conflicts_with_all = [
            "input_file",
            "password_stdin",
            "title",
            "body",
            "is_private",
            "idempotency_key"
        ]
    )]
    pub(crate) input_stdin: bool,
    /// Read the account password from stdin when no local unlock is available.
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
    /// Work-list UUID containing the note.
    #[arg(long)]
    pub(crate) work_list_id: Uuid,
    /// Note UUID.
    #[arg(long)]
    pub(crate) note_id: Uuid,
    /// Replace the note title.
    #[arg(long)]
    pub(crate) title: Option<String>,
    /// Replace the Markdown note body.
    #[arg(long)]
    pub(crate) body: Option<String>,
    /// Read the complete camelCase note patch from a UTF-8 JSON file.
    #[arg(
        long,
        value_name = "PATH",
        conflicts_with_all = ["input_stdin", "title", "body"]
    )]
    pub(crate) input_file: Option<PathBuf>,
    /// Read the complete camelCase note patch from stdin.
    #[arg(
        long,
        conflicts_with_all = ["input_file", "password_stdin", "title", "body"]
    )]
    pub(crate) input_stdin: bool,
    /// Read the account password from stdin when no local unlock is available.
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
    /// Work-list UUID containing the note.
    #[arg(long)]
    pub(crate) work_list_id: Uuid,
    /// Note UUID.
    #[arg(long)]
    pub(crate) note_id: Uuid,
    /// Read an optional audit patch from a UTF-8 JSON file.
    #[arg(long, value_name = "PATH", conflicts_with = "input_stdin")]
    pub(crate) input_file: Option<PathBuf>,
    /// Read an optional audit patch from stdin.
    #[arg(long, conflicts_with = "input_file")]
    pub(crate) input_stdin: bool,
}

#[derive(Args, Debug)]
pub(crate) struct TaskAttachmentUploadArgsCli {
    /// Work-list UUID containing the task.
    #[arg(long)]
    pub(crate) work_list_id: Uuid,
    /// Task UUID.
    #[arg(long)]
    pub(crate) task_id: Uuid,
    #[arg(
        long,
        help = "Current-working-directory-relative regular file to upload (absolute paths, parent traversal, and symlinks are rejected)"
    )]
    pub(crate) file: PathBuf,
    /// Override the attachment file name stored in the encrypted task.
    #[arg(long)]
    pub(crate) file_name: Option<String>,
    /// Override the detected MIME content type.
    #[arg(long)]
    pub(crate) content_type: Option<String>,
    /// Read the account password from stdin when no local unlock is available.
    #[arg(long)]
    pub(crate) password_stdin: bool,
}

#[derive(Args, Debug)]
pub(crate) struct TaskAttachmentDeleteArgsCli {
    /// Work-list UUID containing the task.
    #[arg(long)]
    pub(crate) work_list_id: Uuid,
    /// Task UUID.
    #[arg(long)]
    pub(crate) task_id: Uuid,
    /// Attachment UUID.
    #[arg(long)]
    pub(crate) attachment_id: Uuid,
    /// Read the account password from stdin when no local unlock is available.
    #[arg(long)]
    pub(crate) password_stdin: bool,
}

#[derive(Args, Debug)]
pub(crate) struct TaskAttachmentReadArgsCli {
    /// Work-list UUID containing the task.
    #[arg(long)]
    pub(crate) work_list_id: Uuid,
    /// Task UUID.
    #[arg(long)]
    pub(crate) task_id: Uuid,
    /// Attachment UUID.
    #[arg(long)]
    pub(crate) attachment_id: Uuid,
    /// Read the account password from stdin when no local unlock is available.
    #[arg(long)]
    pub(crate) password_stdin: bool,
}

#[derive(Args, Debug)]
pub(crate) struct TaskAttachmentDownloadArgsCli {
    /// Work-list UUID containing the task.
    #[arg(long)]
    pub(crate) work_list_id: Uuid,
    /// Task UUID.
    #[arg(long)]
    pub(crate) task_id: Uuid,
    /// Attachment UUID.
    #[arg(long)]
    pub(crate) attachment_id: Uuid,
    #[arg(
        long,
        help = "Current-working-directory-relative output path (absolute paths, parent traversal, and symlinks are rejected)"
    )]
    pub(crate) output: Option<PathBuf>,
    /// Replace an existing output file.
    #[arg(long)]
    pub(crate) force: bool,
    /// Read the account password from stdin when no local unlock is available.
    #[arg(long)]
    pub(crate) password_stdin: bool,
}

#[derive(Subcommand, Debug)]
pub(crate) enum AuthCommand {
    /// Sign in with an email and password, optionally completing MFA.
    Login {
        /// Account email. Required with --non-interactive.
        #[arg(long)]
        email: Option<String>,
        #[arg(
            long,
            help = "Read login input from stdin: trimmed password on line 1 and optional exact authenticator or backup code on line 2"
        )]
        password_stdin: bool,
    },
    /// Start or refresh the memory-only local unlock daemon.
    Unlock {
        /// Number of seconds before the memory-only unlock expires.
        #[arg(long, default_value_t = 8 * 60 * 60)]
        ttl_seconds: u64,
        /// Read the account password from stdin.
        #[arg(long)]
        password_stdin: bool,
    },
    /// Clear the current profile's daemon unlock and stop the daemon.
    Lock,
    /// Store or clear a decrypted data-key bootstrap in the platform keychain.
    Keychain {
        #[command(subcommand)]
        command: KeychainCommand,
    },
    /// Revoke the remote session and clear this profile's local credentials.
    Logout,
    /// Inspect credentials, token expiry, daemon state, and keychain state.
    Status,
}

#[derive(Subcommand, Debug)]
pub(crate) enum KeychainCommand {
    /// Store a decrypted data-key bootstrap in the platform keychain.
    Store {
        /// Read the account password from stdin.
        #[arg(long)]
        password_stdin: bool,
    },
    /// Remove this profile's platform-keychain bootstrap.
    Clear,
}

fn parse_priority(value: &str) -> Result<i8, String> {
    match value {
        "1" => Ok(1),
        "3" => Ok(3),
        "5" => Ok(5),
        "8" => Ok(8),
        _ => Err("priority must be one of 1, 3, 5, or 8".to_string()),
    }
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
