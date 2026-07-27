use crate::human_input::parse_priority;
use crate::selectors::{EntitySelector, IdSelector};
use chrono::{DateTime, Utc};
use clap::{ArgAction, ArgGroup, Args, Parser, Subcommand, ValueEnum};
use std::fmt;
use std::path::{Path, PathBuf};
use uuid::Uuid;

fn task_target_group() -> ArgGroup {
    ArgGroup::new("task_target")
        .required(true)
        .multiple(false)
        .args(["task", "task_id"])
}

fn note_target_group() -> ArgGroup {
    ArgGroup::new("note_target")
        .required(true)
        .multiple(false)
        .args(["note", "note_id"])
}

#[derive(Parser, Debug)]
#[command(
    name = "sealtask",
    version,
    about = "CLI for working with SealTask tasks, comments, notes, attachments, and decrypted workspace data",
    after_help = "Get started:\n  sealtask auth login\n  sealtask auth status\n  sealtask projects list\n  sealtask tasks list --all\n\nRun 'sealtask help <command>' for command-specific help."
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

    /// Select human-readable, finite JSON, or streaming JSON Lines output.
    #[arg(
        long = "format",
        global = true,
        value_enum,
        value_name = "FORMAT",
        conflicts_with = "json"
    )]
    pub(crate) format: Option<OutputArg>,

    /// Control colors in human-readable output.
    #[arg(
        long,
        env = "SEALTASK_COLOR",
        value_enum,
        default_value_t = ColorArg::Auto,
        global = true,
        value_name = "WHEN"
    )]
    pub(crate) color: ColorArg,

    /// Control paging of long human-readable output.
    #[arg(
        long,
        env = "SEALTASK_PAGER_MODE",
        value_enum,
        default_value_t = PagerArg::Auto,
        global = true,
        value_name = "WHEN"
    )]
    pub(crate) pager: PagerArg,

    /// Disable paging (equivalent to --pager never).
    #[arg(long, global = true)]
    pub(crate) no_pager: bool,

    /// Control delayed progress indicators on stderr.
    #[arg(
        long,
        env = "SEALTASK_PROGRESS",
        value_enum,
        default_value_t = ProgressArg::Auto,
        global = true,
        value_name = "WHEN"
    )]
    pub(crate) progress: ProgressArg,

    /// Suppress automatic paging, progress, and successful mutation acknowledgements.
    #[arg(
        short = 'q',
        long,
        global = true,
        conflicts_with_all = ["verbosity", "debug"]
    )]
    pub(crate) quiet: bool,

    /// Never prompt; fail with an actionable validation error when input is missing.
    #[arg(long, global = true)]
    pub(crate) non_interactive: bool,

    /// Emit redacted operator telemetry to stderr; repeat for more detail.
    #[arg(short = 'v', action = ArgAction::Count, global = true)]
    pub(crate) verbosity: u8,

    /// Emit maximum redacted diagnostic telemetry to stderr.
    #[arg(long, global = true)]
    pub(crate) debug: bool,

    /// Maximum time to establish a control-plane connection (for example 5s).
    #[arg(
        long,
        env = "SEALTASK_CONNECT_TIMEOUT",
        global = true,
        value_name = "DURATION"
    )]
    pub(crate) connect_timeout: Option<String>,

    /// Maximum idle time while reading a control-plane response (for example 30s).
    #[arg(
        long,
        env = "SEALTASK_READ_TIMEOUT",
        global = true,
        value_name = "DURATION"
    )]
    pub(crate) read_timeout: Option<String>,

    /// Maximum total time for one control-plane request (for example 1m).
    #[arg(
        long,
        env = "SEALTASK_REQUEST_TIMEOUT",
        global = true,
        value_name = "DURATION"
    )]
    pub(crate) request_timeout: Option<String>,

    /// Retry replay-safe API requests after transient failures.
    #[arg(
        long,
        env = "SEALTASK_RETRY",
        global = true,
        default_value_t = 2,
        value_name = "COUNT",
        value_parser = clap::value_parser!(u8).range(0..=10)
    )]
    pub(crate) retry: u8,

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
    Jsonl,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
pub(crate) enum ColorArg {
    #[default]
    Auto,
    Always,
    Never,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
pub(crate) enum PagerArg {
    #[default]
    Auto,
    Always,
    Never,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
pub(crate) enum ProgressArg {
    #[default]
    Auto,
    Always,
    Never,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum CompletionShell {
    Bash,
    Zsh,
    Fish,
    #[value(name = "powershell")]
    PowerShell,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, ValueEnum)]
pub(crate) enum TaskListColumnArg {
    Id,
    Title,
    Project,
    #[value(name = "project-id")]
    ProjectId,
    Priority,
    Due,
    Status,
    Comments,
    Created,
    Updated,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum TaskListSortArg {
    Id,
    Title,
    Project,
    Priority,
    Due,
    Status,
    Created,
    Updated,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum TaskListFieldArg {
    Id,
    Title,
    Url,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Command {
    /// Generate a shell completion script without reading configuration or credentials.
    Completion {
        /// Shell whose completion script should be written to stdout.
        #[arg(value_enum)]
        shell: CompletionShell,
    },
    /// Render a manual page for the root command or a nested command path.
    Man {
        /// Optional nested command path, for example: tasks create.
        #[arg(value_name = "COMMAND", num_args = 0.., conflicts_with = "output_dir")]
        command: Vec<String>,
        /// Generate the root and every visible subcommand manual beneath this directory.
        #[arg(long, value_name = "DIR")]
        output_dir: Option<PathBuf>,
    },
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
    /// Fuzzy-pick an entity while printing only a reusable opaque selector.
    Pick {
        #[command(subcommand)]
        command: PickCommand,
    },
    /// List, inspect, select, archive, or restore projects.
    #[command(name = "projects", visible_alias = "lists")]
    Projects {
        #[arg(long = "verbose", hide = true)]
        legacy_verbose: bool,
        /// Include archived projects.
        #[arg(long)]
        include_archived: bool,
        /// Read the account password from stdin when no local unlock is available.
        #[arg(long)]
        password_stdin: bool,
        #[arg(long, hide = true)]
        raw: bool,
        #[command(subcommand)]
        command: Option<ProjectsCommand>,
    },
    /// List, inspect, create, update, move, or delete tasks.
    Tasks {
        #[command(subcommand)]
        command: TasksCommand,
    },
    /// Show current dashboard task counts.
    Stats,
    /// Inspect or continuously follow recent account activity.
    Activity {
        #[command(subcommand)]
        command: ActivityCommand,
    },
    /// Validate, execute, and safely resume task mutations from JSON Lines.
    Batch {
        #[command(subcommand)]
        command: BatchCommand,
    },
    /// Diagnose local state, authentication, unlock, and API connectivity.
    Doctor {
        /// Run local checks only and make no network requests.
        #[arg(long)]
        offline: bool,
        /// Exit unsuccessfully when any check warns.
        #[arg(long)]
        strict: bool,
        /// Inspect the platform keychain (may trigger an operating-system prompt).
        #[arg(long)]
        include_keychain: bool,
    },
    /// Inspect resolved operator configuration.
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    /// List profiles or change the persisted default profile.
    Profile {
        #[command(subcommand)]
        command: ProfileCommand,
    },
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
pub(crate) enum BatchCommand {
    /// Run a strict versioned JSONL task-mutation batch.
    Run(BatchRunArgs),
}

#[derive(Args)]
pub(crate) struct BatchRunArgs {
    /// JSONL input path, or '-' to read stdin.
    #[arg(long, value_name = "PATH", required = true)]
    pub(crate) input: PathBuf,
    /// Maximum number of unrelated operations in flight.
    #[arg(
        long,
        value_name = "COUNT",
        default_value_t = 4,
        value_parser = clap::value_parser!(u8).range(1..=16)
    )]
    pub(crate) jobs: u8,
    /// Keep scheduling independent operations after an operation fails.
    #[arg(long)]
    pub(crate) continue_on_error: bool,
    /// Durable resumable checkpoint path (Linux and macOS only).
    #[arg(long, value_name = "PATH")]
    pub(crate) checkpoint: Option<PathBuf>,
    /// Resume an existing Linux/macOS checkpoint bound to the exact canonical input.
    #[arg(long)]
    pub(crate) resume: bool,
    /// Resolve and prepare every operation without issuing mutations.
    #[arg(long, conflicts_with_all = ["checkpoint", "resume"])]
    pub(crate) dry_run: bool,
}

impl fmt::Debug for BatchRunArgs {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BatchRunArgs")
            .field("input_stdin", &(self.input == Path::new("-")))
            .field("input_path_present", &(self.input != Path::new("-")))
            .field("jobs", &self.jobs)
            .field("continue_on_error", &self.continue_on_error)
            .field("checkpoint_present", &self.checkpoint.is_some())
            .field("resume", &self.resume)
            .field("dry_run", &self.dry_run)
            .finish()
    }
}

#[derive(Subcommand, Debug)]
pub(crate) enum ConfigCommand {
    /// Show safe configuration values and where they came from.
    Show {
        /// Include resolution sources and defaults.
        #[arg(long)]
        resolved: bool,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum ProfileCommand {
    /// List known local profiles and mark the active one.
    List,
    /// Persist the default profile for future commands.
    Use {
        /// Profile name (letters, digits, '.', '_', and '-' only).
        #[arg(value_name = "NAME")]
        name: String,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum PickCommand {
    /// Pick an accessible project.
    Project {
        /// Include archived projects.
        #[arg(long)]
        include_archived: bool,
        /// Read the account password from stdin when no local unlock is available.
        #[arg(long)]
        password_stdin: bool,
    },
    /// Pick a task in the selected/current project.
    Task {
        /// Project name, UUID, or unique UUID prefix.
        #[arg(long, conflicts_with = "work_list_id")]
        project: Option<EntitySelector>,
        /// Exact project UUID (legacy compatibility).
        #[arg(long)]
        work_list_id: Option<Uuid>,
        /// Include completed tasks.
        #[arg(long)]
        include_completed: bool,
        /// Include archived tasks.
        #[arg(long)]
        include_archived: bool,
        /// Read the account password from stdin when no local unlock is available.
        #[arg(long)]
        password_stdin: bool,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum ProjectsCommand {
    /// List accessible projects.
    List {
        /// Print expanded human-readable project details.
        #[arg(long, alias = "verbose")]
        details: bool,
        /// Include archived projects.
        #[arg(long)]
        include_archived: bool,
        /// Read the account password from stdin when no local unlock is available.
        #[arg(long)]
        password_stdin: bool,
        #[arg(long, hide = true)]
        raw: bool,
    },
    /// Show one decrypted project.
    Get {
        /// Project name, UUID, or unique UUID prefix.
        project: EntitySelector,
        /// Read the account password from stdin when no local unlock is available.
        #[arg(long)]
        password_stdin: bool,
        #[arg(long, hide = true)]
        raw: bool,
    },
    /// Archive a project and make it read-only.
    Archive {
        /// Project name, UUID, or unique UUID prefix.
        project: EntitySelector,
        /// Read the account password from stdin when no local unlock is available.
        #[arg(long)]
        password_stdin: bool,
        #[arg(long, hide = true)]
        raw: bool,
    },
    /// Restore an archived project.
    Unarchive {
        /// Project name, UUID, or unique UUID prefix.
        project: EntitySelector,
        /// Read the account password from stdin when no local unlock is available.
        #[arg(long)]
        password_stdin: bool,
        #[arg(long, hide = true)]
        raw: bool,
    },
    /// Save a project as the current project for this profile.
    Use {
        /// Active project name, UUID, or unique UUID prefix.
        project: EntitySelector,
        /// Read the account password from stdin when no local unlock is available.
        #[arg(long)]
        password_stdin: bool,
    },
    /// Show the saved current project without accessing the network.
    Current,
    /// Clear the saved current project.
    Clear,
    /// Discover sections in a project.
    Sections {
        #[command(subcommand)]
        command: ProjectSectionsCommand,
    },
    /// Show a bounded page of safe project audit metadata.
    Audit {
        /// Project name, UUID, or unique UUID prefix; defaults to the current project.
        #[arg(value_name = "PROJECT", conflicts_with = "work_list_id")]
        project: Option<EntitySelector>,
        /// Exact project UUID (legacy compatibility).
        #[arg(long)]
        work_list_id: Option<Uuid>,
        /// Fetch entries older than this audit-event UUID.
        #[arg(long)]
        cursor: Option<Uuid>,
        /// Maximum number of audit entries to return.
        #[arg(
            long,
            default_value_t = 50,
            value_parser = clap::value_parser!(u32).range(1..=100)
        )]
        limit: u32,
        /// Read the account password from stdin when project-name resolution needs an unlock.
        #[arg(long)]
        password_stdin: bool,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum ProjectSectionsCommand {
    /// List normalized project sections and their IDs.
    List {
        /// Project name, UUID, or unique UUID prefix.
        #[arg(long, conflicts_with = "work_list_id")]
        project: Option<EntitySelector>,
        /// Exact project UUID (legacy compatibility).
        #[arg(long)]
        work_list_id: Option<Uuid>,
        /// Read the account password from stdin when no local unlock is available.
        #[arg(long)]
        password_stdin: bool,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum TasksCommand {
    /// List tasks in the selected/current project, or assigned tasks when none is selected.
    List {
        /// Restrict results to a project name, UUID, or unique UUID prefix.
        #[arg(long, conflicts_with = "work_list_id")]
        project: Option<EntitySelector>,
        /// Restrict results to one exact project UUID (legacy compatibility).
        #[arg(long)]
        work_list_id: Option<Uuid>,
        /// Include tasks in completed sections.
        #[arg(long)]
        include_completed: bool,
        /// Include archived tasks from the selected/current project.
        #[arg(long, conflicts_with = "all")]
        include_archived: bool,
        /// List assigned tasks across all accessible projects.
        #[arg(long, conflicts_with_all = ["project", "work_list_id"])]
        all: bool,
        /// Select and order human table columns (comma-separated or repeatable).
        #[arg(
            long,
            value_enum,
            value_delimiter = ',',
            value_name = "COLUMN",
            conflicts_with_all = ["field", "raw"]
        )]
        columns: Vec<TaskListColumnArg>,
        /// Sort text/date/status ascending, priority high-first, or timestamps newest-first.
        #[arg(long, value_enum, value_name = "FIELD", conflicts_with = "raw")]
        sort: Option<TaskListSortArg>,
        /// Emit one sanitized raw value per task with no headings, totals, or empty-state text.
        #[arg(
            long,
            value_enum,
            value_name = "FIELD",
            conflicts_with_all = ["columns", "raw"]
        )]
        field: Option<TaskListFieldArg>,
        /// Browser application origin; valid only with --field url (defaults to the API origin).
        #[arg(
            long,
            env = "SEALTASK_WEB_URL",
            value_name = "ORIGIN",
            conflicts_with = "raw"
        )]
        web_url: Option<String>,
        /// Read the account password from stdin when no local unlock is available.
        #[arg(long)]
        password_stdin: bool,
        #[arg(long, hide = true)]
        raw: bool,
    },
    /// Show one decrypted task, including comments and attachment metadata.
    #[command(group(task_target_group()))]
    Get {
        /// Task title, UUID, or unique UUID prefix.
        #[arg(value_name = "TASK", conflicts_with = "task_id")]
        task: Option<EntitySelector>,
        /// Exact task UUID (legacy compatibility).
        #[arg(long)]
        task_id: Option<Uuid>,
        /// Project name, UUID, or unique UUID prefix.
        #[arg(long, conflicts_with = "work_list_id")]
        project: Option<EntitySelector>,
        /// Exact project UUID (legacy compatibility).
        #[arg(long)]
        work_list_id: Option<Uuid>,
        /// Read the account password from stdin when no local unlock is available.
        #[arg(long)]
        password_stdin: bool,
        #[arg(long, hide = true)]
        raw: bool,
    },
    /// Resolve an encrypted current or historical task reference locally.
    Resolve(TaskResolveArgsCli),
    /// Install an owner repair or explicitly quarantine unreadable history.
    TaskReferences {
        #[command(subcommand)]
        command: TaskReferencesCommand,
    },
    /// Follow authoritative task changes in one project until interrupted.
    Watch {
        /// Restrict results to a project name, UUID, or unique UUID prefix.
        #[arg(long, conflicts_with = "work_list_id")]
        project: Option<EntitySelector>,
        /// Restrict results to one exact project UUID (legacy compatibility).
        #[arg(long)]
        work_list_id: Option<Uuid>,
        /// Include completed tasks.
        #[arg(long)]
        include_completed: bool,
        /// Include archived tasks.
        #[arg(long)]
        include_archived: bool,
        /// Read the account password from stdin when no local unlock is available.
        #[arg(long)]
        password_stdin: bool,
    },
    /// Create an encrypted task.
    Create(TaskCreateArgsCli),
    /// Edit a task's title and Markdown body in your configured editor.
    Edit(TaskEditArgsCli),
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

#[derive(Args)]
pub(crate) struct TaskResolveArgsCli {
    /// Full encrypted-scheme reference, for example OPS-0042.
    #[arg(value_name = "REFERENCE")]
    pub(crate) reference: String,
    /// Restrict resolution to one canonical work-list UUID.
    #[arg(long, value_parser = parse_canonical_uuid)]
    pub(crate) work_list_id: Option<Uuid>,
    /// Read the account password from stdin when no local unlock is available.
    #[arg(long)]
    pub(crate) password_stdin: bool,
}

impl fmt::Debug for TaskResolveArgsCli {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TaskResolveArgsCli")
            .field("reference", &"<redacted>")
            .field("work_list_id", &self.work_list_id)
            .field("password_stdin", &self.password_stdin)
            .finish()
    }
}

#[derive(Subcommand, Debug)]
pub(crate) enum TaskReferencesCommand {
    /// Inspect verified history and list exact unreadable revision IDs.
    Status(TaskReferenceStatusArgsCli),
    /// Install a valid encrypted current scheme from reserved owner repair capacity.
    Repair(TaskReferenceRepairArgsCli),
    /// Irreversibly exclude one unreadable historical alias from local resolution.
    Quarantine(TaskReferenceQuarantineArgsCli),
}

#[derive(Args, Debug)]
pub(crate) struct TaskReferenceStatusArgsCli {
    /// Work-list UUID whose encrypted scheme history should be inspected.
    #[arg(long, value_parser = parse_canonical_uuid)]
    pub(crate) work_list_id: Uuid,
    /// Read the account password from stdin when no local unlock is available.
    #[arg(long)]
    pub(crate) password_stdin: bool,
}

#[derive(Args)]
pub(crate) struct TaskReferenceRepairArgsCli {
    /// Work-list UUID whose current encrypted scheme needs replacement.
    #[arg(long, value_parser = parse_canonical_uuid)]
    pub(crate) work_list_id: Uuid,
    /// Replacement private prefix, using 2-10 uppercase ASCII letters or digits.
    #[arg(long)]
    pub(crate) prefix: String,
    /// Minimum number of digits displayed after the prefix.
    #[arg(long, default_value_t = 4, value_parser = clap::value_parser!(u8).range(1..=8))]
    pub(crate) minimum_digits: u8,
    /// Read the account password from stdin when no local unlock is available.
    #[arg(long)]
    pub(crate) password_stdin: bool,
}

impl fmt::Debug for TaskReferenceRepairArgsCli {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TaskReferenceRepairArgsCli")
            .field("work_list_id", &self.work_list_id)
            .field("prefix", &"<redacted>")
            .field("minimum_digits", &self.minimum_digits)
            .field("password_stdin", &self.password_stdin)
            .finish()
    }
}

#[derive(Args, Debug)]
pub(crate) struct TaskReferenceQuarantineArgsCli {
    /// Work-list UUID containing the unreadable historical scheme.
    #[arg(long, value_parser = parse_canonical_uuid)]
    pub(crate) work_list_id: Uuid,
    /// Exact historical scheme-revision UUID to quarantine.
    #[arg(long, value_parser = parse_canonical_uuid)]
    pub(crate) scheme_revision_id: Uuid,
    /// Confirm the irreversible loss of this historical alias.
    #[arg(long)]
    pub(crate) confirm: bool,
    /// Read the account password from stdin when no local unlock is available.
    #[arg(long)]
    pub(crate) password_stdin: bool,
}

#[derive(Subcommand, Debug)]
pub(crate) enum ActivityCommand {
    /// Follow new activity using bounded cursor catch-up polling.
    Follow {
        /// Delay between activity polls (for example 2s or 1m).
        #[arg(long, default_value = "5s", value_name = "DURATION")]
        interval: String,
        /// Emit recent history from this window before following new events.
        #[arg(long, default_value = "10m", value_name = "DURATION")]
        since: String,
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
    /// List decrypted notes in a project.
    List {
        /// Project name, UUID, or unique UUID prefix.
        #[arg(long, conflicts_with = "work_list_id")]
        project: Option<EntitySelector>,
        /// Exact project UUID (legacy compatibility).
        #[arg(long)]
        work_list_id: Option<Uuid>,
        /// Read the account password from stdin when no local unlock is available.
        #[arg(long)]
        password_stdin: bool,
    },
    /// Show one decrypted note.
    #[command(group(note_target_group()))]
    Get {
        /// Note title, UUID, or unique UUID prefix.
        #[arg(value_name = "NOTE", conflicts_with = "note_id")]
        note: Option<EntitySelector>,
        /// Exact note UUID (legacy compatibility).
        #[arg(long)]
        note_id: Option<Uuid>,
        /// Project name, UUID, or unique UUID prefix.
        #[arg(long, conflicts_with = "work_list_id")]
        project: Option<EntitySelector>,
        /// Exact project UUID (legacy compatibility).
        #[arg(long)]
        work_list_id: Option<Uuid>,
        /// Read the account password from stdin when no local unlock is available.
        #[arg(long)]
        password_stdin: bool,
    },
    /// Create an encrypted shared or private note.
    Create(NoteCreateArgsCli),
    /// Edit a note's title and Markdown body in your configured editor.
    Edit(NoteEditArgsCli),
    /// Patch an encrypted note.
    Update(NoteUpdateArgsCli),
    /// Permanently delete a note.
    Delete(NoteDeleteArgsCli),
}

#[derive(Subcommand, Debug)]
pub(crate) enum CommentsCommand {
    /// List decrypted comments on a task.
    #[command(group(task_target_group()))]
    List {
        /// Project name, UUID, or unique UUID prefix.
        #[arg(long, conflicts_with = "work_list_id")]
        project: Option<EntitySelector>,
        /// Exact project UUID (legacy compatibility).
        #[arg(long)]
        work_list_id: Option<Uuid>,
        /// Task title, UUID, or unique UUID prefix.
        #[arg(value_name = "TASK", conflicts_with = "task_id")]
        task: Option<EntitySelector>,
        /// Exact task UUID (legacy compatibility).
        #[arg(long)]
        task_id: Option<Uuid>,
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

#[derive(Args)]
pub(crate) struct TaskCreateArgsCli {
    /// Project name, UUID, or unique UUID prefix.
    #[arg(long, conflicts_with = "work_list_id")]
    pub(crate) project: Option<EntitySelector>,
    /// Exact project UUID (legacy compatibility).
    #[arg(long)]
    pub(crate) work_list_id: Option<Uuid>,
    /// Plaintext task title.
    #[arg(long)]
    pub(crate) title: Option<String>,
    /// Plaintext Markdown task body.
    #[arg(long, conflicts_with = "body_file")]
    pub(crate) body: Option<String>,
    /// Read the plaintext Markdown task body from PATH; use '-' for stdin.
    #[arg(
        long,
        value_name = "PATH",
        conflicts_with_all = ["body", "input_file", "input_stdin"]
    )]
    pub(crate) body_file: Option<PathBuf>,
    /// Open your configured editor; --title, --body, and --body-file seed its contents.
    #[arg(long, conflicts_with_all = ["input_file", "input_stdin"])]
    pub(crate) edit: bool,
    /// Task priority: low/p4/1, medium/p3/3, high/p2/5, or urgent/p1/8.
    #[arg(long, value_parser = parse_priority)]
    pub(crate) priority: Option<i8>,
    /// Due time as an RFC 3339 timestamp.
    #[arg(long, conflicts_with = "due")]
    pub(crate) due_at: Option<DateTime<Utc>>,
    /// Human due date in the project's timezone (for example tomorrow or 2026-08-10).
    #[arg(long, conflicts_with = "due_at")]
    pub(crate) due: Option<String>,
    /// Start time as an RFC 3339 timestamp.
    #[arg(long)]
    pub(crate) start_at: Option<DateTime<Utc>>,
    /// Initial section UUID.
    #[arg(long, conflicts_with = "section")]
    pub(crate) section_id: Option<Uuid>,
    /// Initial section name, UUID, or unique UUID prefix.
    #[arg(long, conflicts_with = "section_id")]
    pub(crate) section: Option<EntitySelector>,
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
            "body_file",
            "edit",
            "priority",
            "due_at",
            "due",
            "start_at",
            "section_id",
            "section",
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
            "body_file",
            "edit",
            "priority",
            "due_at",
            "due",
            "start_at",
            "section_id",
            "section",
            "idempotency_key"
        ]
    )]
    pub(crate) input_stdin: bool,
    /// Resolve, validate, and encrypt the request but do not create the task.
    #[arg(long)]
    pub(crate) dry_run: bool,
    /// Read the account password from stdin when no local unlock is available.
    #[arg(long)]
    pub(crate) password_stdin: bool,
}

impl fmt::Debug for TaskCreateArgsCli {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TaskCreateArgsCli")
            .field("project", &self.project)
            .field("work_list_id", &self.work_list_id)
            .field("title_present", &self.title.is_some())
            .field("body_present", &self.body.is_some())
            .field("body_file_present", &self.body_file.is_some())
            .field("edit", &self.edit)
            .field("priority_present", &self.priority.is_some())
            .field("due_at_present", &self.due_at.is_some())
            .field("due_present", &self.due.is_some())
            .field("start_at_present", &self.start_at.is_some())
            .field("section_id_present", &self.section_id.is_some())
            .field("section", &self.section)
            .field("idempotency_key_present", &self.idempotency_key.is_some())
            .field("input_file_present", &self.input_file.is_some())
            .field("input_stdin", &self.input_stdin)
            .field("dry_run", &self.dry_run)
            .field("password_stdin", &self.password_stdin)
            .finish()
    }
}

#[derive(Args, Debug)]
#[command(group(task_target_group()))]
pub(crate) struct TaskEditArgsCli {
    /// Task title, UUID, or unique UUID prefix.
    #[arg(value_name = "TASK", conflicts_with = "task_id")]
    pub(crate) task: Option<EntitySelector>,
    /// Exact task UUID (legacy compatibility).
    #[arg(long)]
    pub(crate) task_id: Option<Uuid>,
    /// Project name, UUID, or unique UUID prefix.
    #[arg(long, conflicts_with = "work_list_id")]
    pub(crate) project: Option<EntitySelector>,
    /// Exact project UUID (legacy compatibility).
    #[arg(long)]
    pub(crate) work_list_id: Option<Uuid>,
    /// Read the account password from stdin when no local unlock is available.
    #[arg(long)]
    pub(crate) password_stdin: bool,
}

#[derive(Args)]
#[command(group(task_target_group()))]
pub(crate) struct TaskUpdateArgsCli {
    /// Task title, UUID, or unique UUID prefix.
    #[arg(value_name = "TASK", conflicts_with = "task_id")]
    pub(crate) task: Option<EntitySelector>,
    /// Exact task UUID (legacy compatibility).
    #[arg(long)]
    pub(crate) task_id: Option<Uuid>,
    /// Project name, UUID, or unique UUID prefix.
    #[arg(long, conflicts_with = "work_list_id")]
    pub(crate) project: Option<EntitySelector>,
    /// Exact project UUID (legacy compatibility).
    #[arg(long)]
    pub(crate) work_list_id: Option<Uuid>,
    /// Replace the task title.
    #[arg(long)]
    pub(crate) title: Option<String>,
    /// Replace the Markdown body.
    #[arg(long, conflicts_with_all = ["body_file", "clear_body"])]
    pub(crate) body: Option<String>,
    /// Read the replacement Markdown task body from PATH; use '-' for stdin.
    #[arg(
        long,
        value_name = "PATH",
        conflicts_with_all = [
            "body",
            "clear_body",
            "input_file",
            "input_stdin"
        ]
    )]
    pub(crate) body_file: Option<PathBuf>,
    /// Remove the task body.
    #[arg(long, conflicts_with = "body_file")]
    pub(crate) clear_body: bool,
    /// Set priority to low/p4/1, medium/p3/3, high/p2/5, or urgent/p1/8.
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
    #[arg(long, conflicts_with_all = ["clear_due_at", "due"])]
    pub(crate) due_at: Option<DateTime<Utc>>,
    /// Set a human due date in the project's timezone.
    #[arg(long, conflicts_with_all = ["due_at", "clear_due_at"])]
    pub(crate) due: Option<String>,
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
    #[arg(long, conflicts_with_all = ["clear_section", "section"])]
    pub(crate) section_id: Option<Uuid>,
    /// Move the task to a section name, UUID, or unique UUID prefix.
    #[arg(long, conflicts_with_all = ["section_id", "clear_section"])]
    pub(crate) section: Option<EntitySelector>,
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
            "body_file",
            "clear_body",
            "priority",
            "clear_priority",
            "due_at",
            "due",
            "clear_due_at",
            "start_at",
            "clear_start_at",
            "section_id",
            "section",
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
            "body_file",
            "clear_body",
            "priority",
            "clear_priority",
            "due_at",
            "due",
            "clear_due_at",
            "start_at",
            "clear_start_at",
            "section_id",
            "section",
            "clear_section"
        ]
    )]
    pub(crate) input_stdin: bool,
    /// Resolve, validate, and encrypt the request but do not update the task.
    #[arg(long)]
    pub(crate) dry_run: bool,
    /// Read the account password from stdin when no local unlock is available.
    #[arg(long)]
    pub(crate) password_stdin: bool,
}

impl fmt::Debug for TaskUpdateArgsCli {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TaskUpdateArgsCli")
            .field("task", &self.task)
            .field("task_id", &self.task_id)
            .field("project", &self.project)
            .field("work_list_id", &self.work_list_id)
            .field("title_present", &self.title.is_some())
            .field("body_present", &self.body.is_some())
            .field("body_file_present", &self.body_file.is_some())
            .field("clear_body", &self.clear_body)
            .field("priority_present", &self.priority.is_some())
            .field("clear_priority", &self.clear_priority)
            .field("due_at_present", &self.due_at.is_some())
            .field("due_present", &self.due.is_some())
            .field("clear_due_at", &self.clear_due_at)
            .field("start_at_present", &self.start_at.is_some())
            .field("clear_start_at", &self.clear_start_at)
            .field("section_id_present", &self.section_id.is_some())
            .field("section", &self.section)
            .field("clear_section", &self.clear_section)
            .field("input_file_present", &self.input_file.is_some())
            .field("input_stdin", &self.input_stdin)
            .field("dry_run", &self.dry_run)
            .field("password_stdin", &self.password_stdin)
            .finish()
    }
}

#[derive(Args, Debug)]
#[command(group(task_target_group()))]
pub(crate) struct TaskMoveArgsCli {
    /// Task title, UUID, or unique UUID prefix.
    #[arg(value_name = "TASK", conflicts_with = "task_id")]
    pub(crate) task: Option<EntitySelector>,
    /// Exact task UUID (legacy compatibility).
    #[arg(long)]
    pub(crate) task_id: Option<Uuid>,
    /// Project name, UUID, or unique UUID prefix.
    #[arg(long, conflicts_with = "work_list_id")]
    pub(crate) project: Option<EntitySelector>,
    /// Exact project UUID (legacy compatibility).
    #[arg(long)]
    pub(crate) work_list_id: Option<Uuid>,
    /// Destination section UUID.
    #[arg(long, conflicts_with = "section")]
    pub(crate) section_id: Option<Uuid>,
    /// Destination section name, UUID, or unique UUID prefix.
    #[arg(long, conflicts_with = "section_id")]
    pub(crate) section: Option<EntitySelector>,
    /// Place the task immediately before this task UUID.
    #[arg(long, conflicts_with = "before")]
    pub(crate) insert_before_task_id: Option<Uuid>,
    /// Place the task immediately before this task title, UUID, or unique UUID prefix.
    #[arg(long, conflicts_with = "insert_before_task_id")]
    pub(crate) before: Option<EntitySelector>,
    /// Read the account password from stdin when no local unlock is available.
    #[arg(long)]
    pub(crate) password_stdin: bool,
}

#[derive(Args, Debug)]
#[command(group(task_target_group()))]
pub(crate) struct TaskCompletionArgsCli {
    /// Task title, UUID, or unique UUID prefix.
    #[arg(value_name = "TASK", conflicts_with = "task_id")]
    pub(crate) task: Option<EntitySelector>,
    /// Exact task UUID (legacy compatibility).
    #[arg(long)]
    pub(crate) task_id: Option<Uuid>,
    /// Project name, UUID, or unique UUID prefix.
    #[arg(long, conflicts_with = "work_list_id")]
    pub(crate) project: Option<EntitySelector>,
    /// Exact project UUID (legacy compatibility).
    #[arg(long)]
    pub(crate) work_list_id: Option<Uuid>,
    /// Read the account password from stdin when no local unlock is available.
    #[arg(long)]
    pub(crate) password_stdin: bool,
}

#[derive(Args, Debug)]
#[command(group(task_target_group()))]
pub(crate) struct TaskArchiveArgsCli {
    /// Task title, UUID, or unique UUID prefix.
    #[arg(value_name = "TASK", conflicts_with = "task_id")]
    pub(crate) task: Option<EntitySelector>,
    /// Exact task UUID (legacy compatibility).
    #[arg(long)]
    pub(crate) task_id: Option<Uuid>,
    /// Project name, UUID, or unique UUID prefix.
    #[arg(long, conflicts_with = "work_list_id")]
    pub(crate) project: Option<EntitySelector>,
    /// Exact project UUID (legacy compatibility).
    #[arg(long)]
    pub(crate) work_list_id: Option<Uuid>,
    /// Read the account password from stdin when no local unlock is available.
    #[arg(long)]
    pub(crate) password_stdin: bool,
}

#[derive(Args, Debug)]
#[command(group(task_target_group()))]
pub(crate) struct TaskUnarchiveArgsCli {
    /// Task title, UUID, or unique UUID prefix.
    #[arg(value_name = "TASK", conflicts_with = "task_id")]
    pub(crate) task: Option<EntitySelector>,
    /// Exact task UUID (legacy compatibility).
    #[arg(long)]
    pub(crate) task_id: Option<Uuid>,
    /// Project name, UUID, or unique UUID prefix.
    #[arg(long, conflicts_with = "work_list_id")]
    pub(crate) project: Option<EntitySelector>,
    /// Exact project UUID (legacy compatibility).
    #[arg(long)]
    pub(crate) work_list_id: Option<Uuid>,
    /// Read the account password from stdin when no local unlock is available.
    #[arg(long)]
    pub(crate) password_stdin: bool,
}

#[derive(Args, Debug)]
#[command(group(task_target_group()))]
pub(crate) struct TaskDeleteArgsCli {
    /// Task title, UUID, or unique UUID prefix.
    #[arg(value_name = "TASK", conflicts_with = "task_id")]
    pub(crate) task: Option<EntitySelector>,
    /// Exact task UUID (legacy compatibility).
    #[arg(long)]
    pub(crate) task_id: Option<Uuid>,
    /// Project name, UUID, or unique UUID prefix.
    #[arg(long, conflicts_with = "work_list_id")]
    pub(crate) project: Option<EntitySelector>,
    /// Exact project UUID (legacy compatibility).
    #[arg(long)]
    pub(crate) work_list_id: Option<Uuid>,
    /// Read an optional audit patch from a UTF-8 JSON file.
    #[arg(long, value_name = "PATH", conflicts_with = "input_stdin")]
    pub(crate) input_file: Option<PathBuf>,
    /// Read an optional audit patch from stdin.
    #[arg(long, conflicts_with = "input_file")]
    pub(crate) input_stdin: bool,
    /// Read the account password from stdin while resolving human selectors.
    #[arg(long, conflicts_with = "input_stdin")]
    pub(crate) password_stdin: bool,
    /// Confirm permanent deletion without prompting.
    #[arg(long)]
    pub(crate) yes: bool,
}

#[derive(Args)]
#[command(group(task_target_group()))]
pub(crate) struct CommentCreateArgsCli {
    /// Task title, UUID, or unique UUID prefix.
    #[arg(value_name = "TASK", conflicts_with = "task_id")]
    pub(crate) task: Option<EntitySelector>,
    /// Exact task UUID (legacy compatibility).
    #[arg(long)]
    pub(crate) task_id: Option<Uuid>,
    /// Project name, UUID, or unique UUID prefix.
    #[arg(long, conflicts_with = "work_list_id")]
    pub(crate) project: Option<EntitySelector>,
    /// Exact project UUID (legacy compatibility).
    #[arg(long)]
    pub(crate) work_list_id: Option<Uuid>,
    /// Plaintext Markdown comment body.
    #[arg(long, conflicts_with = "body_file")]
    pub(crate) body: Option<String>,
    /// Read the plaintext Markdown comment body from PATH; use '-' for stdin.
    #[arg(
        long,
        value_name = "PATH",
        conflicts_with_all = ["body", "input_file", "input_stdin"]
    )]
    pub(crate) body_file: Option<PathBuf>,
    /// Read the complete camelCase comment input object from a UTF-8 JSON file.
    #[arg(
        long,
        value_name = "PATH",
        conflicts_with_all = ["input_stdin", "body", "body_file"]
    )]
    pub(crate) input_file: Option<PathBuf>,
    /// Read the complete camelCase comment input object from stdin.
    #[arg(
        long,
        conflicts_with_all = [
            "input_file",
            "password_stdin",
            "body",
            "body_file"
        ]
    )]
    pub(crate) input_stdin: bool,
    /// Read the account password from stdin when no local unlock is available.
    #[arg(long)]
    pub(crate) password_stdin: bool,
}

impl fmt::Debug for CommentCreateArgsCli {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CommentCreateArgsCli")
            .field("task", &self.task)
            .field("task_id", &self.task_id)
            .field("project", &self.project)
            .field("work_list_id", &self.work_list_id)
            .field("body_present", &self.body.is_some())
            .field("body_file_present", &self.body_file.is_some())
            .field("input_file_present", &self.input_file.is_some())
            .field("input_stdin", &self.input_stdin)
            .field("password_stdin", &self.password_stdin)
            .finish()
    }
}

#[derive(Args)]
#[command(group(task_target_group()))]
pub(crate) struct CommentUpdateArgsCli {
    /// Task title, UUID, or unique UUID prefix.
    #[arg(value_name = "TASK", conflicts_with = "task_id")]
    pub(crate) task: Option<EntitySelector>,
    /// Exact task UUID (legacy compatibility).
    #[arg(long)]
    pub(crate) task_id: Option<Uuid>,
    /// Project name, UUID, or unique UUID prefix.
    #[arg(long, conflicts_with = "work_list_id")]
    pub(crate) project: Option<EntitySelector>,
    /// Exact project UUID (legacy compatibility).
    #[arg(long)]
    pub(crate) work_list_id: Option<Uuid>,
    /// Comment UUID or unique UUID prefix.
    #[arg(long)]
    pub(crate) comment_id: IdSelector,
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

impl fmt::Debug for CommentUpdateArgsCli {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CommentUpdateArgsCli")
            .field("task", &self.task)
            .field("task_id", &self.task_id)
            .field("project", &self.project)
            .field("work_list_id", &self.work_list_id)
            .field("comment_id", &self.comment_id)
            .field("body_present", &self.body.is_some())
            .field("input_file_present", &self.input_file.is_some())
            .field("input_stdin", &self.input_stdin)
            .field("password_stdin", &self.password_stdin)
            .finish()
    }
}

#[derive(Args, Debug)]
#[command(group(task_target_group()))]
pub(crate) struct CommentDeleteArgsCli {
    /// Task title, UUID, or unique UUID prefix.
    #[arg(value_name = "TASK", conflicts_with = "task_id")]
    pub(crate) task: Option<EntitySelector>,
    /// Exact task UUID (legacy compatibility).
    #[arg(long)]
    pub(crate) task_id: Option<Uuid>,
    /// Project name, UUID, or unique UUID prefix.
    #[arg(long, conflicts_with = "work_list_id")]
    pub(crate) project: Option<EntitySelector>,
    /// Exact project UUID (legacy compatibility).
    #[arg(long)]
    pub(crate) work_list_id: Option<Uuid>,
    /// Comment UUID or unique UUID prefix.
    #[arg(long)]
    pub(crate) comment_id: IdSelector,
    /// Read an optional audit patch from a UTF-8 JSON file.
    #[arg(long, value_name = "PATH", conflicts_with = "input_stdin")]
    pub(crate) input_file: Option<PathBuf>,
    /// Read an optional audit patch from stdin.
    #[arg(long, conflicts_with = "input_file")]
    pub(crate) input_stdin: bool,
    /// Read the account password from stdin while resolving human selectors.
    #[arg(long, conflicts_with = "input_stdin")]
    pub(crate) password_stdin: bool,
    /// Confirm permanent deletion without prompting.
    #[arg(long)]
    pub(crate) yes: bool,
}

#[derive(Args)]
pub(crate) struct NoteCreateArgsCli {
    /// Project name, UUID, or unique UUID prefix.
    #[arg(long, conflicts_with = "work_list_id")]
    pub(crate) project: Option<EntitySelector>,
    /// Exact project UUID (legacy compatibility).
    #[arg(long)]
    pub(crate) work_list_id: Option<Uuid>,
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
            .field("idempotency_key_present", &self.idempotency_key.is_some())
            .field("input_file_present", &self.input_file.is_some())
            .field("input_stdin", &self.input_stdin)
            .field("password_stdin", &self.password_stdin)
            .finish()
    }
}

#[derive(Args, Debug)]
#[command(group(note_target_group()))]
pub(crate) struct NoteEditArgsCli {
    /// Note title, UUID, or unique UUID prefix.
    #[arg(value_name = "NOTE", conflicts_with = "note_id")]
    pub(crate) note: Option<EntitySelector>,
    /// Exact note UUID (legacy compatibility).
    #[arg(long)]
    pub(crate) note_id: Option<Uuid>,
    /// Project name, UUID, or unique UUID prefix.
    #[arg(long, conflicts_with = "work_list_id")]
    pub(crate) project: Option<EntitySelector>,
    /// Exact project UUID (legacy compatibility).
    #[arg(long)]
    pub(crate) work_list_id: Option<Uuid>,
    /// Read the account password from stdin when no local unlock is available.
    #[arg(long)]
    pub(crate) password_stdin: bool,
}

#[derive(Args)]
#[command(group(note_target_group()))]
pub(crate) struct NoteUpdateArgsCli {
    /// Note title, UUID, or unique UUID prefix.
    #[arg(value_name = "NOTE", conflicts_with = "note_id")]
    pub(crate) note: Option<EntitySelector>,
    /// Exact note UUID (legacy compatibility).
    #[arg(long)]
    pub(crate) note_id: Option<Uuid>,
    /// Project name, UUID, or unique UUID prefix.
    #[arg(long, conflicts_with = "work_list_id")]
    pub(crate) project: Option<EntitySelector>,
    /// Exact project UUID (legacy compatibility).
    #[arg(long)]
    pub(crate) work_list_id: Option<Uuid>,
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
#[command(group(note_target_group()))]
pub(crate) struct NoteDeleteArgsCli {
    /// Note title, UUID, or unique UUID prefix.
    #[arg(value_name = "NOTE", conflicts_with = "note_id")]
    pub(crate) note: Option<EntitySelector>,
    /// Exact note UUID (legacy compatibility).
    #[arg(long)]
    pub(crate) note_id: Option<Uuid>,
    /// Project name, UUID, or unique UUID prefix.
    #[arg(long, conflicts_with = "work_list_id")]
    pub(crate) project: Option<EntitySelector>,
    /// Exact project UUID (legacy compatibility).
    #[arg(long)]
    pub(crate) work_list_id: Option<Uuid>,
    /// Read an optional audit patch from a UTF-8 JSON file.
    #[arg(long, value_name = "PATH", conflicts_with = "input_stdin")]
    pub(crate) input_file: Option<PathBuf>,
    /// Read an optional audit patch from stdin.
    #[arg(long, conflicts_with = "input_file")]
    pub(crate) input_stdin: bool,
    /// Read the account password from stdin while resolving human selectors.
    #[arg(long, conflicts_with = "input_stdin")]
    pub(crate) password_stdin: bool,
    /// Confirm permanent deletion without prompting.
    #[arg(long)]
    pub(crate) yes: bool,
}

#[derive(Args, Debug)]
#[command(group(task_target_group()))]
pub(crate) struct TaskAttachmentUploadArgsCli {
    /// Task title, UUID, or unique UUID prefix.
    #[arg(value_name = "TASK", conflicts_with = "task_id")]
    pub(crate) task: Option<EntitySelector>,
    /// Exact task UUID (legacy compatibility).
    #[arg(long)]
    pub(crate) task_id: Option<Uuid>,
    /// Project name, UUID, or unique UUID prefix.
    #[arg(long, conflicts_with = "work_list_id")]
    pub(crate) project: Option<EntitySelector>,
    /// Exact project UUID (legacy compatibility).
    #[arg(long)]
    pub(crate) work_list_id: Option<Uuid>,
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
#[command(group(task_target_group()))]
pub(crate) struct TaskAttachmentDeleteArgsCli {
    /// Task title, UUID, or unique UUID prefix.
    #[arg(value_name = "TASK", conflicts_with = "task_id")]
    pub(crate) task: Option<EntitySelector>,
    /// Exact task UUID (legacy compatibility).
    #[arg(long)]
    pub(crate) task_id: Option<Uuid>,
    /// Project name, UUID, or unique UUID prefix.
    #[arg(long, conflicts_with = "work_list_id")]
    pub(crate) project: Option<EntitySelector>,
    /// Exact project UUID (legacy compatibility).
    #[arg(long)]
    pub(crate) work_list_id: Option<Uuid>,
    /// Attachment UUID or unique UUID prefix.
    #[arg(long)]
    pub(crate) attachment_id: IdSelector,
    /// Read the account password from stdin when no local unlock is available.
    #[arg(long)]
    pub(crate) password_stdin: bool,
    /// Confirm permanent deletion without prompting.
    #[arg(long)]
    pub(crate) yes: bool,
}

#[derive(Args, Debug)]
#[command(group(task_target_group()))]
pub(crate) struct TaskAttachmentReadArgsCli {
    /// Task title, UUID, or unique UUID prefix.
    #[arg(value_name = "TASK", conflicts_with = "task_id")]
    pub(crate) task: Option<EntitySelector>,
    /// Exact task UUID (legacy compatibility).
    #[arg(long)]
    pub(crate) task_id: Option<Uuid>,
    /// Project name, UUID, or unique UUID prefix.
    #[arg(long, conflicts_with = "work_list_id")]
    pub(crate) project: Option<EntitySelector>,
    /// Exact project UUID (legacy compatibility).
    #[arg(long)]
    pub(crate) work_list_id: Option<Uuid>,
    /// Attachment UUID or unique UUID prefix.
    #[arg(long)]
    pub(crate) attachment_id: IdSelector,
    /// Read the account password from stdin when no local unlock is available.
    #[arg(long)]
    pub(crate) password_stdin: bool,
}

#[derive(Args, Debug)]
#[command(group(task_target_group()))]
pub(crate) struct TaskAttachmentDownloadArgsCli {
    /// Task title, UUID, or unique UUID prefix.
    #[arg(value_name = "TASK", conflicts_with = "task_id")]
    pub(crate) task: Option<EntitySelector>,
    /// Exact task UUID (legacy compatibility).
    #[arg(long)]
    pub(crate) task_id: Option<Uuid>,
    /// Project name, UUID, or unique UUID prefix.
    #[arg(long, conflicts_with = "work_list_id")]
    pub(crate) project: Option<EntitySelector>,
    /// Exact project UUID (legacy compatibility).
    #[arg(long)]
    pub(crate) work_list_id: Option<Uuid>,
    /// Attachment UUID or unique UUID prefix.
    #[arg(long)]
    pub(crate) attachment_id: IdSelector,
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
    /// Unlock workspace data in memory for a bounded session.
    Unlock {
        /// Human duration before the memory-only unlock expires (for example 30m or 8h).
        #[arg(long, conflicts_with = "ttl_seconds")]
        ttl: Option<String>,
        /// Number of seconds before the memory-only unlock expires (legacy compatibility).
        #[arg(long, conflicts_with = "ttl")]
        ttl_seconds: Option<u64>,
        /// Read the account password from stdin.
        #[arg(long)]
        password_stdin: bool,
    },
    /// Lock workspace data and stop the in-memory unlock session.
    Lock,
    /// Store or clear this profile's saved unlock key.
    Keychain {
        #[command(subcommand)]
        command: KeychainCommand,
    },
    /// Revoke the remote session and clear this profile's local credentials.
    Logout,
    /// Inspect sign-in, token expiry, workspace-data, and saved-key state.
    Status,
}

#[derive(Subcommand, Debug)]
pub(crate) enum KeychainCommand {
    /// Save this profile's unlock key in the platform keychain.
    Store {
        /// Read the account password from stdin.
        #[arg(long)]
        password_stdin: bool,
    },
    /// Remove this profile's saved unlock key.
    Clear,
}

fn parse_canonical_uuid(value: &str) -> Result<Uuid, String> {
    let parsed = Uuid::parse_str(value).map_err(|_| "must be a UUID".to_string())?;
    if parsed.to_string() != value {
        return Err("must be a canonical lowercase hyphenated UUID".to_string());
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_level_cli_debug_redacts_plaintext_mutation_fields() {
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

        for args in [
            vec![
                "sealtask",
                "tasks",
                "create",
                "--work-list-id",
                "018f4a76-c9f2-7f38-a09a-2ac748db8ee8",
                "--title",
                "canary-task-title-secret",
                "--body",
                "canary-task-body-secret",
            ],
            vec![
                "sealtask",
                "comments",
                "create",
                "--work-list-id",
                "018f4a76-c9f2-7f38-a09a-2ac748db8ee8",
                "--task-id",
                "018f4a76-c9f2-7f38-a09a-2ac748db8ee9",
                "--body",
                "canary-comment-body-secret",
            ],
            vec![
                "sealtask",
                "tasks",
                "update",
                "--work-list-id",
                "018f4a76-c9f2-7f38-a09a-2ac748db8ee8",
                "--task-id",
                "018f4a76-c9f2-7f38-a09a-2ac748db8ee9",
                "--title",
                "canary-task-update-title-secret",
                "--body",
                "canary-task-update-body-secret",
            ],
            vec![
                "sealtask",
                "comments",
                "update",
                "--work-list-id",
                "018f4a76-c9f2-7f38-a09a-2ac748db8ee8",
                "--task-id",
                "018f4a76-c9f2-7f38-a09a-2ac748db8ee9",
                "--comment-id",
                "018f4a76-c9f2-7f38-a09a-2ac748db8ee7",
                "--body",
                "canary-comment-update-body-secret",
            ],
        ] {
            let parsed = Cli::try_parse_from(args.clone()).expect("parse plaintext mutation");
            let debug = format!("{parsed:?}");
            for canary in args.iter().filter(|argument| argument.contains("canary-")) {
                assert!(!debug.contains(canary), "debug leaked {canary}");
            }
            assert!(debug.contains("body_present: true"));
        }
    }

    #[test]
    fn task_reference_resolve_requires_canonical_uuid_and_redacts_reference_debug() {
        let reference = "CANARY-0042";
        let parsed = Cli::try_parse_from([
            "sealtask",
            "tasks",
            "resolve",
            reference,
            "--work-list-id",
            "018f4a76-c9f2-7f38-a09a-2ac748db8ee8",
        ])
        .expect("parse task reference resolution");
        let debug = format!("{parsed:?}");
        assert!(!debug.contains(reference));
        assert!(debug.contains("<redacted>"));

        assert!(
            Cli::try_parse_from([
                "sealtask",
                "tasks",
                "resolve",
                "OPS-42",
                "--work-list-id",
                "018F4A76-C9F2-7F38-A09A-2AC748DB8EE8",
            ])
            .is_err()
        );
    }

    #[test]
    fn task_reference_repair_redacts_prefix_and_quarantine_requires_explicit_confirmation_flag() {
        let repair = Cli::try_parse_from([
            "sealtask",
            "tasks",
            "task-references",
            "repair",
            "--work-list-id",
            "018f4a76-c9f2-7f38-a09a-2ac748db8ee8",
            "--prefix",
            "CANARY",
        ])
        .expect("parse task reference repair");
        let debug = format!("{repair:?}");
        assert!(!debug.contains("CANARY"));
        assert!(debug.contains("<redacted>"));

        let quarantine = Cli::try_parse_from([
            "sealtask",
            "tasks",
            "task-references",
            "quarantine",
            "--work-list-id",
            "018f4a76-c9f2-7f38-a09a-2ac748db8ee8",
            "--scheme-revision-id",
            "018f4a76-c9f2-7f38-a09a-2ac748db8ee9",
        ])
        .expect("confirmation is enforced by the command runner");
        let Command::Tasks {
            command:
                TasksCommand::TaskReferences {
                    command: TaskReferencesCommand::Quarantine(args),
                },
        } = quarantine.command.expect("command")
        else {
            panic!("expected task-reference quarantine command");
        };
        assert!(!args.confirm);
    }

    #[test]
    fn editor_commands_accept_human_and_legacy_targets() {
        let task = Cli::try_parse_from([
            "sealtask",
            "tasks",
            "edit",
            "Release checklist",
            "--project",
            "Operations",
            "--password-stdin",
        ])
        .expect("parse task editor command");
        assert!(matches!(
            task.command,
            Some(Command::Tasks {
                command: TasksCommand::Edit(TaskEditArgsCli {
                    task: Some(_),
                    project: Some(_),
                    password_stdin: true,
                    ..
                })
            })
        ));

        let note = Cli::try_parse_from([
            "sealtask",
            "notes",
            "edit",
            "--note-id",
            "018f4a76-c9f2-7f38-a09a-2ac748db8ee9",
            "--work-list-id",
            "018f4a76-c9f2-7f38-a09a-2ac748db8ee8",
        ])
        .expect("parse note editor command");
        assert!(matches!(
            note.command,
            Some(Command::Notes {
                command: NotesCommand::Edit(NoteEditArgsCli {
                    note_id: Some(_),
                    work_list_id: Some(_),
                    ..
                })
            })
        ));
    }

    #[test]
    fn task_create_editor_accepts_scalar_seeds_and_redacts_body_file_path() {
        let body_path = "canary-task-create-body-path.md";
        let parsed = Cli::try_parse_from([
            "sealtask",
            "tasks",
            "create",
            "--title",
            "Seed title",
            "--body-file",
            body_path,
            "--edit",
        ])
        .expect("parse task create editor seeds");
        let debug = format!("{parsed:?}");
        assert!(!debug.contains(body_path));
        assert!(debug.contains("body_file_present: true"));
        assert!(debug.contains("edit: true"));

        for args in [
            vec![
                "sealtask",
                "tasks",
                "create",
                "--edit",
                "--input-file",
                "task.json",
            ],
            vec!["sealtask", "tasks", "create", "--edit", "--input-stdin"],
            vec![
                "sealtask",
                "tasks",
                "create",
                "--body",
                "inline",
                "--body-file",
                "body.md",
            ],
            vec![
                "sealtask",
                "tasks",
                "create",
                "--body-file",
                "body.md",
                "--input-file",
                "task.json",
            ],
        ] {
            assert!(
                Cli::try_parse_from(args).is_err(),
                "conflicting task create input parsed successfully"
            );
        }
    }

    #[test]
    fn task_update_and_comment_create_body_files_are_exclusive_and_redacted() {
        let task_body_path = "canary-task-update-body-path.md";
        let task = Cli::try_parse_from([
            "sealtask",
            "tasks",
            "update",
            "Release checklist",
            "--body-file",
            task_body_path,
        ])
        .expect("parse task update body file");
        let task_debug = format!("{task:?}");
        assert!(!task_debug.contains(task_body_path));
        assert!(task_debug.contains("body_file_present: true"));

        let comment_body_path = "canary-comment-create-body-path.md";
        let comment = Cli::try_parse_from([
            "sealtask",
            "comments",
            "create",
            "Release checklist",
            "--body-file",
            comment_body_path,
        ])
        .expect("parse comment create body file");
        let comment_debug = format!("{comment:?}");
        assert!(!comment_debug.contains(comment_body_path));
        assert!(comment_debug.contains("body_file_present: true"));

        for args in [
            vec![
                "sealtask",
                "tasks",
                "update",
                "Release checklist",
                "--body-file",
                "body.md",
                "--clear-body",
            ],
            vec![
                "sealtask",
                "tasks",
                "update",
                "Release checklist",
                "--body-file",
                "body.md",
                "--input-stdin",
            ],
            vec![
                "sealtask",
                "comments",
                "create",
                "Release checklist",
                "--body-file",
                "body.md",
                "--body",
                "inline",
            ],
            vec![
                "sealtask",
                "comments",
                "create",
                "Release checklist",
                "--body-file",
                "body.md",
                "--input-file",
                "comment.json",
            ],
        ] {
            assert!(
                Cli::try_parse_from(args).is_err(),
                "conflicting body input parsed successfully"
            );
        }
    }

    #[test]
    fn batch_paths_are_redacted_from_debug_output() {
        let input_canary = "batch-input-secret-canary.jsonl";
        let checkpoint_canary = "batch-checkpoint-secret-canary.json";
        let parsed = Cli::try_parse_from([
            "sealtask",
            "batch",
            "run",
            "--input",
            input_canary,
            "--checkpoint",
            checkpoint_canary,
            "--jobs",
            "8",
            "--continue-on-error",
        ])
        .expect("parse batch");
        let debug = format!("{parsed:?}");
        assert!(!debug.contains(input_canary));
        assert!(!debug.contains(checkpoint_canary));
        assert!(debug.contains("input_path_present: true"));
        assert!(debug.contains("checkpoint_present: true"));
        assert!(debug.contains("jobs: 8"));
    }
}
