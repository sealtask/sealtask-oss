use crate::args::{Cli, CompletionShell};
use crate::output::{CliResult, write_stdout, write_stdout_line};
use crate::table::sanitize_cell;
use clap::{Arg, Command, CommandFactory};
use clap_complete::{Shell, generate};
use sealtask_client_core::PublicError;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub(crate) fn command() -> Command {
    configure_command(Cli::command(), &[])
}

pub(crate) fn print_root_help(color: clap::ColorChoice) -> CliResult<()> {
    let mut command = command().color(color);
    let help = command.render_help();
    write_stdout(format_args!("{help}"))
}

pub(crate) fn print_completion(shell: CompletionShell) -> CliResult<()> {
    let mut command = command();
    let mut output = Vec::new();
    generate(shell.generator(), &mut command, "sealtask", &mut output);
    write_generated_stdout(output, "shell completion")
}

pub(crate) fn print_manpage(path: &[String]) -> CliResult<()> {
    let (command, display_path, page_name) = select_command(path)?;
    let mut output = Vec::new();
    render_manpage(command, &display_path, &page_name, &mut output).map_err(|error| {
        PublicError::unexpected(format!(
            "failed to render manual page for {display_path}: {error}"
        ))
    })?;
    write_generated_stdout(output, "manual page")
}

pub(crate) fn generate_manpages(output_dir: &Path) -> CliResult<()> {
    std::fs::create_dir_all(output_dir).map_err(|error| {
        PublicError::unexpected(format!(
            "failed to create manual page directory {}: {error}",
            output_dir.display()
        ))
    })?;
    let mut command = command().disable_help_subcommand(true);
    command.build();
    generate_manpage_tree(&command, &mut Vec::new(), output_dir).map_err(|error| {
        PublicError::unexpected(format!(
            "failed to generate manual pages beneath {}: {error}",
            output_dir.display()
        ))
    })?;
    write_stdout_line(format_args!(
        "Generated manual pages in {}",
        sanitize_cell(&output_dir.display().to_string())
    ))
}

fn generate_manpage_tree(
    command: &Command,
    path: &mut Vec<String>,
    output_dir: &Path,
) -> std::io::Result<()> {
    if command.is_hide_set() || command.get_name() == "help" {
        return Ok(());
    }

    let display_path = std::iter::once("sealtask")
        .chain(path.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join(" ");
    let page_name = std::iter::once("sealtask")
        .chain(path.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join("-");
    let mut file = File::create(output_dir.join(format!("{page_name}.1")))?;
    render_manpage(command.clone(), &display_path, &page_name, &mut file)?;
    file.flush()?;

    for child in command.get_subcommands() {
        path.push(child.get_name().to_string());
        generate_manpage_tree(child, path, output_dir)?;
        path.pop();
    }
    Ok(())
}

fn render_manpage(
    command: Command,
    display_path: &str,
    page_name: &str,
    writer: &mut dyn Write,
) -> std::io::Result<()> {
    let mut output = Vec::new();
    clap_mangen::Man::new(
        command
            .bin_name(display_path)
            .display_name(page_name)
            .disable_help_subcommand(true),
    )
    .title(page_name)
    .source(format!("SealTask CLI {}", env!("CARGO_PKG_VERSION")))
    .manual("SealTask Manual")
    .render(&mut output)?;
    writer.write_all(&without_trailing_horizontal_whitespace(&output))
}

fn without_trailing_horizontal_whitespace(output: &[u8]) -> Vec<u8> {
    let mut normalized = Vec::with_capacity(output.len());
    for line in output.split_inclusive(|byte| *byte == b'\n') {
        let has_newline = line.last() == Some(&b'\n');
        let mut content_end = line.len() - usize::from(has_newline);
        while content_end > 0 && matches!(line[content_end - 1], b' ' | b'\t') {
            content_end -= 1;
        }
        normalized.extend_from_slice(&line[..content_end]);
        if has_newline {
            normalized.push(b'\n');
        }
    }
    normalized
}

fn write_generated_stdout(output: Vec<u8>, kind: &str) -> CliResult<()> {
    let output = String::from_utf8(output).map_err(|error| {
        PublicError::unexpected(format!("generated {kind} was not valid UTF-8: {error}"))
    })?;
    write_stdout(format_args!("{output}"))
}

fn select_command(path: &[String]) -> Result<(Command, String, String), PublicError> {
    let mut root = command();
    root.build();
    let mut selected = &root;
    let mut canonical_path = Vec::with_capacity(path.len());

    for segment in path {
        let available = visible_subcommand_names(selected);
        let Some(next) = selected.get_subcommands().find(|candidate| {
            !candidate.is_hide_set()
                && candidate.get_name() != "help"
                && (candidate.get_name() == segment
                    || candidate.get_all_aliases().any(|alias| alias == segment))
        }) else {
            return Err(PublicError::validation(format!(
                "unknown command path segment '{}'; available: {}",
                sanitize_cell(segment),
                available.join(", ")
            )));
        };
        canonical_path.push(next.get_name().to_string());
        selected = next;
    }

    let display_path = std::iter::once("sealtask")
        .chain(canonical_path.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join(" ");
    let page_name = std::iter::once("sealtask")
        .chain(canonical_path.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join("-");
    Ok((selected.clone(), display_path, page_name))
}

fn visible_subcommand_names(command: &Command) -> Vec<&str> {
    command
        .get_subcommands()
        .filter(|candidate| !candidate.is_hide_set() && candidate.get_name() != "help")
        .map(Command::get_name)
        .collect()
}

fn configure_command(mut command: Command, parent_path: &[String]) -> Command {
    let mut path = parent_path.to_vec();
    if command.get_name() != "sealtask" {
        path.push(command.get_name().to_string());
    }

    command = command.mut_args(|argument| {
        let heading = argument_heading(&path, &argument);
        argument.help_heading(heading)
    });
    if let Some(examples) = command_examples(&path) {
        command = command.after_long_help(examples);
    }

    let subcommand_names = command
        .get_subcommands()
        .map(|subcommand| subcommand.get_name().to_string())
        .collect::<Vec<_>>();
    for name in subcommand_names {
        let Some(subcommand) = command.find_subcommand_mut(&name) else {
            continue;
        };
        *subcommand = configure_command(subcommand.clone(), &path);
    }
    command
}

fn argument_heading(path: &[String], argument: &Arg) -> &'static str {
    let id = argument.get_id().as_str();
    if path.is_empty() {
        return match id {
            "api_url" | "storage_origin" | "connect_timeout" | "read_timeout"
            | "request_timeout" | "retry" | "offline" => "Connection",
            "profile" | "config_dir" => "Profile",
            "json" | "format" | "color" | "pager" | "no_pager" | "progress" | "quiet" => "Output",
            "non_interactive" => "Interaction",
            "verbosity" | "debug" => "Diagnostics",
            _ => "Options",
        };
    }

    match id {
        "project"
        | "work_list_id"
        | "task"
        | "task_id"
        | "note"
        | "note_id"
        | "comment_id"
        | "attachment_id"
        | "section"
        | "section_id"
        | "reference"
        | "scheme_revision_id"
        | "insert_before_task_id"
        | "before" => "Target",
        "title" | "body" | "priority" | "due_at" | "due" | "start_at" | "clear_body"
        | "clear_priority" | "clear_due_at" | "clear_start_at" | "clear_section" | "is_private"
        | "file_name" | "content_type" | "prefix" | "minimum_digits" => "Fields",
        "include_archived" | "include_completed" | "all" => "Filters",
        "input_file" | "input_stdin" | "body_file" | "password_stdin" | "file" => "Input",
        "format" | "json" | "verbose" | "raw" | "output" | "color" | "pager" | "no_pager"
        | "progress" | "quiet" => "Output",
        "yes" | "force" | "confirm" => "Safety",
        "edit" => "Interaction",
        "idempotency_key" | "ttl_seconds" => "Advanced",
        _ => "Options",
    }
}

fn command_examples(path: &[String]) -> Option<&'static str> {
    let path = path.join(" ");
    let examples = match path.as_str() {
        "completion" => {
            "Examples:\n  mkdir -p ~/.zfunc && sealtask completion zsh > ~/.zfunc/_sealtask\n  sealtask completion fish | source"
        }
        "man" => {
            "Examples:\n  sealtask man\n  sealtask man tasks create\n  sealtask man --output-dir ./target/man"
        }
        "info" => "Examples:\n  sealtask info\n  sealtask --json info",
        "schema" => {
            "Examples:\n  sealtask schema tasks create\n  sealtask --json schema tasks create"
        }
        "auth" => "Examples:\n  sealtask auth login\n  sealtask auth status",
        "auth login" => {
            "Examples:\n  sealtask auth login\n  printf '%s\\n' \"$SEALTASK_PASSWORD\" | sealtask auth login --email operator@example.com --password-stdin"
        }
        "auth unlock" => {
            "Examples:\n  sealtask auth unlock --ttl 30m\n  printf '%s\\n' \"$SEALTASK_PASSWORD\" | sealtask auth unlock --password-stdin"
        }
        "auth lock" => "Examples:\n  sealtask auth lock",
        "auth keychain" => {
            "Examples:\n  sealtask auth keychain store\n  sealtask auth keychain clear"
        }
        "auth keychain store" => {
            "Examples:\n  sealtask auth keychain store\n  printf '%s\\n' \"$SEALTASK_PASSWORD\" | sealtask auth keychain store --password-stdin"
        }
        "auth keychain clear" => "Examples:\n  sealtask auth keychain clear",
        "auth logout" => "Examples:\n  sealtask auth logout",
        "auth status" => "Examples:\n  sealtask auth status\n  sealtask --json auth status",
        "me" => "Examples:\n  sealtask me\n  sealtask --json me",
        "pick" => {
            "Examples:\n  sealtask pick project\n  sealtask pick project \"Release Engineering\" --scope global\n  sealtask pick task --project \"Operations\""
        }
        "pick project" => {
            "Examples:\n  sealtask pick project\n  sealtask pick project \"Release Engineering\" --scope global\n  sealtask projects get \"$(sealtask pick project --print-selector)\""
        }
        "pick task" => {
            "Examples:\n  sealtask tasks get \"$(sealtask pick task)\"\n  sealtask pick task --project \"Operations\""
        }
        "projects" => {
            "Examples:\n  sealtask projects\n  sealtask projects list --include-archived --details"
        }
        "projects list" => {
            "Examples:\n  sealtask projects list\n  sealtask projects list --details\n  sealtask projects list --include-archived --details"
        }
        "projects get" => {
            "Examples:\n  sealtask projects get \"Release Engineering\"\n  sealtask projects get id:019f42ab"
        }
        "projects archive" => "Examples:\n  sealtask projects archive \"Finished launch\"",
        "projects unarchive" => "Examples:\n  sealtask projects unarchive \"Finished launch\"",
        "projects current" => {
            "Examples:\n  sealtask projects current\n  sealtask projects current --scope global\n  sealtask --json projects current"
        }
        "projects clear" => {
            "Examples:\n  sealtask projects clear\n  sealtask projects clear --scope global"
        }
        "projects sections" => {
            "Examples:\n  sealtask projects sections list\n  sealtask projects sections list --project \"Release Engineering\""
        }
        "projects sections list" => {
            "Examples:\n  sealtask projects sections list\n  sealtask projects sections list --project \"Release Engineering\""
        }
        "projects audit" => {
            "Examples:\n  sealtask projects audit\n  sealtask projects audit \"Release Engineering\" --limit 25\n  sealtask --json projects audit id:019f42ab"
        }
        "tasks" => {
            "Examples:\n  sealtask tasks list\n  sealtask tasks get OPS-184\n  sealtask tasks create --title \"Ship 0.4\" --due tomorrow"
        }
        "tasks list" => {
            "Examples:\n  sealtask tasks list\n  sealtask tasks list --all --sort reference\n  sealtask tasks list --columns reference,project,title,due,status\n  sealtask tasks list --field reference\n  sealtask tasks list --field id\n  sealtask tasks list --field url --web-url https://app.example"
        }
        "tasks get" => {
            "Examples:\n  sealtask tasks get OPS-184\n  sealtask tasks get '#184' --project Operations\n  sealtask tasks get name:OPS-184 --project Operations\n  sealtask tasks get id:019f42ab"
        }
        "tasks resolve" => {
            "Examples:\n  sealtask tasks resolve OPS-184\n  sealtask tasks resolve OPS-184 --work-list-id 019f42ab-0000-7000-8000-000000000000"
        }
        "tasks task-references" => {
            "Examples:\n  sealtask tasks task-references status --work-list-id 019f42ab-0000-7000-8000-000000000000"
        }
        "tasks task-references status" => {
            "Examples:\n  sealtask tasks task-references status --work-list-id 019f42ab-0000-7000-8000-000000000000"
        }
        "tasks task-references repair" => {
            "Examples:\n  sealtask tasks task-references repair --work-list-id 019f42ab-0000-7000-8000-000000000000 --prefix OPS --minimum-digits 4"
        }
        "tasks task-references quarantine" => {
            "Examples:\n  sealtask tasks task-references quarantine --work-list-id 019f42ab-0000-7000-8000-000000000000 --scheme-revision-id 019f42ab-0000-7000-8000-000000000001 --confirm"
        }
        "tasks watch" => {
            "Examples:\n  sealtask tasks watch --project \"Release Engineering\"\n  sealtask --format jsonl tasks watch --work-list-id 019f42ab-0000-7000-8000-000000000000"
        }
        "tasks create" => {
            "Examples:\n  sealtask tasks create --edit\n  sealtask tasks create --title \"Ship 0.4\" --due tomorrow\n  sealtask tasks create --project Release --section Doing --priority high --title \"Publish artifacts\"\n  sealtask tasks create --input-file ./task.json\n  sealtask --json tasks create --input-file ./task.json --dry-run"
        }
        "tasks edit" => "Examples:\n  sealtask tasks edit \"Release checklist\"",
        "tasks update" => {
            "Examples:\n  sealtask tasks update OPS-184 --priority urgent --due tomorrow\n  sealtask tasks update id:019f42ab --body-file ./release.md\n  sealtask tasks update id:019f42ab --clear-due-at\n  sealtask --json tasks update id:019f42ab --priority urgent --dry-run"
        }
        "tasks move" => {
            "Examples:\n  sealtask tasks move OPS-184 --section Review\n  sealtask tasks move '#184' --before '#185' --project Operations"
        }
        "tasks complete" => "Examples:\n  sealtask tasks complete \"Ship 0.4\"",
        "tasks reopen" => "Examples:\n  sealtask tasks reopen \"Ship 0.4\"",
        "tasks archive" => "Examples:\n  sealtask tasks archive \"Ship 0.4\"",
        "tasks unarchive" => "Examples:\n  sealtask tasks unarchive \"Ship 0.4\"",
        "tasks delete" => {
            "Examples:\n  sealtask tasks delete \"Obsolete task\"\n  sealtask --non-interactive tasks delete id:019f42ab --yes"
        }
        "tasks attachments" => {
            "Examples:\n  sealtask tasks attachments upload \"Ship 0.4\" --file ./release.pdf\n  sealtask tasks attachments read \"Ship 0.4\" --attachment-id id:019f42ab"
        }
        "tasks attachments upload" => {
            "Examples:\n  sealtask tasks attachments upload \"Ship 0.4\" --file ./release.pdf\n  sealtask tasks attachments upload \"Ship 0.4\" --file ./notes.txt --content-type text/plain"
        }
        "tasks attachments delete" => {
            "Examples:\n  sealtask tasks attachments delete \"Ship 0.4\" --attachment-id id:019f42ab\n  sealtask --non-interactive tasks attachments delete \"Ship 0.4\" --attachment-id id:019f42ab --yes"
        }
        "tasks attachments read" => {
            "Examples:\n  sealtask tasks attachments read \"Ship 0.4\" --attachment-id id:019f42ab"
        }
        "tasks attachments download" => {
            "Examples:\n  sealtask tasks attachments download \"Ship 0.4\" --attachment-id id:019f42ab\n  sealtask tasks attachments download \"Ship 0.4\" --attachment-id id:019f42ab --output ./release.pdf"
        }
        "stats" => "Examples:\n  sealtask stats\n  sealtask --json stats",
        "activity follow" => {
            "Examples:\n  sealtask activity follow\n  sealtask activity follow --since 30m --interval 10s\n  sealtask --format jsonl activity follow"
        }
        "browse" => {
            "Examples:\n  sealtask browse\n  sealtask --offline browse --include-completed\n  sealtask browse --include-archived"
        }
        "cache" => {
            "Examples:\n  sealtask cache status\n  sealtask cache verify\n  sealtask cache clear"
        }
        "cache status" => "Examples:\n  sealtask cache status\n  sealtask --json cache status",
        "cache verify" => {
            "Examples:\n  sealtask cache verify\n  printf '%s\\n' \"$SEALTASK_PASSWORD\" | sealtask cache verify --password-stdin"
        }
        "cache clear" => "Examples:\n  sealtask cache clear",
        "batch" => {
            "Examples:\n  sealtask batch run --input ./operations.jsonl --dry-run\n  sealtask --format jsonl batch run --input ./operations.jsonl"
        }
        "batch run" => {
            "Examples:\n  sealtask --format jsonl batch run --input ./operations.jsonl\n  sealtask --format jsonl batch run --input - --dry-run\n  sealtask --format jsonl batch run --input ./operations.jsonl --checkpoint \"$HOME/.local/state/sealtask/batch/run.json\"\n  sealtask --format jsonl batch run --input ./operations.jsonl --checkpoint \"$HOME/.local/state/sealtask/batch/run.json\" --resume"
        }
        "doctor" => {
            "Examples:\n  sealtask doctor\n  sealtask doctor --offline\n  sealtask doctor --strict"
        }
        "config" => "Examples:\n  sealtask config show --resolved",
        "config show" => {
            "Examples:\n  sealtask config show\n  sealtask config show --resolved\n  sealtask --json config show --resolved"
        }
        "profile" => "Examples:\n  sealtask profile list\n  sealtask profile use build-agent",
        "profile list" => "Examples:\n  sealtask profile list\n  sealtask --json profile list",
        "profile use" => "Examples:\n  sealtask profile use build-agent",
        "comments" => {
            "Examples:\n  sealtask comments list \"Ship 0.4\"\n  sealtask comments create \"Ship 0.4\" --body \"Ready for review\""
        }
        "comments list" => {
            "Examples:\n  sealtask comments list OPS-184\n  sealtask --json comments list id:019f42ab"
        }
        "comments create" => {
            "Examples:\n  sealtask comments create \"Ship 0.4\" --body \"Ready for review\"\n  sealtask comments create \"Ship 0.4\" --body-file -\n  sealtask comments create \"Ship 0.4\" --input-file ./comment.json"
        }
        "comments update" => {
            "Examples:\n  sealtask comments update \"Ship 0.4\" --comment-id id:019f42ab --body \"Approved\""
        }
        "comments delete" => {
            "Examples:\n  sealtask comments delete \"Ship 0.4\" --comment-id id:019f42ab\n  sealtask --non-interactive comments delete \"Ship 0.4\" --comment-id id:019f42ab --yes"
        }
        "notes" => {
            "Examples:\n  sealtask notes list\n  sealtask notes create --title Runbook --body \"Recovery steps\""
        }
        "notes list" => {
            "Examples:\n  sealtask notes list\n  sealtask notes list --project Operations"
        }
        "notes get" => {
            "Examples:\n  sealtask notes get \"Incident runbook\"\n  sealtask notes get id:019f42ab"
        }
        "notes create" => {
            "Examples:\n  sealtask notes create --title \"Incident runbook\" --body \"Recovery steps\"\n  sealtask notes create --private --input-file ./note.json"
        }
        "notes edit" => "Examples:\n  sealtask notes edit \"Incident runbook\"",
        "notes update" => {
            "Examples:\n  sealtask notes update \"Incident runbook\" --body \"Updated recovery steps\""
        }
        "notes delete" => {
            "Examples:\n  sealtask notes delete \"Obsolete runbook\"\n  sealtask --non-interactive notes delete id:019f42ab --yes"
        }
        _ => return None,
    };
    Some(examples)
}

impl CompletionShell {
    const fn generator(self) -> Shell {
        match self {
            Self::Bash => Shell::Bash,
            Self::Zsh => Shell::Zsh,
            Self::Fish => Shell::Fish,
            Self::PowerShell => Shell::PowerShell,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_visible_leaf_command_has_examples() {
        fn visit(command: &Command, path: &mut Vec<String>, missing: &mut Vec<String>) {
            if command.get_name() == "help" || command.is_hide_set() {
                return;
            }
            let visible_children = command
                .get_subcommands()
                .filter(|child| child.get_name() != "help" && !child.is_hide_set())
                .collect::<Vec<_>>();
            if visible_children.is_empty()
                && command.get_name() != "sealtask"
                && command.get_after_long_help().is_none()
            {
                missing.push(path.join(" "));
            }
            if visible_children.is_empty()
                && command.get_name() != "sealtask"
                && let Some(examples) = command.get_after_long_help()
            {
                for line in examples.to_string().lines().skip(1) {
                    if !line.trim().is_empty() {
                        assert!(
                            line.contains("sealtask"),
                            "example for {} is not runnable: {line}",
                            path.join(" ")
                        );
                    }
                }
            }
            for child in visible_children {
                path.push(child.get_name().to_string());
                visit(child, path, missing);
                path.pop();
            }
        }

        let mut command = command();
        command.build();
        let mut missing = Vec::new();
        visit(&command, &mut Vec::new(), &mut missing);
        assert!(
            missing.is_empty(),
            "leaf commands missing examples: {missing:?}"
        );
    }

    #[test]
    fn task_create_help_is_progressively_grouped() {
        let command = command();
        let tasks = command
            .get_subcommands()
            .find(|child| child.get_name() == "tasks")
            .expect("tasks");
        let mut create = tasks
            .get_subcommands()
            .find(|child| child.get_name() == "create")
            .expect("tasks create")
            .clone()
            .bin_name("sealtask tasks create");
        let help = create.render_long_help().to_string();
        for heading in ["Target:", "Fields:", "Input:", "Advanced:", "Examples:"] {
            assert!(help.contains(heading), "missing {heading} in:\n{help}");
        }
    }

    #[test]
    fn task_get_help_documents_reference_selectors_and_automation_identity() {
        let command = command();
        let tasks = command
            .get_subcommands()
            .find(|child| child.get_name() == "tasks")
            .expect("tasks");
        let mut get = tasks
            .get_subcommands()
            .find(|child| child.get_name() == "get")
            .expect("tasks get")
            .clone()
            .bin_name("sealtask tasks get");
        let help = get.render_long_help().to_string();

        for expected in ["OPS-184", "#184", "name:OPS-184", "id:019f42ab"] {
            assert!(help.contains(expected), "missing {expected:?} in:\n{help}");
        }
    }

    #[test]
    fn all_supported_shells_generate_nonempty_scripts() {
        for shell in [
            CompletionShell::Bash,
            CompletionShell::Zsh,
            CompletionShell::Fish,
            CompletionShell::PowerShell,
        ] {
            let mut command = command();
            let mut output = Vec::new();
            generate(shell.generator(), &mut command, "sealtask", &mut output);
            assert!(
                output.len() > 100,
                "{shell:?} completion was unexpectedly short"
            );
        }
    }

    #[test]
    fn nested_manual_uses_canonical_command_name() {
        let (command, display_path, page_name) =
            select_command(&["tasks".to_string(), "create".to_string()]).expect("select command");
        assert_eq!(display_path, "sealtask tasks create");
        assert_eq!(page_name, "sealtask-tasks-create");

        let (command_from_alias, display_from_alias, _) =
            select_command(&["lists".to_string(), "get".to_string()]).expect("select alias");
        assert_eq!(command.get_name(), "create");
        assert_eq!(command_from_alias.get_name(), "get");
        assert_eq!(display_from_alias, "sealtask projects get");
    }

    #[test]
    fn generated_manual_pages_have_no_trailing_horizontal_whitespace() {
        let (command, display_path, page_name) =
            select_command(&["tasks".to_string(), "create".to_string()]).expect("select command");
        let mut output = Vec::new();
        render_manpage(command, &display_path, &page_name, &mut output).expect("render manual");
        assert!(
            output
                .split(|byte| *byte == b'\n')
                .all(|line| !matches!(line.last(), Some(b' ' | b'\t')))
        );
    }
}
