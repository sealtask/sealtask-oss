use crate::args::{TaskListColumnArg, TaskListFieldArg, TaskListSortArg};
use crate::output::{CliResult, OutputFormat, terminal_line, write_stdout_line};
use crate::render::{print_tasks, task_due_date};
use crate::resolver::ResolvedProject;
use crate::table::{Alignment, Column, ColumnStyle, Table, short_unique_ids};
use crate::terminal;
use reqwest::Url;
use sealtask_client_core::PublicError;
use sealtask_client_runtime::AgentTaskSummary;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use unicode_normalization::UnicodeNormalization;
use uuid::Uuid;

const MAXIMUM_WEB_ORIGIN_BYTES: usize = 2_048;

#[derive(Clone, Debug)]
pub(crate) enum TaskListScope {
    Selected(ResolvedProject),
    Current(ResolvedProject),
    AcrossProjects,
}

#[derive(Clone, Debug)]
pub(crate) struct TaskListOptions<'a> {
    pub(crate) columns: &'a [TaskListColumnArg],
    pub(crate) sort: Option<TaskListSortArg>,
    pub(crate) field: Option<TaskListFieldArg>,
    pub(crate) web_url: Option<&'a str>,
    pub(crate) api_url: &'a str,
    pub(crate) include_completed: bool,
    pub(crate) include_archived: bool,
    pub(crate) scope: TaskListScope,
}

pub(crate) fn is_raw_field_output(command: &crate::args::TasksCommand) -> bool {
    matches!(
        command,
        crate::args::TasksCommand::List { field: Some(_), .. }
    )
}

pub(crate) fn validate_output_mode(
    format: OutputFormat,
    columns: &[TaskListColumnArg],
    field: Option<TaskListFieldArg>,
    web_url_explicit: bool,
) -> CliResult<()> {
    if format.is_json() && !columns.is_empty() {
        return Err(PublicError::validation(
            "--columns controls human table output and cannot be combined with --json or any JSON --format value",
        )
        .into());
    }
    if format.is_json() && field.is_some() {
        return Err(PublicError::validation(
            "--field emits raw newline-delimited values and cannot be combined with --json or any JSON --format value",
        )
        .into());
    }
    if web_url_explicit && field != Some(TaskListFieldArg::Url) {
        return Err(PublicError::validation("--web-url is only used with '--field url'").into());
    }
    let mut seen = HashSet::with_capacity(columns.len());
    if let Some(duplicate) = columns.iter().copied().find(|column| !seen.insert(*column)) {
        return Err(PublicError::validation(format!(
            "duplicate task-list column '{}'; list each --columns value once",
            column_name(duplicate)
        ))
        .into());
    }
    Ok(())
}

pub(crate) fn print_task_list(
    mut tasks: Vec<AgentTaskSummary>,
    format: OutputFormat,
    options: TaskListOptions<'_>,
) -> CliResult<()> {
    validate_output_mode(format, options.columns, options.field, false)?;
    if let Some(sort) = options.sort {
        sort_tasks(&mut tasks, sort);
    }

    if let Some(field) = options.field {
        return print_fields(
            &tasks,
            field,
            options.web_url,
            options.api_url,
            options.scope.project_id(),
        );
    }

    if format.is_json() {
        return print_tasks(&tasks, format);
    }

    if terminal::stdout_is_terminal() {
        print_scope(&options.scope, &tasks)?;
    }
    if tasks.is_empty() {
        println!(
            "{}",
            empty_guidance(
                &options.scope,
                options.include_completed,
                options.include_archived
            )
        );
        return Ok(());
    }

    print_table(&tasks, options.columns, options.scope.is_cross_project())
}

impl TaskListScope {
    fn project_id(&self) -> Option<Uuid> {
        match self {
            Self::Selected(project) | Self::Current(project) => Some(project.id),
            Self::AcrossProjects => None,
        }
    }

    fn is_cross_project(&self) -> bool {
        matches!(self, Self::AcrossProjects)
    }
}

fn print_scope(scope: &TaskListScope, tasks: &[AgentTaskSummary]) -> CliResult<()> {
    match scope {
        TaskListScope::Selected(project) => {
            println!(
                "Selected project: {}",
                project_label(project, task_project_title(tasks, project.id))
            );
        }
        TaskListScope::Current(project) => {
            println!(
                "Current project: {}",
                project_label(project, task_project_title(tasks, project.id))
            );
        }
        TaskListScope::AcrossProjects => {
            println!("Scope: assigned tasks across all projects");
        }
    }
    println!();
    Ok(())
}

fn project_label(project: &ResolvedProject, task_title: Option<&str>) -> String {
    let selector = format!("id:{}", project.id.simple());
    project
        .title
        .as_deref()
        .or(task_title)
        .map_or(selector.clone(), |title| {
            format!("\"{}\" ({selector})", terminal_line(title))
        })
}

fn task_project_title(tasks: &[AgentTaskSummary], project_id: Uuid) -> Option<&str> {
    tasks
        .iter()
        .find(|task| task.work_list_id == project_id)
        .and_then(|task| task.work_list_title.as_deref())
}

fn empty_guidance(
    scope: &TaskListScope,
    include_completed: bool,
    include_archived: bool,
) -> &'static str {
    if scope.is_cross_project() {
        return if include_completed {
            "No assigned tasks found across projects.\nChoose a current project: sealtask pick project"
        } else {
            "No active assigned tasks found across projects.\nShow completed: sealtask tasks list --all --include-completed\nChoose a current project: sealtask pick project"
        };
    }

    match (include_completed, include_archived) {
        (false, false) => {
            "No active tasks in this project.\nNext: sealtask tasks create --title \"<TITLE>\"\nShow completed: sealtask tasks list --include-completed"
        }
        (true, false) => {
            "No unarchived tasks in this project.\nNext: sealtask tasks create --title \"<TITLE>\"\nShow archived: sealtask tasks list --include-completed --include-archived"
        }
        (false, true) => {
            "No incomplete tasks in this project.\nNext: sealtask tasks create --title \"<TITLE>\"\nShow every task: sealtask tasks list --include-completed --include-archived"
        }
        (true, true) => {
            "No tasks in this project.\nNext: sealtask tasks create --title \"<TITLE>\""
        }
    }
}

fn print_fields(
    tasks: &[AgentTaskSummary],
    field: TaskListFieldArg,
    web_url: Option<&str>,
    api_url: &str,
    scoped_project_id: Option<Uuid>,
) -> CliResult<()> {
    let lines = match field {
        TaskListFieldArg::Id => tasks
            .iter()
            .map(|task| format!("id:{}", task.id.simple()))
            .collect::<Vec<_>>(),
        TaskListFieldArg::Title => tasks
            .iter()
            .map(|task| {
                task.title
                    .as_deref()
                    .map(terminal_line)
                    .ok_or_else(|| {
                        PublicError::validation(format!(
                            "cannot emit --field title because task id:{} has no readable title; inspect its readError with JSON output",
                            task.id.simple()
                        ))
                    })
            })
            .collect::<Result<Vec<_>, _>>()?,
        TaskListFieldArg::Url => {
            let web_origin = resolve_web_origin(web_url, api_url)?;
            tasks
                .iter()
                .map(|task| {
                    let project_id = if task.work_list_id.is_nil() {
                        scoped_project_id.ok_or_else(|| {
                            PublicError::validation(format!(
                                "cannot build a URL for task id:{} because its project ID is unavailable",
                                task.id.simple()
                            ))
                        })?
                    } else {
                        task.work_list_id
                    };
                    Ok(task_url(&web_origin, project_id, task.id).to_string())
                })
                .collect::<Result<Vec<_>, PublicError>>()?
        }
    };

    for line in lines {
        write_stdout_line(format_args!("{line}"))?;
    }
    Ok(())
}

pub(crate) fn resolve_web_origin(
    override_url: Option<&str>,
    api_url: &str,
) -> Result<Url, PublicError> {
    let raw = override_url.unwrap_or(api_url);
    if raw.is_empty() || raw.len() > MAXIMUM_WEB_ORIGIN_BYTES {
        return Err(PublicError::validation(
            "web URL must be a non-empty HTTP(S) origin of at most 2048 bytes",
        ));
    }
    let mut url = Url::parse(raw)
        .map_err(|_| PublicError::validation("web URL must be a valid absolute HTTP(S) origin"))?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(PublicError::validation(
            "web URL must use HTTP or HTTPS and include a host",
        ));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(PublicError::validation(
            "web URL must not contain embedded credentials",
        ));
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(PublicError::validation(
            "web URL must not contain a query string or fragment",
        ));
    }
    if override_url.is_some() && !matches!(url.path(), "" | "/") {
        return Err(PublicError::validation(
            "web URL must be an origin without an application path",
        ));
    }
    url.set_path("/");
    Ok(url)
}

fn task_url(origin: &Url, project_id: Uuid, task_id: Uuid) -> Url {
    let mut url = origin.clone();
    url.set_path(&format!("/workspace/work-lists/{project_id}"));
    url.query_pairs_mut()
        .clear()
        .append_pair("task", &task_id.to_string());
    url
}

fn print_table(
    tasks: &[AgentTaskSummary],
    requested_columns: &[TaskListColumnArg],
    cross_project: bool,
) -> CliResult<()> {
    print!(
        "{}",
        render_task_table(tasks, requested_columns, cross_project)
    );
    Ok(())
}

pub(crate) fn render_default_project_task_table(tasks: &[AgentTaskSummary]) -> String {
    render_task_table(tasks, &[], false)
}

fn render_task_table(
    tasks: &[AgentTaskSummary],
    requested_columns: &[TaskListColumnArg],
    cross_project: bool,
) -> String {
    let columns = if requested_columns.is_empty() {
        default_columns(cross_project)
    } else {
        requested_columns
            .iter()
            .copied()
            .map(explicit_column)
            .collect()
    };
    let task_ids = selectable_ids(tasks.iter().map(|task| task.id).collect());
    let project_ids = selectable_project_ids(tasks);
    let mut table = Table::new(columns);
    let effective_columns = if requested_columns.is_empty() {
        default_column_values(cross_project)
    } else {
        requested_columns.to_vec()
    };
    for (task, task_id) in tasks.iter().zip(task_ids) {
        table.push_row(
            effective_columns
                .iter()
                .map(|column| column_value(task, task_id.as_str(), &project_ids, *column)),
        );
    }
    format!("{}\nTotal: {} task(s)\n", table.render(), tasks.len())
}

fn default_columns(cross_project: bool) -> Vec<Column> {
    let mut columns = vec![
        default_column(TaskListColumnArg::Id),
        default_column(TaskListColumnArg::Title),
    ];
    if cross_project {
        columns.push(default_column(TaskListColumnArg::Project));
    }
    columns.extend([
        default_column(TaskListColumnArg::Priority),
        default_column(TaskListColumnArg::Due),
        default_column(TaskListColumnArg::Status),
    ]);
    columns
}

fn default_column_values(cross_project: bool) -> Vec<TaskListColumnArg> {
    let mut columns = vec![TaskListColumnArg::Id, TaskListColumnArg::Title];
    if cross_project {
        columns.push(TaskListColumnArg::Project);
    }
    columns.extend([
        TaskListColumnArg::Priority,
        TaskListColumnArg::Due,
        TaskListColumnArg::Status,
    ]);
    columns
}

fn default_column(column: TaskListColumnArg) -> Column {
    match column {
        TaskListColumnArg::Id => Column::required("ID", 11, 39).preserve(),
        TaskListColumnArg::Title => Column::required("Title", 12, 60).flex(4),
        TaskListColumnArg::Project => Column::required("Project", 12, 40).flex(2),
        TaskListColumnArg::Priority => Column::optional("Pri", 3, 3, 40)
            .align(Alignment::Right)
            .semantic(ColumnStyle::Priority),
        TaskListColumnArg::Due => Column::optional("Due", 10, 10, 30),
        TaskListColumnArg::Status => {
            Column::required("Status", 6, 10).semantic(ColumnStyle::Status)
        }
        TaskListColumnArg::ProjectId
        | TaskListColumnArg::Comments
        | TaskListColumnArg::Created
        | TaskListColumnArg::Updated => explicit_column(column),
    }
}

fn explicit_column(column: TaskListColumnArg) -> Column {
    let column = match column {
        TaskListColumnArg::Id => Column::required("ID", 11, 39).preserve(),
        TaskListColumnArg::Title => Column::required("Title", 8, 60).flex(4),
        TaskListColumnArg::Project => Column::required("Project", 8, 40).flex(2),
        TaskListColumnArg::ProjectId => Column::required("Project ID", 11, 39).preserve(),
        TaskListColumnArg::Priority => Column::required("Pri", 3, 3)
            .align(Alignment::Right)
            .semantic(ColumnStyle::Priority),
        TaskListColumnArg::Due => Column::required("Due", 10, 10),
        TaskListColumnArg::Status => {
            Column::required("Status", 6, 10).semantic(ColumnStyle::Status)
        }
        TaskListColumnArg::Comments => Column::required("Comments", 8, 8).align(Alignment::Right),
        TaskListColumnArg::Created => Column::required("Created (UTC)", 16, 16),
        TaskListColumnArg::Updated => Column::required("Updated (UTC)", 16, 16),
    };
    column.retain()
}

fn column_name(column: TaskListColumnArg) -> &'static str {
    match column {
        TaskListColumnArg::Id => "id",
        TaskListColumnArg::Title => "title",
        TaskListColumnArg::Project => "project",
        TaskListColumnArg::ProjectId => "project-id",
        TaskListColumnArg::Priority => "priority",
        TaskListColumnArg::Due => "due",
        TaskListColumnArg::Status => "status",
        TaskListColumnArg::Comments => "comments",
        TaskListColumnArg::Created => "created",
        TaskListColumnArg::Updated => "updated",
    }
}

fn column_value(
    task: &AgentTaskSummary,
    task_id: &str,
    project_ids: &HashMap<Uuid, String>,
    column: TaskListColumnArg,
) -> String {
    match column {
        TaskListColumnArg::Id => task_id.to_string(),
        TaskListColumnArg::Title => task.title.as_deref().unwrap_or("-").to_string(),
        TaskListColumnArg::Project => task.work_list_title.clone().unwrap_or_else(|| {
            project_ids
                .get(&task.work_list_id)
                .cloned()
                .unwrap_or_else(|| format!("id:{}", task.work_list_id.simple()))
        }),
        TaskListColumnArg::ProjectId => project_ids
            .get(&task.work_list_id)
            .cloned()
            .unwrap_or_else(|| format!("id:{}", task.work_list_id.simple())),
        TaskListColumnArg::Priority => priority_label(task.priority),
        TaskListColumnArg::Due => task_due_date(task),
        TaskListColumnArg::Status => task_status(task).to_string(),
        TaskListColumnArg::Comments => task.comment_count.to_string(),
        TaskListColumnArg::Created => task.created_at.format("%Y-%m-%d %H:%M").to_string(),
        TaskListColumnArg::Updated => task.updated_at.format("%Y-%m-%d %H:%M").to_string(),
    }
}

fn selectable_ids(ids: Vec<Uuid>) -> Vec<String> {
    short_unique_ids(&ids)
        .into_iter()
        .map(|id| format!("id:{id}"))
        .collect()
}

fn selectable_project_ids(tasks: &[AgentTaskSummary]) -> HashMap<Uuid, String> {
    let mut ids = tasks
        .iter()
        .map(|task| task.work_list_id)
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids.dedup();
    ids.iter()
        .copied()
        .zip(selectable_ids(ids.clone()))
        .collect()
}

fn task_status(task: &AgentTaskSummary) -> &'static str {
    if task.is_completed {
        "Done"
    } else if task.archived_at.is_some() {
        "Archived"
    } else {
        "Active"
    }
}

fn status_rank(task: &AgentTaskSummary) -> u8 {
    match task_status(task) {
        "Active" => 0,
        "Done" => 1,
        "Archived" => 2,
        _ => unreachable!("task status has three fixed values"),
    }
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

fn sort_tasks(tasks: &mut [AgentTaskSummary], sort: TaskListSortArg) {
    tasks.sort_by(|left, right| {
        let primary = match sort {
            TaskListSortArg::Id => left.id.cmp(&right.id),
            TaskListSortArg::Title => compare_optional_text(&left.title, &right.title),
            TaskListSortArg::Project => {
                compare_optional_text(&left.work_list_title, &right.work_list_title)
            }
            TaskListSortArg::Priority => compare_optional_desc(left.priority, right.priority),
            TaskListSortArg::Due => compare_optional_asc(left.due_at, right.due_at),
            TaskListSortArg::Status => status_rank(left).cmp(&status_rank(right)),
            TaskListSortArg::Created => right.created_at.cmp(&left.created_at),
            TaskListSortArg::Updated => right.updated_at.cmp(&left.updated_at),
        };
        primary
            .then_with(|| left.work_list_id.cmp(&right.work_list_id))
            .then_with(|| left.id.cmp(&right.id))
    });
}

fn compare_optional_text(left: &Option<String>, right: &Option<String>) -> Ordering {
    match (left.as_deref(), right.as_deref()) {
        (Some(left), Some(right)) => normalized_text(left).cmp(&normalized_text(right)),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn normalized_text(value: &str) -> String {
    value.nfkc().flat_map(char::to_lowercase).collect()
}

fn compare_optional_desc<T: Ord>(left: Option<T>, right: Option<T>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => right.cmp(&left),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn compare_optional_asc<T: Ord>(left: Option<T>, right: Option<T>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left.cmp(&right),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use sealtask_client_runtime::AgentTaskSummary;

    #[test]
    fn web_origins_build_the_frontend_task_route() {
        let origin =
            resolve_web_origin(Some("https://app.example:8443/"), "https://api.example/v1")
                .expect("web origin");
        let project_id = Uuid::parse_str("01900000-0000-7000-8000-000000000001").expect("project");
        let task_id = Uuid::parse_str("01900000-0000-7000-8000-000000000002").expect("task");
        assert_eq!(
            task_url(&origin, project_id, task_id).as_str(),
            "https://app.example:8443/workspace/work-lists/01900000-0000-7000-8000-000000000001?task=01900000-0000-7000-8000-000000000002"
        );
    }

    #[test]
    fn unsafe_or_ambiguous_web_origins_are_rejected() {
        for value in [
            "file:///tmp/app",
            "https://user:secret@app.example/",
            "https://app.example/base",
            "https://app.example/?tenant=one",
            "https://app.example/#fragment",
        ] {
            assert!(
                resolve_web_origin(Some(value), "https://api.example/v1").is_err(),
                "{value}"
            );
        }
        assert_eq!(
            resolve_web_origin(None, "https://api.example/v1")
                .expect("derived origin")
                .as_str(),
            "https://api.example/"
        );
    }

    #[test]
    fn output_preflight_rejects_non_composable_or_ignored_controls() {
        assert!(
            validate_output_mode(OutputFormat::Json, &[], Some(TaskListFieldArg::Id), false,)
                .is_err()
        );
        assert!(validate_output_mode(OutputFormat::Table, &[], None, true,).is_err());
        assert!(
            validate_output_mode(
                OutputFormat::Table,
                &[TaskListColumnArg::Id, TaskListColumnArg::Id],
                None,
                false,
            )
            .is_err()
        );
        assert!(
            validate_output_mode(OutputFormat::Table, &[], Some(TaskListFieldArg::Url), true,)
                .is_ok()
        );
    }

    #[test]
    fn explicit_columns_keep_exact_order_and_are_all_required() {
        let columns = [
            TaskListColumnArg::Status,
            TaskListColumnArg::Title,
            TaskListColumnArg::Id,
        ];
        let table_columns = columns.into_iter().map(explicit_column).collect::<Vec<_>>();
        let headers = table_columns
            .iter()
            .map(|column| format!("{column:?}"))
            .collect::<Vec<_>>();
        assert_eq!(headers.len(), 3);

        let mut table = Table::new(table_columns);
        table.push_row(["Active", "Release", "id:01900000"]);
        let rendered = table.render_with_width(28);
        let header = rendered.lines().next().expect("header");
        assert!(header.starts_with("Status"));
        assert!(header.contains("Title"));
        assert!(header.ends_with("ID"));
    }

    #[test]
    fn natural_sorting_handles_priority_due_and_unreadable_values() {
        let now = Utc
            .with_ymd_and_hms(2026, 7, 26, 12, 0, 0)
            .single()
            .expect("timestamp");
        let mut tasks = vec![
            task(
                "01900000-0000-7000-8000-000000000003",
                None,
                None,
                None,
                now,
            ),
            task(
                "01900000-0000-7000-8000-000000000002",
                Some("zeta"),
                Some(3),
                Some(now),
                now,
            ),
            task(
                "01900000-0000-7000-8000-000000000001",
                Some("Alpha"),
                Some(8),
                Some(now - chrono::Duration::days(1)),
                now,
            ),
        ];

        sort_tasks(&mut tasks, TaskListSortArg::Title);
        assert_eq!(tasks[0].title.as_deref(), Some("Alpha"));
        assert!(tasks[2].title.is_none());
        sort_tasks(&mut tasks, TaskListSortArg::Priority);
        assert_eq!(
            tasks.iter().map(|task| task.priority).collect::<Vec<_>>(),
            [Some(8), Some(3), None]
        );
        sort_tasks(&mut tasks, TaskListSortArg::Due);
        assert_eq!(
            tasks[0].id.simple().to_string(),
            "01900000000070008000000000000001"
        );
        assert!(tasks[2].due_at.is_none());
    }

    #[test]
    fn empty_guidance_is_specific_to_scope_and_filters() {
        let project = ResolvedProject {
            id: Uuid::nil(),
            title: None,
        };
        assert!(
            empty_guidance(&TaskListScope::Current(project.clone()), false, false)
                .contains("--include-completed")
        );
        assert!(
            empty_guidance(&TaskListScope::Selected(project), true, true).contains("tasks create")
        );
        assert!(
            empty_guidance(&TaskListScope::AcrossProjects, false, false)
                .contains("--all --include-completed")
        );
    }

    fn task(
        id: &str,
        title: Option<&str>,
        priority: Option<i8>,
        due_at: Option<chrono::DateTime<Utc>>,
        updated_at: chrono::DateTime<Utc>,
    ) -> AgentTaskSummary {
        AgentTaskSummary {
            id: Uuid::parse_str(id).expect("task id"),
            work_list_id: Uuid::parse_str("01900000-0000-7000-8000-000000000010")
                .expect("project id"),
            work_list_title: Some("Operations".to_string()),
            work_list_timezone: Some("UTC".to_string()),
            created_by_membership_id: Uuid::nil(),
            section_id: None,
            priority,
            position: None,
            due_at,
            start_at: None,
            completed_at: None,
            archived_at: None,
            is_completed: false,
            recurrence_id: None,
            recurrence_schedule: None,
            recurrence_iteration: None,
            materialized_at: None,
            created_at: updated_at,
            updated_at,
            comment_count: 0,
            title: title.map(str::to_string),
            body_markdown: None,
            body_rich_text: None,
            checklist: None,
            attachments: None,
            references: None,
            mentions: None,
            client_meta: None,
            recurrence_state: None,
            delegations: Vec::new(),
            read_error: None,
        }
    }
}
