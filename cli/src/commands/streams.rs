use crate::live_output::LiveRegion;
use crate::output::{
    CliError, CliResult, OutputFormat, emit_warnings_best_effort, print_jsonl, warning_result,
    write_stderr_line, write_stdout_flushed, write_stdout_line_flushed,
};
use crate::output_models::{TaskSummaryV1, task_summaries_v1};
use crate::render::task_reference_title_label;
use crate::table::sanitize_cell;
use crate::task_list::render_default_project_task_table;
use crate::terminal;
use chrono::{DateTime, Utc};
use sealtask_client_api::BoardStreamEvent;
use sealtask_client_core::{PublicError, PublicResult};
use sealtask_client_runtime::{AgentTaskSummary, RuntimeClient};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use uuid::Uuid;
use zeroize::Zeroize;

const MAX_RECONNECT_DELAY: Duration = Duration::from_secs(60);
const STREAM_BURST_COALESCE_WINDOW: Duration = Duration::from_millis(10);
const MAX_COALESCED_STREAM_EVENTS: usize = 256;

pub(super) async fn watch_tasks(
    runtime: &RuntimeClient,
    format: OutputFormat,
    work_list_id: Uuid,
    include_completed: bool,
    include_archived: bool,
    password_stdin: bool,
) -> CliResult<()> {
    terminal::clear_active_progress();
    let mut session = runtime
        .project_task_session(work_list_id, password_stdin)
        .await?;
    let mut renderer = TaskWatchRenderer::new(format, work_list_id);
    let mut tasks = None;

    let shutdown = shutdown_signal();
    tokio::pin!(shutdown);
    let mut reconnect_attempt = 0_u32;

    loop {
        let connection = tokio::select! {
            biased;
            () = &mut shutdown => return renderer.interrupted(),
            connection = session.connect_events() => connection,
        };
        let mut stream = match connection {
            Ok(stream) => stream,
            Err(error) if reconnectable(&error) => {
                reconnect_attempt = reconnect_attempt.saturating_add(1);
                let Some(delay) = reconnect_delay(reconnect_attempt, error.retry_after()) else {
                    return Err(error.into());
                };
                renderer.connection_status(
                    "reconnecting",
                    reconnect_attempt,
                    Some(delay),
                    Some(error.code()),
                    tasks.as_deref(),
                )?;
                tokio::select! {
                    biased;
                    () = &mut shutdown => return renderer.interrupted(),
                    () = tokio::time::sleep(delay) => {}
                }
                continue;
            }
            Err(error) => return Err(error.into()),
        };

        // Subscribe before the authoritative fetch so a mutation cannot fall into
        // the gap between the snapshot and event subscription. The same ordering
        // closes reconnect races: any event queued during this fetch causes
        // another authoritative refresh below.
        let refreshed = tokio::select! {
            biased;
            () = &mut shutdown => return renderer.interrupted(),
            refreshed = session.list_tasks(include_completed, include_archived) => refreshed,
        };
        let refreshed = match refreshed {
            Ok(refreshed) => refreshed,
            Err(error) if reconnectable(&error) => {
                reconnect_attempt = reconnect_attempt.saturating_add(1);
                let Some(delay) = reconnect_delay(reconnect_attempt, error.retry_after()) else {
                    return Err(error.into());
                };
                renderer.connection_status(
                    "reconnecting",
                    reconnect_attempt,
                    Some(delay),
                    Some(error.code()),
                    tasks.as_deref(),
                )?;
                tokio::select! {
                    biased;
                    () = &mut shutdown => return renderer.interrupted(),
                    () = tokio::time::sleep(delay) => {}
                }
                continue;
            }
            Err(error) => return Err(error.into()),
        };
        if let Some(previous) = tasks.as_deref() {
            renderer.refresh(previous, &refreshed, "reconnect", None)?;
        } else {
            renderer.snapshot(&refreshed)?;
        }
        tasks = Some(refreshed);
        if reconnect_attempt > 0 {
            renderer.connection_status(
                "connected",
                reconnect_attempt,
                None,
                None,
                tasks.as_deref(),
            )?;
        }
        reconnect_attempt = 0;

        let disconnect_error = loop {
            let next = tokio::select! {
                biased;
                () = &mut shutdown => return renderer.interrupted(),
                next = stream.next_event() => next,
            };
            let event = match next {
                Some(Ok(event)) => event,
                Some(Err(error)) => break Some(error),
                None => break None,
            };

            let mut burst = StreamEventBurst::default();
            burst.observe(event);
            let mut stream_ended_after_refresh = false;
            let mut disconnect_after_refresh = None;
            let coalesce_deadline = tokio::time::sleep(STREAM_BURST_COALESCE_WINDOW);
            tokio::pin!(coalesce_deadline);
            while burst.event_count < MAX_COALESCED_STREAM_EVENTS {
                let next = tokio::select! {
                    biased;
                    () = &mut shutdown => return renderer.interrupted(),
                    next = stream.next_event() => next,
                    () = &mut coalesce_deadline => break,
                };
                match next {
                    Some(Ok(event)) => burst.observe(event),
                    Some(Err(error)) => {
                        stream_ended_after_refresh = true;
                        disconnect_after_refresh = Some(error);
                        break;
                    }
                    None => {
                        stream_ended_after_refresh = true;
                        break;
                    }
                }
            }

            let (trigger, missed_events) = burst.trigger();
            let refreshed = tokio::select! {
                biased;
                () = &mut shutdown => return renderer.interrupted(),
                refreshed = session.list_tasks(include_completed, include_archived) => refreshed,
            };
            let refreshed = match refreshed {
                Ok(tasks) => tasks,
                Err(error) if reconnectable(&error) => {
                    break disconnect_after_refresh.or(Some(error));
                }
                Err(error) => return Err(error.into()),
            };
            let previous = tasks
                .as_deref()
                .expect("snapshot established before events");
            renderer.refresh(previous, &refreshed, trigger, missed_events)?;
            tasks = Some(refreshed);
            if stream_ended_after_refresh {
                break disconnect_after_refresh;
            }
        };

        if disconnect_error
            .as_ref()
            .is_some_and(|error| !reconnectable(error))
        {
            return Err(disconnect_error
                .expect("non-reconnectable stream errors are present")
                .into());
        }
        reconnect_attempt = reconnect_attempt.saturating_add(1);
        let Some(delay) = reconnect_delay(
            reconnect_attempt,
            disconnect_error.as_ref().and_then(PublicError::retry_after),
        ) else {
            return Err(disconnect_error
                .expect("an oversized Retry-After requires a typed stream error")
                .into());
        };
        renderer.connection_status(
            "reconnecting",
            reconnect_attempt,
            Some(delay),
            disconnect_error.as_ref().map(PublicError::code),
            tasks.as_deref(),
        )?;
        tokio::select! {
            biased;
            () = &mut shutdown => return renderer.interrupted(),
            () = tokio::time::sleep(delay) => {}
        }
    }
}

struct TaskWatchRenderer {
    mode: StreamMode,
    work_list_id: Uuid,
    sequence: u64,
    live_region: LiveRegion,
    status: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StreamMode {
    LiveTerminal,
    AppendOnly,
    Jsonl,
}

#[derive(Debug, Default, Eq, PartialEq)]
struct StreamEventBurst {
    event_count: usize,
    missed_events: Option<u64>,
}

impl StreamEventBurst {
    fn observe(&mut self, event: BoardStreamEvent) {
        self.event_count = self.event_count.saturating_add(1);
        if let BoardStreamEvent::Resync { missed_events } = event {
            self.missed_events = Some(
                self.missed_events
                    .unwrap_or_default()
                    .saturating_add(missed_events),
            );
        }
    }

    fn trigger(&self) -> (&'static str, Option<u64>) {
        if self.missed_events.is_some() {
            ("resync", self.missed_events)
        } else {
            ("board_event", None)
        }
    }
}

struct TaskDiff<'current, 'previous> {
    added: Vec<&'current AgentTaskSummary>,
    updated: Vec<&'current AgentTaskSummary>,
    removed: Vec<&'previous AgentTaskSummary>,
    removed_task_ids: Vec<Uuid>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TaskWatchSnapshotV1<'a> {
    schema_version: u8,
    #[serde(rename = "type")]
    record_type: &'static str,
    sequence: u64,
    observed_at: DateTime<Utc>,
    work_list_id: Uuid,
    tasks: Vec<TaskSummaryV1<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TaskWatchRefreshV1<'a> {
    schema_version: u8,
    #[serde(rename = "type")]
    record_type: &'static str,
    sequence: u64,
    observed_at: DateTime<Utc>,
    work_list_id: Uuid,
    trigger: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    missed_events: Option<u64>,
    tasks: Vec<TaskSummaryV1<'a>>,
    added_task_ids: Vec<Uuid>,
    updated_task_ids: Vec<Uuid>,
    removed_task_ids: &'a [Uuid],
}

impl TaskWatchRenderer {
    fn new(format: OutputFormat, work_list_id: Uuid) -> Self {
        let mode = if format == OutputFormat::Jsonl {
            StreamMode::Jsonl
        } else if terminal::stdout_supports_live_updates() {
            StreamMode::LiveTerminal
        } else {
            StreamMode::AppendOnly
        };
        Self {
            mode,
            work_list_id,
            sequence: 0,
            live_region: LiveRegion::default(),
            status: "connected".to_string(),
        }
    }

    fn snapshot(&mut self, tasks: &[AgentTaskSummary]) -> CliResult<()> {
        match self.mode {
            StreamMode::Jsonl => print_jsonl(
                &TaskWatchSnapshotV1 {
                    schema_version: 1,
                    record_type: "snapshot",
                    sequence: self.next_sequence(),
                    observed_at: Utc::now(),
                    work_list_id: self.work_list_id,
                    tasks: task_summaries_v1(tasks),
                },
                "serializing task-watch snapshot should succeed",
            ),
            StreamMode::LiveTerminal => self.render_live(tasks),
            StreamMode::AppendOnly => {
                write_stderr_line(format_args!(
                    "Watching project id:{}; waiting for task changes",
                    self.work_list_id.simple()
                ))?;
                write_stdout_flushed(format_args!("{}", render_default_project_task_table(tasks)))
            }
        }
    }

    fn refresh(
        &mut self,
        previous: &[AgentTaskSummary],
        current: &[AgentTaskSummary],
        trigger: &'static str,
        missed_events: Option<u64>,
    ) -> CliResult<()> {
        let diff = task_diff(previous, current)?;
        match self.mode {
            StreamMode::Jsonl => {
                let sequence = self.next_sequence();
                print_jsonl(
                    &TaskWatchRefreshV1 {
                        schema_version: 1,
                        record_type: "refresh",
                        sequence,
                        observed_at: Utc::now(),
                        work_list_id: self.work_list_id,
                        trigger,
                        missed_events,
                        tasks: task_summaries_v1(current),
                        added_task_ids: diff.added.iter().map(|task| task.id).collect(),
                        updated_task_ids: diff.updated.iter().map(|task| task.id).collect(),
                        removed_task_ids: &diff.removed_task_ids,
                    },
                    "serializing task-watch refresh should succeed",
                )
            }
            StreamMode::LiveTerminal => self.render_live(current),
            StreamMode::AppendOnly => self.render_append_diff(&diff, trigger, missed_events),
        }
    }

    fn connection_status(
        &mut self,
        status: &'static str,
        attempt: u32,
        retry_in: Option<Duration>,
        error_code: Option<&'static str>,
        tasks: Option<&[AgentTaskSummary]>,
    ) -> CliResult<()> {
        self.status = match retry_in {
            Some(delay) => format!("{status} in {}s", delay.as_secs()),
            None => status.to_string(),
        };
        match self.mode {
            StreamMode::Jsonl => {
                emit_warnings_best_effort(
                    OutputFormat::Jsonl,
                    &[warning_result(
                        "stream_status",
                        stream_status_message(status, attempt, retry_in, error_code),
                    )],
                );
                Ok(())
            }
            StreamMode::LiveTerminal => match tasks {
                Some(tasks) => self.render_live(tasks),
                None => self.live_region.render(&format!(
                    "Watching project id:{} · {} · Ctrl-C to stop",
                    self.work_list_id.simple(),
                    sanitize_cell(&self.status),
                )),
            },
            StreamMode::AppendOnly => write_stderr_line(format_args!(
                "warning: {}",
                sanitize_cell(&stream_status_message(
                    status, attempt, retry_in, error_code
                ))
            )),
        }
    }

    fn interrupted(&mut self) -> CliResult<()> {
        if self.mode == StreamMode::LiveTerminal {
            self.live_region.finish();
        }
        Err(CliError::interrupted("task watch interrupted", &[]))
    }

    fn render_live(&mut self, tasks: &[AgentTaskSummary]) -> CliResult<()> {
        self.live_region.render(&format!(
            "Watching project id:{} · {} · Ctrl-C to stop\n{}",
            self.work_list_id.simple(),
            sanitize_cell(&self.status),
            render_default_project_task_table(tasks)
        ))
    }

    fn render_append_diff(
        &mut self,
        diff: &TaskDiff<'_, '_>,
        trigger: &str,
        missed_events: Option<u64>,
    ) -> CliResult<()> {
        let observed_at = Utc::now().to_rfc3339();
        for task in &diff.added {
            write_stdout_line_flushed(format_args!(
                "{observed_at} added id:{} {}",
                task.id.simple(),
                task_label(task)
            ))?;
        }
        for task in &diff.updated {
            write_stdout_line_flushed(format_args!(
                "{observed_at} updated id:{} {}",
                task.id.simple(),
                task_label(task)
            ))?;
        }
        for task in &diff.removed {
            write_stdout_line_flushed(format_args!(
                "{observed_at} removed id:{} {}",
                task.id.simple(),
                task_label(task)
            ))?;
        }
        if diff.added.is_empty() && diff.updated.is_empty() && diff.removed_task_ids.is_empty() {
            write_stdout_line_flushed(format_args!(
                "{observed_at} refreshed after {trigger}{}; no task changes",
                missed_events.map_or_else(String::new, |missed| {
                    format!(" ({missed} missed event(s))")
                })
            ))?;
        }
        Ok(())
    }

    fn next_sequence(&mut self) -> u64 {
        let sequence = self.sequence;
        self.sequence = self.sequence.saturating_add(1);
        sequence
    }
}

fn task_diff<'current, 'previous>(
    previous: &'previous [AgentTaskSummary],
    current: &'current [AgentTaskSummary],
) -> PublicResult<TaskDiff<'current, 'previous>> {
    let previous_by_id = previous
        .iter()
        .map(|task| task_fingerprint(task).map(|fingerprint| (task.id, fingerprint)))
        .collect::<PublicResult<HashMap<_, _>>>()?;
    let current_ids = current.iter().map(|task| task.id).collect::<HashSet<_>>();
    let mut added = Vec::new();
    let mut updated = Vec::new();
    for task in current {
        let fingerprint = task_fingerprint(task)?;
        match previous_by_id.get(&task.id) {
            None => added.push(task),
            Some(previous) if previous != &fingerprint => updated.push(task),
            Some(_) => {}
        }
    }
    let removed = previous
        .iter()
        .filter(|task| !current_ids.contains(&task.id))
        .collect::<Vec<_>>();
    let removed_task_ids = removed.iter().map(|task| task.id).collect();
    Ok(TaskDiff {
        added,
        updated,
        removed,
        removed_task_ids,
    })
}

fn task_fingerprint(task: &AgentTaskSummary) -> PublicResult<[u8; 32]> {
    let mut serialized = serde_json::to_vec(task).map_err(|error| {
        PublicError::unexpected(format!("failed to fingerprint task state: {error}"))
    })?;
    let fingerprint = Sha256::digest(&serialized).into();
    serialized.zeroize();
    Ok(fingerprint)
}

fn task_label(task: &AgentTaskSummary) -> String {
    sanitize_cell(&task_reference_title_label(task))
}

pub(super) fn reconnect_delay(attempt: u32, retry_after: Option<Duration>) -> Option<Duration> {
    if retry_after.is_some_and(|delay| delay > MAX_RECONNECT_DELAY) {
        return None;
    }
    let exponent = attempt.saturating_sub(1).min(5);
    let backoff = Duration::from_secs(1_u64 << exponent);
    Some(
        backoff
            .max(retry_after.unwrap_or_default())
            .min(MAX_RECONNECT_DELAY),
    )
}

pub(super) fn reconnectable(error: &PublicError) -> bool {
    match error.http_status() {
        Some(408 | 429) | Some(500..=599) => true,
        Some(_) => false,
        None => matches!(
            error.code(),
            "rate_limited"
                | "request_timeout"
                | "transport_timeout"
                | "transport_connect"
                | "transport_body"
                | "transport_other"
                | "response_body_read"
                | "response_body_truncated"
                | "response_transport"
        ),
    }
}

fn stream_status_message(
    status: &str,
    attempt: u32,
    retry_in: Option<Duration>,
    error_code: Option<&str>,
) -> String {
    format!(
        "task stream {status} (attempt {attempt}){}{}",
        retry_in.map_or_else(String::new, |delay| format!(" in {}s", delay.as_secs())),
        error_code.map_or_else(String::new, |code| format!(" after {code}")),
    )
}

pub(super) async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate());
        match terminate {
            Ok(mut terminate) => {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {}
                    _ = terminate.recv() => {}
                }
            }
            Err(_) => {
                let _ = tokio::signal::ctrl_c().await;
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sealtask_client_runtime::AgentDelegation;

    #[test]
    fn task_diffs_are_deterministic_and_include_non_revision_changes() {
        let first = task(Uuid::now_v7(), "one", 0);
        let removed = task(Uuid::now_v7(), "removed", 0);
        let mut changed = first.clone();
        changed.comment_count = 1;
        let added = task(Uuid::now_v7(), "added", 0);

        let previous = vec![first, removed.clone()];
        let current = vec![changed, added.clone()];
        let diff = task_diff(&previous, &current).expect("task diff");
        assert_eq!(
            diff.added.iter().map(|task| task.id).collect::<Vec<_>>(),
            [added.id]
        );
        assert_eq!(diff.updated.len(), 1);
        assert_eq!(diff.removed_task_ids, [removed.id]);
    }

    #[test]
    fn reconnect_backoff_is_bounded_and_honors_retry_after() {
        assert_eq!(reconnect_delay(1, None), Some(Duration::from_secs(1)));
        assert_eq!(reconnect_delay(6, None), Some(Duration::from_secs(32)));
        assert_eq!(
            reconnect_delay(2, Some(Duration::from_secs(60))),
            Some(Duration::from_secs(60))
        );
        assert_eq!(reconnect_delay(99, Some(Duration::from_secs(600))), None);
    }

    #[test]
    fn event_bursts_preserve_accumulated_resync_context() {
        let mut burst = StreamEventBurst::default();
        burst.observe(BoardStreamEvent::Resync { missed_events: 2 });
        burst.observe(BoardStreamEvent::Resync { missed_events: 3 });

        assert_eq!(burst.event_count, 2);
        assert_eq!(burst.trigger(), ("resync", Some(5)));
    }

    #[test]
    fn task_watch_labels_and_jsonl_summaries_include_decrypted_references() {
        let task = task(Uuid::now_v7(), "one", 0);
        assert_eq!(task_label(&task), "OPS-0031 · one");

        let tasks = vec![task];
        let record = serde_json::to_value(TaskWatchSnapshotV1 {
            schema_version: 1,
            record_type: "snapshot",
            sequence: 0,
            observed_at: Utc::now(),
            work_list_id: tasks[0].work_list_id,
            tasks: task_summaries_v1(&tasks),
        })
        .expect("task-watch snapshot");
        assert_eq!(record["tasks"][0]["reference"], "OPS-0031");
        assert_eq!(record["tasks"][0]["referenceNumber"], 31);
        assert_eq!(record["tasks"][0]["id"], tasks[0].id.to_string());
    }

    fn task(id: Uuid, title: &str, comment_count: i64) -> AgentTaskSummary {
        let now = Utc::now();
        AgentTaskSummary {
            id,
            work_list_id: Uuid::now_v7(),
            work_list_title: Some("Project".to_string()),
            work_list_timezone: Some("UTC".to_string()),
            created_by_membership_id: Uuid::now_v7(),
            section_id: None,
            priority: None,
            position: Some("a0".to_string()),
            due_at: None,
            start_at: None,
            completed_at: None,
            archived_at: None,
            is_completed: false,
            recurrence_id: None,
            recurrence_schedule: None,
            recurrence_iteration: None,
            materialized_at: None,
            created_at: now,
            updated_at: now,
            comment_count,
            reference_number: Some(31),
            reference: Some("OPS-0031".to_string()),
            title: Some(title.to_string()),
            body_markdown: None,
            body_rich_text: None,
            checklist: None,
            attachments: None,
            references: None,
            mentions: None,
            client_meta: None,
            recurrence_state: None,
            delegations: Vec::<AgentDelegation>::new(),
            read_error: None,
        }
    }
}
