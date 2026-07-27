use crate::args::BrowseArgs;
use crate::output::CliResult;
use crate::picker::{PickerCandidate, pick_candidate, show_private_document};
use crate::render::{task_reference_label, task_reference_title_label};
use crate::terminal::with_progress;
use sealtask_client_core::PublicError;
use sealtask_client_runtime::{AgentTaskDetail, RuntimeClient};

pub(crate) async fn run_browse(runtime: &RuntimeClient, args: BrowseArgs) -> CliResult<()> {
    let projects = with_progress(
        "Loading private project index…",
        runtime.list_work_lists_with_archived(args.password_stdin, args.include_archived),
    )
    .await?;
    let project_candidates = projects
        .iter()
        .filter(|project| args.include_archived || project.archived_at.is_none())
        .map(|project| PickerCandidate::new(project.id, project.title.clone()))
        .collect::<Vec<_>>();
    if project_candidates.is_empty() {
        return Err(PublicError::validation(
            "no projects are available to browse; broaden the archived scope or refresh the online cache",
        )
        .into());
    }
    let project_id = pick_candidate("project to browse", project_candidates)?;

    let tasks = with_progress(
        "Loading private task index…",
        runtime.list_project_tasks(
            project_id,
            args.include_completed,
            args.include_archived,
            args.password_stdin,
        ),
    )
    .await?;
    let task_candidates = tasks
        .iter()
        .map(|task| PickerCandidate::new(task.id, Some(task_reference_title_label(task))))
        .collect::<Vec<_>>();
    if task_candidates.is_empty() {
        return Err(PublicError::validation(
            "no tasks are available to browse in the selected project; broaden the completed or archived scope",
        )
        .into());
    }
    let task_id = pick_candidate("task to open", task_candidates)?;
    let detail = with_progress(
        "Loading private task detail…",
        runtime.get_task(project_id, task_id, args.password_stdin),
    )
    .await?;
    let (title, lines) = private_task_document(&detail);
    show_private_document(title, lines)
}

fn private_task_document(detail: &AgentTaskDetail) -> (String, Vec<String>) {
    let task = &detail.task;
    let title = format!("Task · {}", task_reference_title_label(task));
    let mut lines = vec![
        format!(
            "Title: {}",
            task.title.as_deref().unwrap_or("<unreadable title>")
        ),
        format!("Reference: {}", task_reference_label(task)),
        format!("ID: {}", task.id),
        format!(
            "Project: {}",
            task.work_list_title
                .as_deref()
                .unwrap_or("<unreadable project>")
        ),
        format!(
            "Status: {}{}",
            if task.is_completed {
                "completed"
            } else {
                "open"
            },
            if task.archived_at.is_some() {
                " · archived"
            } else {
                ""
            }
        ),
        format!(
            "Priority: {}",
            task.priority
                .map_or_else(|| "none".to_string(), |priority| priority.to_string())
        ),
        format!(
            "Due: {}",
            task.due_at
                .map_or_else(|| "none".to_string(), |due_at| due_at.to_rfc3339())
        ),
        String::new(),
        "Description".to_string(),
    ];
    match task.body_markdown.as_deref() {
        Some(body) if !body.is_empty() => {
            lines.extend(body.lines().map(ToOwned::to_owned));
        }
        _ => lines.push("<none>".to_string()),
    }

    if let Some(checklist) = task.checklist.as_ref()
        && !checklist.is_empty()
    {
        lines.push(String::new());
        lines.push("Checklist".to_string());
        lines.extend(checklist.iter().map(|item| {
            format!(
                "{} {}",
                if item.is_done { "[x]" } else { "[ ]" },
                item.title
            )
        }));
    }

    if !detail.comments.is_empty() {
        lines.push(String::new());
        lines.push(format!("Comments ({})", detail.comments.len()));
        for comment in &detail.comments {
            lines.push(format!("— {}", comment.created_at.to_rfc3339()));
            match comment.body_markdown.as_deref() {
                Some(body) if !body.is_empty() => {
                    lines.extend(body.lines().map(ToOwned::to_owned));
                }
                _ => lines.push("<unreadable comment>".to_string()),
            }
        }
    }

    if let Some(attachments) = task.attachments.as_ref()
        && !attachments.is_empty()
    {
        lines.push(String::new());
        lines.push(format!("Attachments ({})", attachments.len()));
        lines.extend(
            attachments
                .iter()
                .map(|attachment| format!("• {}", attachment.file_name)),
        );
    }
    (title, lines)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sealtask_client_runtime::AgentTaskSummary;
    use uuid::Uuid;

    #[test]
    fn private_document_keeps_decrypted_fields_out_of_debug_contracts() {
        let title_canary = "private-title-canary";
        let body_canary = "private-body-canary";
        let detail = AgentTaskDetail {
            task: AgentTaskSummary {
                id: Uuid::now_v7(),
                work_list_id: Uuid::now_v7(),
                work_list_title: None,
                work_list_timezone: None,
                created_by_membership_id: Uuid::now_v7(),
                section_id: None,
                priority: None,
                position: None,
                due_at: None,
                start_at: None,
                completed_at: None,
                archived_at: None,
                is_completed: false,
                recurrence_id: None,
                recurrence_schedule: None,
                recurrence_iteration: None,
                materialized_at: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                comment_count: 0,
                reference_number: Some(31),
                reference: Some("OPS-0031".to_string()),
                title: Some(title_canary.to_string()),
                body_markdown: Some(body_canary.to_string()),
                body_rich_text: None,
                checklist: None,
                attachments: None,
                references: None,
                mentions: None,
                client_meta: None,
                recurrence_state: None,
                delegations: Vec::new(),
                read_error: None,
            },
            comments: Vec::new(),
        };

        let (title, lines) = private_task_document(&detail);
        assert!(title.contains(title_canary));
        assert!(title.contains("OPS-0031"));
        assert!(lines.iter().any(|line| line.contains(body_canary)));
        assert!(lines.iter().any(|line| line == "Reference: OPS-0031"));
        assert!(
            !format!("{:?}", PickerCandidate::new(detail.task.id, None)).contains(title_canary)
        );
    }
}
