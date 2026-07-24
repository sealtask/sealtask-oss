use crate::output::{
    CliResult, OutputFormat, print_pretty_json, print_simple_result, terminal_block, terminal_line,
};
use sealtask_client_api::{
    CurrentUserResponse, DashboardStatsResponse, TaskDetailResponse, TaskResponse,
    WorkListDetailResponse, WorkListResponse,
};
use sealtask_client_runtime::{
    AgentComment, AgentNote, AgentTaskDetail, AgentTaskSummary, AgentWorkListDetail,
    AgentWorkListSummary, ReadableAttachment,
};
use serde_json::json;
use std::path::Path;

pub(crate) fn print_download_result(
    format: OutputFormat,
    file_name: &str,
    output_path: &Path,
) -> CliResult<()> {
    match format {
        OutputFormat::Json => print_pretty_json(
            &json!({
                "fileName": file_name,
                "outputPath": output_path.display().to_string(),
            }),
            "serializing download result should succeed",
        )?,
        OutputFormat::Table => {
            println!(
                "Saved attachment to {}",
                terminal_line(&output_path.display().to_string())
            );
        }
    }
    Ok(())
}

pub(crate) fn print_readable_attachment(
    attachment: &ReadableAttachment,
    format: OutputFormat,
) -> CliResult<()> {
    match format {
        OutputFormat::Json => {
            print_pretty_json(attachment, "serializing readable attachment should succeed")?;
        }
        OutputFormat::Table => {
            let text = readable_attachment_terminal_text(&attachment.text);
            print!("{text}");
            if !text.ends_with('\n') {
                println!();
            }
        }
    }
    Ok(())
}

fn readable_attachment_terminal_text(text: &str) -> String {
    terminal_block(text)
}

pub(crate) fn print_comment_json(comment: &AgentComment) -> CliResult<()> {
    print_pretty_json(comment, "serializing comment should succeed")
}

pub(crate) fn print_comments(comments: &[AgentComment], format: OutputFormat) -> CliResult<()> {
    match format {
        OutputFormat::Json => {
            print_pretty_json(comments, "serializing comments should succeed")?;
        }
        OutputFormat::Table => {
            println!("{:<36}  {:<16}  Comment", "ID", "Updated");
            println!("{}", "-".repeat(96));
            for comment in comments {
                println!(
                    "{:<36}  {:<16}  {}",
                    comment.id,
                    comment.updated_at.format("%Y-%m-%d %H:%M"),
                    truncate(
                        comment
                            .body_markdown
                            .as_deref()
                            .unwrap_or("<unreadable comment>"),
                        40
                    )
                );
            }
            println!("\nTotal: {} comment(s)", comments.len());
        }
    }
    Ok(())
}

pub(crate) fn print_notes(notes: &[AgentNote], format: OutputFormat) -> CliResult<()> {
    match format {
        OutputFormat::Json => print_pretty_json(notes, "serializing notes should succeed")?,
        OutputFormat::Table => {
            println!("{:<36}  {:<8}  {:<40}  Updated", "ID", "Privacy", "Title");
            println!("{}", "-".repeat(108));
            for note in notes {
                println!(
                    "{:<36}  {:<8}  {:<40}  {}",
                    note.id,
                    if note.is_private { "Private" } else { "Shared" },
                    truncate(note.title.as_deref().unwrap_or("<unreadable note>"), 40),
                    note.updated_at.format("%Y-%m-%d %H:%M")
                );
            }
            println!("\nTotal: {} note(s)", notes.len());
        }
    }
    Ok(())
}

pub(crate) fn print_note(note: &AgentNote, format: OutputFormat) -> CliResult<()> {
    match format {
        OutputFormat::Json => print_pretty_json(note, "serializing note should succeed")?,
        OutputFormat::Table => {
            println!("Note");
            println!("{}", "=".repeat(60));
            println!("ID:        {}", note.id);
            println!("Work List: {}", note.work_list_id);
            println!(
                "Privacy:  {}",
                if note.is_private { "Private" } else { "Shared" }
            );
            println!(
                "Title:    {}",
                terminal_line(note.title.as_deref().unwrap_or("-"))
            );
            if let Some(body) = note.body_markdown.as_deref() {
                println!();
                println!("Body");
                println!("{}", "-".repeat(60));
                println!("{}", terminal_block(body));
            }
            if let Some(read_error) = note.read_error.as_ref() {
                println!();
                println!("Read error: {}", terminal_line(&read_error.message));
            }
        }
    }
    Ok(())
}

pub(crate) fn print_delete_result(
    format: OutputFormat,
    entity: &str,
    payload: &serde_json::Value,
    table_message: &str,
) -> CliResult<()> {
    print_simple_result(
        format,
        payload,
        &format!("serializing deleted {entity} should succeed"),
        table_message,
    )
}

pub(crate) fn print_empty_collection(format: OutputFormat, table_message: &str) -> CliResult<()> {
    match format {
        OutputFormat::Json => print_pretty_json(
            &Vec::<serde_json::Value>::new(),
            "serializing empty collection should succeed",
        ),
        OutputFormat::Table => {
            println!("{table_message}");
            Ok(())
        }
    }
}

pub(crate) fn print_user(user: &CurrentUserResponse, format: OutputFormat) -> CliResult<()> {
    match format {
        OutputFormat::Json => print_pretty_json(user, "serializing user should succeed")?,
        OutputFormat::Table => {
            println!("User Information");
            println!("{}", "-".repeat(40));
            println!("ID:          {}", user.id);
            println!("Email:       {}", terminal_line(&user.email));
            println!("Name:        {}", terminal_line(&user.name));
            println!("Timezone:    {}", terminal_line(&user.timezone));
            println!("Theme:       {}", terminal_line(&user.theme_preference));
            println!(
                "Verified:    {}",
                if user.email_verified { "Yes" } else { "No" }
            );
        }
    }
    Ok(())
}

pub(crate) fn print_work_lists(
    lists: &[AgentWorkListSummary],
    format: OutputFormat,
    verbose: bool,
) -> CliResult<()> {
    match format {
        OutputFormat::Json => {
            print_pretty_json(lists, "serializing work lists should succeed")?;
        }
        OutputFormat::Table => {
            if verbose {
                for (index, list) in lists.iter().enumerate() {
                    if index > 0 {
                        println!();
                    }
                    println!("Work List: {}", list.id);
                    println!("{}", "-".repeat(50));
                    println!(
                        "  Title:         {}",
                        terminal_line(list.title.as_deref().unwrap_or("-"))
                    );
                    println!("  Workspace:     {}", list.workspace_id);
                    println!("  Owner:         {}", list.owner_user_id);
                    println!("  Timezone:      {}", terminal_line(&list.timezone));
                    println!("  Sections:      {}", list.section_snapshots.len());
                    println!(
                        "  Lifecycle:     {}",
                        lifecycle_label(list.archived_at.is_some())
                    );
                    println!("  Your role:     {}", terminal_line(&list.membership.role));
                    println!(
                        "  Your status:   {}",
                        terminal_line(&list.membership.status)
                    );
                    if let Some(read_error) = list.read_error.as_ref() {
                        println!("  Read error:    {}", terminal_line(&read_error.message));
                    }
                    println!(
                        "  Updated:       {}",
                        list.updated_at.format("%Y-%m-%d %H:%M:%S UTC")
                    );
                }
                println!("\nTotal: {} work list(s)", lists.len());
            } else {
                println!(
                    "{:<36}  {:<24}  {:<10}  {:<9}  Updated",
                    "ID", "Title", "Role", "Lifecycle"
                );
                println!("{}", "-".repeat(104));
                for list in lists {
                    println!(
                        "{:<36}  {:<24}  {:<10}  {:<9}  {}",
                        list.id,
                        truncate(list.title.as_deref().unwrap_or("-"), 24),
                        list.membership.role,
                        lifecycle_label(list.archived_at.is_some()),
                        list.updated_at.format("%Y-%m-%d %H:%M")
                    );
                }
                println!("\nTotal: {} work list(s)", lists.len());
            }
        }
    }
    Ok(())
}

pub(crate) fn print_work_list_detail(
    detail: &AgentWorkListDetail,
    format: OutputFormat,
) -> CliResult<()> {
    match format {
        OutputFormat::Json => {
            print_pretty_json(detail, "serializing work list detail should succeed")?;
        }
        OutputFormat::Table => {
            println!("Work List");
            println!("{}", "=".repeat(60));
            println!("ID:          {}", detail.work_list.id);
            println!(
                "Title:       {}",
                terminal_line(detail.work_list.title.as_deref().unwrap_or("-"))
            );
            println!("Workspace:   {}", detail.work_list.workspace_id);
            println!("Owner:       {}", detail.work_list.owner_user_id);
            println!("Timezone:    {}", terminal_line(&detail.work_list.timezone));
            println!(
                "Lifecycle:   {}",
                lifecycle_label(detail.work_list.archived_at.is_some())
            );
            println!("Members:     {}", detail.members.len());
            println!(
                "Your role:   {}",
                terminal_line(&detail.work_list.membership.role)
            );
            println!(
                "Your status: {}",
                terminal_line(&detail.work_list.membership.status)
            );
            if let Some(description) = detail.work_list.description.as_deref() {
                println!("Description: {}", terminal_block(description));
            }
            if let Some(read_error) = detail.work_list.read_error.as_ref() {
                println!("Read error:  {}", terminal_line(&read_error.message));
            }
        }
    }
    Ok(())
}

pub(crate) fn print_tasks(tasks: &[AgentTaskSummary], format: OutputFormat) -> CliResult<()> {
    match format {
        OutputFormat::Json => {
            print_pretty_json(tasks, "serializing tasks should succeed")?;
        }
        OutputFormat::Table => {
            println!(
                "{:<36}  {:<40}  {:<3}  {:<10}  Status",
                "ID", "Title", "Pri", "Due"
            );
            println!("{}", "-".repeat(108));
            for task in tasks {
                let priority = task
                    .priority
                    .map(|value| value.to_string())
                    .unwrap_or_default();
                let due = task
                    .due_at
                    .map(|value| value.format("%Y-%m-%d").to_string())
                    .unwrap_or_else(|| "-".to_string());
                let status = if task.is_completed {
                    "Done"
                } else if task.archived_at.is_some() {
                    "Archived"
                } else {
                    "Active"
                };
                println!(
                    "{:<36}  {:<40}  {:<3}  {:<10}  {}",
                    task.id,
                    truncate(task.title.as_deref().unwrap_or("-"), 40),
                    priority,
                    due,
                    status
                );
            }
            println!("\nTotal: {} task(s)", tasks.len());
        }
    }
    Ok(())
}

pub(crate) fn print_task_detail(detail: &AgentTaskDetail, format: OutputFormat) -> CliResult<()> {
    match format {
        OutputFormat::Json => {
            print_pretty_json(detail, "serializing task detail should succeed")?;
        }
        OutputFormat::Table => {
            let task = &detail.task;
            println!("Task");
            println!("{}", "=".repeat(60));
            println!("ID:          {}", task.id);
            println!(
                "Title:       {}",
                terminal_line(task.title.as_deref().unwrap_or("-"))
            );
            println!("Work List:   {}", task.work_list_id);
            if let Some(work_list_title) = task.work_list_title.as_deref() {
                println!("List Title:  {}", terminal_line(work_list_title));
            }
            println!(
                "Status:      {}",
                if task.is_completed { "Done" } else { "Active" }
            );
            if let Some(body) = task.body_markdown.as_deref() {
                println!();
                println!("Body");
                println!("{}", "-".repeat(60));
                println!("{}", terminal_block(body));
            }
            if let Some(read_error) = task.read_error.as_ref() {
                println!();
                println!("Read error: {}", terminal_line(&read_error.message));
            }
            if let Some(attachments) = task.attachments.as_ref()
                && !attachments.is_empty()
            {
                println!();
                println!("Attachments");
                println!("{}", "-".repeat(60));
                println!("{:<36}  {:<24}  Type / Size", "ID", "File");
                println!("{}", "-".repeat(96));
                for attachment in attachments {
                    println!(
                        "{:<36}  {:<24}  {} / {} B",
                        attachment.id,
                        truncate(&attachment.file_name, 24),
                        terminal_line(&attachment.content_type),
                        attachment.size_bytes
                    );
                }
            }
            if !detail.comments.is_empty() {
                println!();
                println!("Comments");
                println!("{}", "-".repeat(60));
                for comment in &detail.comments {
                    println!(
                        "- {}",
                        terminal_block(
                            comment
                                .body_markdown
                                .as_deref()
                                .unwrap_or("<unreadable comment>")
                        )
                    );
                }
            }
        }
    }
    Ok(())
}

pub(crate) fn print_stats(stats: &DashboardStatsResponse, format: OutputFormat) -> CliResult<()> {
    match format {
        OutputFormat::Json => {
            print_pretty_json(stats, "serializing stats should succeed")?;
        }
        OutputFormat::Table => {
            println!("Dashboard Statistics");
            println!("{}", "-".repeat(30));
            println!("Overdue:        {}", stats.tasks_overdue);
            println!("Due today:      {}", stats.tasks_due_today);
            println!("Due this week:  {}", stats.tasks_due_this_week);
            println!("Completed:      {}", stats.completed);
        }
    }
    Ok(())
}

pub(crate) fn print_raw_work_lists(
    lists: &[WorkListResponse],
    format: OutputFormat,
    verbose: bool,
) -> CliResult<()> {
    match format {
        OutputFormat::Json => {
            print_pretty_json(lists, "serializing work lists should succeed")?;
        }
        OutputFormat::Table => {
            if verbose {
                for (index, list) in lists.iter().enumerate() {
                    if index > 0 {
                        println!();
                    }
                    println!("Work List: {}", list.id);
                    println!("{}", "-".repeat(50));
                    println!("  Workspace:     {}", list.workspace_id);
                    println!("  Owner:         {}", list.owner_user_id);
                    println!("  Timezone:      {}", terminal_line(&list.timezone));
                    println!("  Sections:      {}", list.section_snapshots.len());
                    println!(
                        "  Lifecycle:     {}",
                        lifecycle_label(list.archived_at.is_some())
                    );
                    println!("  Your role:     {}", terminal_line(&list.membership.role));
                    println!(
                        "  Your status:   {}",
                        terminal_line(&list.membership.status)
                    );
                }
                println!("\nTotal: {} work list(s)", lists.len());
            } else {
                println!(
                    "{:<36}  {:<10}  {:<8}  {:<9}  Updated",
                    "ID", "Role", "Sections", "Lifecycle"
                );
                println!("{}", "-".repeat(92));
                for list in lists {
                    println!(
                        "{:<36}  {:<10}  {:<8}  {:<9}  {}",
                        list.id,
                        list.membership.role,
                        list.section_snapshots.len(),
                        lifecycle_label(list.archived_at.is_some()),
                        list.updated_at.format("%Y-%m-%d %H:%M")
                    );
                }
                println!("\nTotal: {} work list(s)", lists.len());
            }
        }
    }
    Ok(())
}

pub(crate) fn print_raw_work_list_detail(
    detail: &WorkListDetailResponse,
    format: OutputFormat,
) -> CliResult<()> {
    match format {
        OutputFormat::Json => {
            print_pretty_json(detail, "serializing raw work list detail should succeed")?;
        }
        OutputFormat::Table => {
            println!("Raw Work List");
            println!("{}", "=".repeat(60));
            println!("ID:          {}", detail.work_list.id);
            println!("Workspace:   {}", detail.work_list.workspace_id);
            println!("Owner:       {}", detail.work_list.owner_user_id);
            println!(
                "Lifecycle:   {}",
                lifecycle_label(detail.work_list.archived_at.is_some())
            );
            println!("Members:     {}", detail.members.len());
        }
    }
    Ok(())
}

fn lifecycle_label(is_archived: bool) -> &'static str {
    if is_archived { "Archived" } else { "Active" }
}

pub(crate) fn print_raw_tasks(tasks: &[TaskResponse], format: OutputFormat) -> CliResult<()> {
    match format {
        OutputFormat::Json => {
            print_pretty_json(tasks, "serializing tasks should succeed")?;
        }
        OutputFormat::Table => {
            println!(
                "{:<36}  {:<3}  {:<10}  {:<10}  Comments",
                "ID", "Pri", "Due", "Status"
            );
            println!("{}", "-".repeat(80));
            for task in tasks {
                let priority = task
                    .priority
                    .map(|value| value.to_string())
                    .unwrap_or_default();
                let due = task
                    .due_at
                    .map(|value| value.format("%Y-%m-%d").to_string())
                    .unwrap_or_else(|| "-".to_string());
                let status = if task.is_completed {
                    "Done"
                } else if task.archived_at.is_some() {
                    "Archived"
                } else {
                    "Active"
                };
                println!(
                    "{:<36}  {:<3}  {:<10}  {:<10}  {}",
                    task.id, priority, due, status, task.comment_count
                );
            }
            println!("\nTotal: {} task(s)", tasks.len());
        }
    }
    Ok(())
}

pub(crate) fn print_raw_my_tasks(
    tasks: &[sealtask_client_api::MyTaskResponse],
    format: OutputFormat,
) -> CliResult<()> {
    match format {
        OutputFormat::Json => {
            print_pretty_json(tasks, "serializing my tasks should succeed")?;
        }
        OutputFormat::Table => {
            println!(
                "{:<36}  {:<36}  {:<3}  {:<10}  Status",
                "Task ID", "Work List ID", "Pri", "Due"
            );
            println!("{}", "-".repeat(100));
            for task in tasks {
                let priority = task
                    .priority
                    .map(|value| value.to_string())
                    .unwrap_or_default();
                let due = task
                    .due_at
                    .map(|value| value.format("%Y-%m-%d").to_string())
                    .unwrap_or_else(|| "-".to_string());
                let status = if task.is_completed { "Done" } else { "Active" };
                println!(
                    "{:<36}  {:<36}  {:<3}  {:<10}  {}",
                    task.id, task.work_list_id, priority, due, status
                );
            }
            println!("\nTotal: {} task(s)", tasks.len());
        }
    }
    Ok(())
}

pub(crate) fn print_raw_task_detail(
    detail: &TaskDetailResponse,
    format: OutputFormat,
) -> CliResult<()> {
    match format {
        OutputFormat::Json => {
            print_pretty_json(detail, "serializing raw task detail should succeed")?;
        }
        OutputFormat::Table => {
            println!("Raw Task");
            println!("{}", "=".repeat(60));
            println!("ID:          {}", detail.task.id);
            println!("Work List:   {}", detail.task.work_list_id);
            println!("Comments:    {}", detail.comments.len());
        }
    }
    Ok(())
}

fn truncate(value: &str, width: usize) -> String {
    let sanitized = terminal_line(value);
    let mut chars = sanitized.chars();
    let truncated: String = chars.by_ref().take(width).collect();
    if chars.next().is_some() {
        truncated
    } else {
        sanitized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_prevent_terminal_lines_from_injecting_controls_or_extra_rows() {
        assert_eq!(truncate("a\nb\u{1b}[2J", 4), "a b[");
    }

    #[test]
    fn test_should_make_attachment_text_terminal_safe_without_flattening_readable_content() {
        let input = concat!(
            "plain \u{1b}[31mred\u{1b}[0m\n",
            "osc-bel \u{1b}]52;c;Y2xpcGJvYXJk\u{7} after\n",
            "osc-st \u{1b}]0;forged title\u{1b}\\ after\n",
            "c0 \u{0}\u{8}\t c1 \u{85}\u{9b}31m\n",
            "Unicode: Příliš žluťoučký 🦭"
        );

        let rendered = readable_attachment_terminal_text(input);

        assert!(
            rendered.chars().all(|ch| ch == '\n' || !ch.is_control()),
            "human output must contain no active terminal control characters"
        );
        assert_eq!(
            rendered,
            concat!(
                "plain [31mred[0m\n",
                "osc-bel ]52;c;Y2xpcGJvYXJk after\n",
                "osc-st ]0;forged title\\ after\n",
                "c0   c1  31m\n",
                "Unicode: Příliš žluťoučký 🦭"
            )
        );
        let attachment: ReadableAttachment = serde_json::from_value(json!({
            "attachment": {
                "id": uuid::Uuid::nil(),
                "fileName": "control.txt",
                "contentType": "text/plain",
                "sizeBytes": input.len(),
            },
            "text": input,
            "contentFormat": "text",
            "sourceKind": "plain_text",
        }))
        .expect("readable attachment");
        assert_eq!(
            serde_json::to_value(attachment).expect("readable attachment JSON")["text"],
            input,
            "JSON output must preserve the decrypted text exactly"
        );
    }
}
