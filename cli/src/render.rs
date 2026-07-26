use crate::output::{
    CliResult, OutputFormat, print_json, print_simple_result, terminal_block, terminal_line,
};
use crate::output_models::{
    AttachmentV1, CommentV1, CurrentUserV1, DashboardStatsV1, NoteV1, ReadableAttachmentV1,
    TaskDetailV1, TaskSummaryV1, WorkListDetailV1, comments_v1, notes_v1, task_summaries_v1,
    work_list_summaries_v1,
};
use crate::selectors::ProjectSection;
use crate::table::{Alignment, Column, Table, short_unique_ids};
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use sealtask_client_api::{
    CurrentUserResponse, DashboardStatsResponse, TaskDetailResponse, TaskReferenceSchemeResponse,
    TaskResponse, WorkListDetailResponse, WorkListResponse,
};
use sealtask_client_runtime::{
    AgentComment, AgentNote, AgentTaskDetail, AgentTaskReferenceHistoryStatus, AgentTaskSummary,
    AgentWorkListDetail, AgentWorkListSummary, ReadableAttachment,
};
use serde_json::json;
use std::collections::HashMap;
use std::path::Path;

pub(crate) fn print_project_sections(
    sections: &[ProjectSection],
    format: OutputFormat,
) -> CliResult<()> {
    match format {
        OutputFormat::Json | OutputFormat::JsonPretty => print_json(
            sections,
            format,
            "serializing project sections should succeed",
        ),
        OutputFormat::Table => {
            let ids = selectable_short_ids(
                &sections
                    .iter()
                    .map(|section| section.id)
                    .collect::<Vec<_>>(),
            );
            let mut table = Table::new([
                Column::required("ID", 11, 39).preserve(),
                Column::required("Name", 12, 48).flex(4),
                Column::optional("Position", 8, 8, 30).align(Alignment::Right),
                Column::optional("WIP", 3, 8, 20).align(Alignment::Right),
                Column::optional("Auto archive", 12, 18, 10),
            ]);
            for (section, id) in sections.iter().zip(ids) {
                table.push_row([
                    id,
                    section.name.as_deref().unwrap_or("<unnamed>").to_string(),
                    section.position.to_string(),
                    section
                        .wip_limit
                        .map(|limit| limit.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                    if section.auto_archive_enabled {
                        section
                            .auto_archive_after_days
                            .map(|days| format!("{days} days"))
                            .unwrap_or_else(|| "enabled".to_string())
                    } else {
                        "off".to_string()
                    },
                ]);
            }
            print!("{}", table.render());
            println!("Total: {} section(s)", sections.len());
            Ok(())
        }
    }
}

pub(crate) fn print_download_result(
    format: OutputFormat,
    file_name: &str,
    output_path: &Path,
) -> CliResult<()> {
    match format {
        OutputFormat::Json | OutputFormat::JsonPretty => print_json(
            &json!({
                "fileName": file_name,
                "outputPath": output_path.display().to_string(),
            }),
            format,
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
        OutputFormat::Json | OutputFormat::JsonPretty => {
            print_json(
                &ReadableAttachmentV1::from(attachment),
                format,
                "serializing readable attachment should succeed",
            )?;
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

pub(crate) fn print_attachment(
    attachment: &sealtask_client_runtime::AgentAttachment,
    format: OutputFormat,
) -> CliResult<()> {
    match format {
        OutputFormat::Json | OutputFormat::JsonPretty => print_json(
            &AttachmentV1::from(attachment),
            format,
            "serializing attachment should succeed",
        ),
        OutputFormat::Table => {
            println!(
                "Uploaded attachment {} ({}, {} B).",
                attachment.id,
                terminal_line(&attachment.file_name),
                attachment.size_bytes
            );
            Ok(())
        }
    }
}

pub(crate) fn print_comment(comment: &AgentComment, format: OutputFormat) -> CliResult<()> {
    match format {
        OutputFormat::Json | OutputFormat::JsonPretty => print_json(
            &CommentV1::from(comment),
            format,
            "serializing comment should succeed",
        ),
        OutputFormat::Table => {
            println!("Comment {}", comment.id);
            println!(
                "{}",
                terminal_block(
                    comment
                        .body_markdown
                        .as_deref()
                        .unwrap_or("<unreadable comment>")
                )
            );
            Ok(())
        }
    }
}

pub(crate) fn print_comments(comments: &[AgentComment], format: OutputFormat) -> CliResult<()> {
    match format {
        OutputFormat::Json | OutputFormat::JsonPretty => {
            print_json(
                &comments_v1(comments),
                format,
                "serializing comments should succeed",
            )?;
        }
        OutputFormat::Table => {
            let mut table = Table::new([
                Column::required("ID", 36, 36).preserve(),
                Column::optional("Updated", 16, 16, 20),
                Column::optional("Comment", 12, 64, 40).flex(4),
            ]);
            for comment in comments {
                table.push_row([
                    comment.id.to_string(),
                    comment.updated_at.format("%Y-%m-%d %H:%M").to_string(),
                    comment
                        .body_markdown
                        .as_deref()
                        .unwrap_or("<unreadable comment>")
                        .to_string(),
                ]);
            }
            print!("{}", table.render());
            println!("\nTotal: {} comment(s)", comments.len());
        }
    }
    Ok(())
}

pub(crate) fn print_notes(notes: &[AgentNote], format: OutputFormat) -> CliResult<()> {
    match format {
        OutputFormat::Json | OutputFormat::JsonPretty => {
            print_json(&notes_v1(notes), format, "serializing notes should succeed")?
        }
        OutputFormat::Table => {
            let ids = selectable_short_ids(&notes.iter().map(|note| note.id).collect::<Vec<_>>());
            let mut table = Table::new([
                Column::required("ID", 11, 39).preserve(),
                Column::optional("Privacy", 7, 7, 30),
                Column::required("Title", 12, 56).flex(4),
                Column::optional("Updated", 16, 16, 20),
            ]);
            for (note, id) in notes.iter().zip(ids) {
                table.push_row([
                    id,
                    if note.is_private {
                        "Private".to_string()
                    } else {
                        "Shared".to_string()
                    },
                    note.title
                        .as_deref()
                        .unwrap_or("<unreadable note>")
                        .to_string(),
                    note.updated_at.format("%Y-%m-%d %H:%M").to_string(),
                ]);
            }
            print!("{}", table.render());
            println!("\nTotal: {} note(s)", notes.len());
        }
    }
    Ok(())
}

pub(crate) fn print_note(note: &AgentNote, format: OutputFormat) -> CliResult<()> {
    match format {
        OutputFormat::Json | OutputFormat::JsonPretty => print_json(
            &NoteV1::from(note),
            format,
            "serializing note should succeed",
        )?,
        OutputFormat::Table => {
            println!("Note");
            println!("{}", "=".repeat(60));
            println!("ID:        {}", note.id);
            println!("Project:   {}", note.work_list_id);
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

pub(crate) fn print_task_reference_scheme_result(
    scheme: &TaskReferenceSchemeResponse,
    format: OutputFormat,
    table_message: &str,
) -> CliResult<()> {
    print_simple_result(
        format,
        &json!({
            "workListId": scheme.work_list_id,
            "schemeRevisionId": scheme.scheme_revision_id,
            "revision": scheme.revision,
            "isRepair": scheme.is_repair,
            "createdAt": scheme.created_at,
            "retiredAt": scheme.retired_at,
            "quarantinedAt": scheme.quarantined_at,
            "quarantinedByMembershipId": scheme.quarantined_by_membership_id,
        }),
        "serializing task-reference scheme result should succeed",
        table_message,
    )
}

pub(crate) fn print_task_reference_history_status(
    status: &AgentTaskReferenceHistoryStatus,
    format: OutputFormat,
) -> CliResult<()> {
    match format {
        OutputFormat::Json | OutputFormat::JsonPretty => print_json(
            status,
            format,
            "serializing task-reference history status should succeed",
        ),
        OutputFormat::Table => {
            println!(
                "Task-reference history for {}: {:?}",
                status.work_list_id, status.availability
            );
            if status.schemes.is_empty() {
                println!("No verified scheme rows are available.");
                return Ok(());
            }
            println!(
                "{:<8}  {:<36}  {:<8}  State",
                "Revision", "Scheme ID", "Lane"
            );
            println!("{}", "-".repeat(78));
            for scheme in &status.schemes {
                println!(
                    "{:<8}  {:<36}  {:<8}  {}",
                    scheme.revision,
                    scheme.scheme_revision_id,
                    if scheme.is_repair {
                        "repair"
                    } else {
                        "ordinary"
                    },
                    scheme.state
                );
            }
            Ok(())
        }
    }
}

pub(crate) fn print_empty_collection(format: OutputFormat, table_message: &str) -> CliResult<()> {
    match format {
        OutputFormat::Json | OutputFormat::JsonPretty => print_json(
            &Vec::<serde_json::Value>::new(),
            format,
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
        OutputFormat::Json | OutputFormat::JsonPretty => print_json(
            &CurrentUserV1::from(user),
            format,
            "serializing user should succeed",
        )?,
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
        OutputFormat::Json | OutputFormat::JsonPretty => {
            print_json(
                &work_list_summaries_v1(lists),
                format,
                "serializing projects should succeed",
            )?;
        }
        OutputFormat::Table => {
            if verbose {
                for (index, list) in lists.iter().enumerate() {
                    if index > 0 {
                        println!();
                    }
                    println!("Project: {}", list.id);
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
                println!("\nTotal: {} project(s)", lists.len());
            } else {
                let ids =
                    selectable_short_ids(&lists.iter().map(|list| list.id).collect::<Vec<_>>());
                let mut table = Table::new([
                    Column::required("ID", 11, 39).preserve(),
                    Column::required("Title", 12, 56).flex(4),
                    Column::optional("Role", 6, 12, 30),
                    Column::required("Lifecycle", 9, 9),
                    Column::optional("Updated", 16, 16, 20),
                ]);
                for (list, id) in lists.iter().zip(ids) {
                    table.push_row([
                        id,
                        list.title.as_deref().unwrap_or("-").to_string(),
                        list.membership.role.clone(),
                        lifecycle_label(list.archived_at.is_some()).to_string(),
                        list.updated_at.format("%Y-%m-%d %H:%M").to_string(),
                    ]);
                }
                print!("{}", table.render());
                println!("\nTotal: {} project(s)", lists.len());
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
        OutputFormat::Json | OutputFormat::JsonPretty => {
            print_json(
                &WorkListDetailV1::from(detail),
                format,
                "serializing project detail should succeed",
            )?;
        }
        OutputFormat::Table => {
            println!("Project");
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

pub(crate) fn print_task(task: &AgentTaskSummary, format: OutputFormat) -> CliResult<()> {
    match format {
        OutputFormat::Json | OutputFormat::JsonPretty => print_json(
            &TaskSummaryV1::from(task),
            format,
            "serializing task should succeed",
        ),
        OutputFormat::Table => {
            println!(
                "Task {}: {}",
                task.id,
                terminal_line(task.title.as_deref().unwrap_or("<unreadable task>"))
            );
            if let Some(reference) = task.reference.as_deref() {
                println!("Ref:    {}", terminal_line(reference));
            } else if task.reference_number.is_some() {
                println!("Ref:    <reference unavailable>");
            }
            println!(
                "Status: {}",
                if task.is_completed {
                    "Done"
                } else if task.archived_at.is_some() {
                    "Archived"
                } else {
                    "Active"
                }
            );
            if let Some(due) = task_due_detail(task) {
                println!("Due:    {due}");
            }
            Ok(())
        }
    }
}

pub(crate) fn print_tasks(tasks: &[AgentTaskSummary], format: OutputFormat) -> CliResult<()> {
    match format {
        OutputFormat::Json | OutputFormat::JsonPretty => {
            print_json(
                &task_summaries_v1(tasks),
                format,
                "serializing tasks should succeed",
            )?;
        }
        OutputFormat::Table => {
            let ids = selectable_short_ids(&tasks.iter().map(|task| task.id).collect::<Vec<_>>());
            let mut table = Table::new([
                Column::required("Reference", 11, 32).preserve(),
                Column::optional("ID", 11, 39, 35).preserve(),
                Column::required("Title", 12, 60).flex(4),
                Column::optional("Pri", 3, 3, 40).align(Alignment::Right),
                Column::optional("Due", 10, 10, 30),
                Column::required("Status", 6, 10),
            ]);
            for (task, id) in tasks.iter().zip(ids) {
                let priority = priority_label(task.priority);
                let due = task_due_date(task);
                let status = if task.is_completed {
                    "Done"
                } else if task.archived_at.is_some() {
                    "Archived"
                } else {
                    "Active"
                };
                table.push_row([
                    task.reference
                        .as_deref()
                        .unwrap_or_else(|| {
                            if task.reference_number.is_some() {
                                "<reference unavailable>"
                            } else {
                                "-"
                            }
                        })
                        .to_string(),
                    id,
                    task.title.as_deref().unwrap_or("-").to_string(),
                    priority,
                    due,
                    status.to_string(),
                ]);
            }
            print!("{}", table.render());
            println!("\nTotal: {} task(s)", tasks.len());
        }
    }
    Ok(())
}

pub(crate) fn print_task_detail(detail: &AgentTaskDetail, format: OutputFormat) -> CliResult<()> {
    match format {
        OutputFormat::Json | OutputFormat::JsonPretty => {
            print_json(
                &TaskDetailV1::from(detail),
                format,
                "serializing task detail should succeed",
            )?;
        }
        OutputFormat::Table => {
            let task = &detail.task;
            println!("Task");
            println!("{}", "=".repeat(60));
            println!("ID:          {}", task.id);
            if let Some(reference) = task.reference.as_deref() {
                println!("Reference:   {}", terminal_line(reference));
            } else if task.reference_number.is_some() {
                println!("Reference:   <reference unavailable>");
            }
            println!(
                "Title:       {}",
                terminal_line(task.title.as_deref().unwrap_or("-"))
            );
            println!("Project:     {}", task.work_list_id);
            if let Some(work_list_title) = task.work_list_title.as_deref() {
                println!("Project Title: {}", terminal_line(work_list_title));
            }
            println!(
                "Status:      {}",
                if task.is_completed { "Done" } else { "Active" }
            );
            if let Some(due) = task_due_detail(task) {
                println!("Due:         {due}");
            }
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
                let mut table = Table::new([
                    Column::required("ID", 36, 36).preserve(),
                    Column::optional("File", 12, 56, 40).flex(4),
                    Column::optional("Type", 12, 32, 30).flex(2),
                    Column::optional("Size", 6, 12, 20).align(Alignment::Right),
                ]);
                for attachment in attachments {
                    table.push_row([
                        attachment.id.to_string(),
                        attachment.file_name.clone(),
                        attachment.content_type.clone(),
                        format!("{} B", attachment.size_bytes),
                    ]);
                }
                print!("{}", table.render());
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
        OutputFormat::Json | OutputFormat::JsonPretty => {
            print_json(
                &DashboardStatsV1::from(stats),
                format,
                "serializing stats should succeed",
            )?;
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
        OutputFormat::Json | OutputFormat::JsonPretty => {
            print_json(lists, format, "serializing projects should succeed")?;
        }
        OutputFormat::Table => {
            if verbose {
                for (index, list) in lists.iter().enumerate() {
                    if index > 0 {
                        println!();
                    }
                    println!("Project: {}", list.id);
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
                println!("\nTotal: {} project(s)", lists.len());
            } else {
                let ids =
                    selectable_short_ids(&lists.iter().map(|list| list.id).collect::<Vec<_>>());
                let mut table = Table::new([
                    Column::required("ID", 11, 39).preserve(),
                    Column::optional("Role", 6, 12, 30),
                    Column::optional("Sections", 8, 8, 20).align(Alignment::Right),
                    Column::required("Lifecycle", 9, 9),
                    Column::optional("Updated", 16, 16, 10),
                ]);
                for (list, id) in lists.iter().zip(ids) {
                    table.push_row([
                        id,
                        list.membership.role.clone(),
                        list.section_snapshots.len().to_string(),
                        lifecycle_label(list.archived_at.is_some()).to_string(),
                        list.updated_at.format("%Y-%m-%d %H:%M").to_string(),
                    ]);
                }
                print!("{}", table.render());
                println!("\nTotal: {} project(s)", lists.len());
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
        OutputFormat::Json | OutputFormat::JsonPretty => {
            print_json(
                detail,
                format,
                "serializing raw project detail should succeed",
            )?;
        }
        OutputFormat::Table => {
            println!("Raw Project");
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

fn priority_label(priority: Option<i8>) -> String {
    match priority {
        Some(8) => "P1".to_string(),
        Some(5) => "P2".to_string(),
        Some(3) => "P3".to_string(),
        Some(1) => "P4".to_string(),
        Some(value) => value.to_string(),
        None => String::new(),
    }
}

fn task_due_date(task: &AgentTaskSummary) -> String {
    format_due_date(task.due_at, task.work_list_timezone.as_deref())
}

fn task_due_detail(task: &AgentTaskSummary) -> Option<String> {
    let due_at = task.due_at?;
    match task
        .work_list_timezone
        .as_deref()
        .and_then(|value| value.parse::<Tz>().ok())
    {
        Some(timezone) => Some(format!(
            "{} {}",
            due_at.with_timezone(&timezone).format("%Y-%m-%d %H:%M"),
            timezone
        )),
        None => Some(due_at.format("%Y-%m-%d %H:%M UTC").to_string()),
    }
}

fn format_due_date(due_at: Option<DateTime<Utc>>, timezone: Option<&str>) -> String {
    let Some(due_at) = due_at else {
        return "-".to_string();
    };
    match timezone.and_then(|value| value.parse::<Tz>().ok()) {
        Some(timezone) => due_at
            .with_timezone(&timezone)
            .format("%Y-%m-%d")
            .to_string(),
        None => due_at.format("%Y-%m-%d UTC").to_string(),
    }
}

fn format_utc_due_date(due_at: Option<DateTime<Utc>>) -> String {
    due_at
        .map(|value| value.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn selectable_short_ids(ids: &[uuid::Uuid]) -> Vec<String> {
    short_unique_ids(ids)
        .into_iter()
        .map(|id| format!("id:{id}"))
        .collect()
}

pub(crate) fn print_raw_tasks(tasks: &[TaskResponse], format: OutputFormat) -> CliResult<()> {
    match format {
        OutputFormat::Json | OutputFormat::JsonPretty => {
            print_json(tasks, format, "serializing tasks should succeed")?;
        }
        OutputFormat::Table => {
            let ids = selectable_short_ids(&tasks.iter().map(|task| task.id).collect::<Vec<_>>());
            let mut table = Table::new([
                Column::required("ID", 11, 39).preserve(),
                Column::optional("Pri", 3, 3, 40).align(Alignment::Right),
                Column::optional("Due (UTC)", 10, 10, 30),
                Column::required("Status", 6, 10),
                Column::optional("Comments", 8, 8, 20).align(Alignment::Right),
            ]);
            for (task, id) in tasks.iter().zip(ids) {
                let priority = priority_label(task.priority);
                let due = format_utc_due_date(task.due_at);
                let status = if task.is_completed {
                    "Done"
                } else if task.archived_at.is_some() {
                    "Archived"
                } else {
                    "Active"
                };
                table.push_row([
                    id,
                    priority,
                    due,
                    status.to_string(),
                    task.comment_count.to_string(),
                ]);
            }
            print!("{}", table.render());
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
        OutputFormat::Json | OutputFormat::JsonPretty => {
            print_json(tasks, format, "serializing my tasks should succeed")?;
        }
        OutputFormat::Table => {
            let task_ids =
                selectable_short_ids(&tasks.iter().map(|task| task.id).collect::<Vec<_>>());
            let mut distinct_project_ids = tasks
                .iter()
                .map(|task| task.work_list_id)
                .collect::<Vec<_>>();
            distinct_project_ids.sort_unstable();
            distinct_project_ids.dedup();
            let project_ids = distinct_project_ids
                .iter()
                .copied()
                .zip(selectable_short_ids(&distinct_project_ids))
                .collect::<HashMap<_, _>>();
            let mut table = Table::new([
                Column::required("Task ID", 11, 39).preserve(),
                Column::required("Project ID", 11, 39).preserve(),
                Column::optional("Pri", 3, 3, 40).align(Alignment::Right),
                Column::optional("Due (UTC)", 10, 10, 30),
                Column::required("Status", 6, 10),
            ]);
            for (task, task_id) in tasks.iter().zip(task_ids) {
                let project_id = project_ids
                    .get(&task.work_list_id)
                    .cloned()
                    .unwrap_or_else(|| task.work_list_id.to_string());
                let priority = priority_label(task.priority);
                let due = format_utc_due_date(task.due_at);
                let status = if task.is_completed { "Done" } else { "Active" };
                table.push_row([task_id, project_id, priority, due, status.to_string()]);
            }
            print!("{}", table.render());
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
        OutputFormat::Json | OutputFormat::JsonPretty => {
            print_json(detail, format, "serializing raw task detail should succeed")?;
        }
        OutputFormat::Table => {
            println!("Raw Task");
            println!("{}", "=".repeat(60));
            println!("ID:          {}", detail.task.id);
            println!("Project:     {}", detail.task.work_list_id);
            if let Some(due_at) = detail.task.due_at {
                println!("Due (UTC):   {}", due_at.format("%Y-%m-%d %H:%M UTC"));
            }
            println!("Comments:    {}", detail.comments.len());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_priority_labels_match_accepted_p_aliases() {
        assert_eq!(priority_label(Some(8)), "P1");
        assert_eq!(priority_label(Some(5)), "P2");
        assert_eq!(priority_label(Some(3)), "P3");
        assert_eq!(priority_label(Some(1)), "P4");
        assert_eq!(priority_label(Some(7)), "7");
        assert_eq!(priority_label(None), "");
    }

    #[test]
    fn due_dates_use_the_project_timezone_and_label_utc_fallbacks() {
        let due_at = DateTime::parse_from_rfc3339("2026-07-25T22:00:00Z")
            .expect("RFC 3339")
            .with_timezone(&Utc);

        assert_eq!(
            format_due_date(Some(due_at), Some("Europe/Prague")),
            "2026-07-26"
        );
        assert_eq!(
            format_due_date(Some(due_at), Some("not-a-timezone")),
            "2026-07-25 UTC"
        );
        assert_eq!(format_due_date(None, Some("Europe/Prague")), "-");
    }

    #[test]
    fn displayed_short_ids_force_id_selector_semantics() {
        let id = uuid::Uuid::parse_str("01900000-0000-7000-8000-000000000001").expect("UUID");
        assert_eq!(selectable_short_ids(&[id]), ["id:01900000"]);
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
        let legacy_json =
            serde_json::to_value(&attachment).expect("legacy readable attachment JSON");
        let v1_json = serde_json::to_value(ReadableAttachmentV1::from(&attachment))
            .expect("v1 readable attachment JSON");
        assert_eq!(
            v1_json, legacy_json,
            "explicit v1 DTO must preserve the published field contract"
        );
        assert_eq!(
            legacy_json["text"], input,
            "JSON output must preserve the decrypted text exactly"
        );
    }
}
