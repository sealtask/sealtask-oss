use chrono::{DateTime, Utc};
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(
    name = "worklist",
    version,
    about = "CLI for working with SealTask tasks, comments, and decrypted workspace data"
)]
pub(crate) struct Cli {
    #[arg(
        long,
        env = "WORKLIST_API_URL",
        default_value = "https://sealtask.com",
        global = true
    )]
    pub(crate) api_url: String,

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
    Read(TaskAttachmentReadArgsCli),
    Download(TaskAttachmentDownloadArgsCli),
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
    #[arg(long)]
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
