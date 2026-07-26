use super::audit_output::{activity_line, audit_event_v1};
use super::streams::{reconnect_delay, reconnectable, shutdown_signal};
use crate::args::ActivityCommand;
use crate::live_output::LiveRegion;
use crate::operator_config::parse_human_duration;
use crate::output::{
    CliError, CliResult, OutputFormat, emit_warnings_best_effort, print_jsonl, warning_result,
    write_stderr_line, write_stdout_line_flushed,
};
use crate::table::{ellipsize, sanitize_cell, terminal_width};
use crate::terminal;
use chrono::{DateTime, Utc};
use sealtask_client_api::{AuditLogEvent, AuditLogPage, PublicApiClient};
use sealtask_client_core::{PublicError, PublicResult, ResponseFailureKind};
use sealtask_client_runtime::RuntimeClient;
use serde::Serialize;
use std::collections::{HashSet, VecDeque};
use std::time::Duration;
use uuid::Uuid;

const AUDIT_PAGE_LIMIT: u32 = 100;
const MAX_ACTIVITY_CATCH_UP_EVENTS: usize = 1_000;
const MAX_ACTIVITY_PAGES: usize = 100;
const LIVE_ACTIVITY_ROWS: usize = 20;
const MIN_POLL_INTERVAL: Duration = Duration::from_millis(250);
const MAX_POLL_INTERVAL: Duration = Duration::from_secs(5 * 60);

pub(crate) async fn run_activity(
    runtime: &RuntimeClient,
    format: OutputFormat,
    command: ActivityCommand,
) -> CliResult<()> {
    match command {
        ActivityCommand::Follow { interval, since } => {
            let interval = parse_human_duration(&interval, "activity poll interval")
                .map_err(PublicError::validation)?;
            if !(MIN_POLL_INTERVAL..=MAX_POLL_INTERVAL).contains(&interval) {
                return Err(PublicError::validation(
                    "activity poll interval must be between 250ms and 5m",
                )
                .into());
            }
            let since = parse_human_duration(&since, "activity history window")
                .map_err(PublicError::validation)?;
            follow_activity(runtime, format, interval, since).await
        }
    }
}

async fn follow_activity(
    runtime: &RuntimeClient,
    format: OutputFormat,
    poll_interval: Duration,
    since: Duration,
) -> CliResult<()> {
    terminal::clear_active_progress();
    let mut client = runtime.authenticated_api_client()?;
    let cutoff = Utc::now()
        .checked_sub_signed(chrono::Duration::from_std(since).map_err(|_| {
            PublicError::validation("activity history window exceeds the supported range")
        })?)
        .ok_or_else(|| PublicError::validation("activity history window is out of range"))?;
    let shutdown = shutdown_signal();
    tokio::pin!(shutdown);

    let initial = tokio::select! {
        biased;
        () = &mut shutdown => {
            return Err(CliError::interrupted("activity follow interrupted", &[]));
        }
        initial = load_initial_activity(&mut client, cutoff) => initial?,
    };
    let mut anchor = initial.anchor;
    let mut renderer = ActivityRenderer::new(format);
    renderer.start(&initial.events)?;

    let mut ticker = tokio::time::interval(poll_interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    ticker.tick().await;
    let mut retry_attempt = 0_u32;

    loop {
        tokio::select! {
            biased;
            () = &mut shutdown => return renderer.interrupted(),
            _ = ticker.tick() => {}
        }

        let catch_up = tokio::select! {
            biased;
            () = &mut shutdown => return renderer.interrupted(),
            catch_up = load_activity_since(&mut client, anchor) => catch_up,
        };
        match catch_up {
            Ok(ActivityCatchUp {
                events,
                newest_anchor,
            }) => {
                if let Some(newest_anchor) = newest_anchor {
                    anchor = Some(newest_anchor);
                }
                retry_attempt = 0;
                renderer.connected()?;
                renderer.events(&events)?;
            }
            Err(error) if reconnectable(&error) => {
                retry_attempt = retry_attempt.saturating_add(1);
                let Some(delay) = reconnect_delay(retry_attempt, error.retry_after()) else {
                    return Err(error.into());
                };
                renderer.retrying(retry_attempt, delay, error.code())?;
                tokio::select! {
                    biased;
                    () = &mut shutdown => return renderer.interrupted(),
                    () = tokio::time::sleep(delay) => {}
                }
            }
            Err(error) => return Err(error.into()),
        }
    }
}

struct InitialActivity {
    events: Vec<AuditLogEvent>,
    anchor: Option<Uuid>,
}

struct ActivityCatchUp {
    events: Vec<AuditLogEvent>,
    newest_anchor: Option<Uuid>,
}

async fn load_initial_activity(
    client: &mut PublicApiClient,
    cutoff: DateTime<Utc>,
) -> PublicResult<InitialActivity> {
    let mut cursor = None;
    let mut seen_cursors = HashSet::new();
    let mut seen_event_ids = HashSet::new();
    let mut events = Vec::new();
    let mut anchor = None;
    let mut page_count = 0_usize;

    'pages: loop {
        let page = client.get_my_activity(cursor, AUDIT_PAGE_LIMIT).await?;
        page_count = page_count.saturating_add(1);
        validate_activity_page(&page, page_count, &mut seen_event_ids)?;
        if anchor.is_none() {
            anchor = page.events.first().map(|event| event.id);
        }
        for event in page.events {
            if event.occurred_at < cutoff {
                break 'pages;
            }
            if events.len() >= MAX_ACTIVITY_CATCH_UP_EVENTS {
                return Err(PublicError::validation(format!(
                    "activity history exceeds the {MAX_ACTIVITY_CATCH_UP_EVENTS}-event safety limit; choose a smaller --since window"
                )));
            }
            events.push(event);
        }
        let Some(next_cursor) = checked_next_cursor(page.next_cursor, &mut seen_cursors)? else {
            break;
        };
        cursor = Some(next_cursor);
    }

    // The endpoint is newest-first; terminals and JSONL consumers receive a
    // chronological history before the live anchor begins.
    events.reverse();
    Ok(InitialActivity { events, anchor })
}

async fn load_activity_since(
    client: &mut PublicApiClient,
    anchor: Option<Uuid>,
) -> PublicResult<ActivityCatchUp> {
    let mut cursor = None;
    let mut seen_cursors = HashSet::new();
    let mut seen_event_ids = HashSet::new();
    let mut events = Vec::new();
    let mut newest_anchor = None;
    let mut page_count = 0_usize;

    loop {
        let page = client.get_my_activity(cursor, AUDIT_PAGE_LIMIT).await?;
        page_count = page_count.saturating_add(1);
        validate_activity_page(&page, page_count, &mut seen_event_ids)?;
        if newest_anchor.is_none() {
            newest_anchor = page.events.first().map(|event| event.id);
        }
        for event in page.events {
            if Some(event.id) == anchor {
                events.reverse();
                return Ok(ActivityCatchUp {
                    events,
                    newest_anchor,
                });
            }
            if events.len() >= MAX_ACTIVITY_CATCH_UP_EVENTS {
                return Err(PublicError::validation(format!(
                    "new activity exceeds the {MAX_ACTIVITY_CATCH_UP_EVENTS}-event catch-up safety limit; rerun with a shorter polling interval"
                )));
            }
            events.push(event);
        }
        let Some(next_cursor) = checked_next_cursor(page.next_cursor, &mut seen_cursors)? else {
            // The anchor may have aged out of retained history. Reaching the
            // authoritative end still gives us a complete retained sequence;
            // emit it oldest-first and establish the newest retained anchor.
            events.reverse();
            return Ok(ActivityCatchUp {
                events,
                newest_anchor,
            });
        };
        cursor = Some(next_cursor);
    }
}

fn validate_activity_page(
    page: &AuditLogPage,
    page_count: usize,
    seen_event_ids: &mut HashSet<Uuid>,
) -> PublicResult<()> {
    if page_count > MAX_ACTIVITY_PAGES {
        return Err(PublicError::response(
            ResponseFailureKind::JsonSchema,
            format!("API activity pagination exceeds the {MAX_ACTIVITY_PAGES}-page safety limit"),
        ));
    }
    if page.events.is_empty() && page.next_cursor.is_some() {
        return Err(PublicError::response(
            ResponseFailureKind::JsonSchema,
            "API activity pagination returned an empty page with a continuation cursor",
        ));
    }
    if page
        .events
        .iter()
        .any(|event| !seen_event_ids.insert(event.id))
    {
        return Err(PublicError::response(
            ResponseFailureKind::JsonSchema,
            "API activity pagination repeated an event ID",
        ));
    }
    Ok(())
}

fn checked_next_cursor(
    next_cursor: Option<Uuid>,
    seen_cursors: &mut HashSet<Uuid>,
) -> PublicResult<Option<Uuid>> {
    let Some(next_cursor) = next_cursor else {
        return Ok(None);
    };
    if !seen_cursors.insert(next_cursor) {
        return Err(PublicError::response(
            ResponseFailureKind::JsonSchema,
            "API activity pagination repeated a cursor",
        ));
    }
    Ok(Some(next_cursor))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ActivityMode {
    LiveTerminal,
    AppendOnly,
    Jsonl,
}

struct ActivityRenderer {
    mode: ActivityMode,
    sequence: u64,
    live_region: LiveRegion,
    recent: VecDeque<String>,
    status: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ActivityRecordV1<'a> {
    schema_version: u8,
    #[serde(rename = "type")]
    record_type: &'static str,
    sequence: u64,
    observed_at: DateTime<Utc>,
    event: super::audit_output::AuditEventV1<'a>,
}

impl ActivityRenderer {
    fn new(format: OutputFormat) -> Self {
        let mode = if format == OutputFormat::Jsonl {
            ActivityMode::Jsonl
        } else if terminal::stdout_supports_live_updates() {
            ActivityMode::LiveTerminal
        } else {
            ActivityMode::AppendOnly
        };
        Self {
            mode,
            sequence: 0,
            live_region: LiveRegion::default(),
            recent: VecDeque::with_capacity(LIVE_ACTIVITY_ROWS),
            status: "connected".to_string(),
        }
    }

    fn start(&mut self, events: &[AuditLogEvent]) -> CliResult<()> {
        self.events(events)?;
        if events.is_empty() {
            match self.mode {
                ActivityMode::LiveTerminal => self.render_live()?,
                ActivityMode::AppendOnly => {
                    write_stderr_line(format_args!(
                        "Following account activity; waiting for new events"
                    ))?;
                }
                ActivityMode::Jsonl => {}
            }
        }
        Ok(())
    }

    fn events(&mut self, events: &[AuditLogEvent]) -> CliResult<()> {
        for event in events {
            match self.mode {
                ActivityMode::Jsonl => {
                    let sequence = self.next_sequence();
                    print_jsonl(
                        &ActivityRecordV1 {
                            schema_version: 1,
                            record_type: "activity",
                            sequence,
                            observed_at: Utc::now(),
                            event: audit_event_v1(event),
                        },
                        "serializing activity record should succeed",
                    )?;
                }
                ActivityMode::AppendOnly => {
                    write_stdout_line_flushed(format_args!("{}", activity_line(event)))?;
                }
                ActivityMode::LiveTerminal => {
                    if self.recent.len() == LIVE_ACTIVITY_ROWS {
                        self.recent.pop_front();
                    }
                    self.recent
                        .push_back(bounded_activity_line(event, terminal_width()));
                }
            }
        }
        if self.mode == ActivityMode::LiveTerminal && !events.is_empty() {
            self.render_live()?;
        }
        Ok(())
    }

    fn connected(&mut self) -> CliResult<()> {
        if self.status == "connected" {
            return Ok(());
        }
        self.status = "connected".to_string();
        match self.mode {
            ActivityMode::LiveTerminal => self.render_live()?,
            ActivityMode::AppendOnly => {
                write_stderr_line(format_args!(
                    "Activity polling recovered; following new events"
                ))?;
            }
            ActivityMode::Jsonl => {}
        }
        Ok(())
    }

    fn retrying(&mut self, attempt: u32, delay: Duration, error_code: &str) -> CliResult<()> {
        self.status = format!("retrying in {}s", delay.as_secs());
        let message = format!(
            "activity poll failed after {error_code}; retrying in {}s (attempt {attempt})",
            delay.as_secs()
        );
        match self.mode {
            ActivityMode::LiveTerminal => self.render_live(),
            ActivityMode::AppendOnly => {
                write_stderr_line(format_args!("warning: {}", sanitize_cell(&message)))
            }
            ActivityMode::Jsonl => {
                emit_warnings_best_effort(
                    OutputFormat::Jsonl,
                    &[warning_result("activity_retry", message)],
                );
                Ok(())
            }
        }
    }

    fn render_live(&mut self) -> CliResult<()> {
        let mut frame = format!(
            "Following account activity · {} · Ctrl-C to stop\n",
            sanitize_cell(&self.status)
        );
        if self.recent.is_empty() {
            frame.push_str("No activity in the selected history window.");
        } else {
            let width = terminal_width();
            for line in &self.recent {
                frame.push_str(&ellipsize(line, width));
                frame.push('\n');
            }
        }
        self.live_region.render(&frame)
    }

    fn interrupted(&mut self) -> CliResult<()> {
        if self.mode == ActivityMode::LiveTerminal {
            self.live_region.finish();
        }
        Err(CliError::interrupted("activity follow interrupted", &[]))
    }

    fn next_sequence(&mut self) -> u64 {
        let sequence = self.sequence;
        self.sequence = self.sequence.saturating_add(1);
        sequence
    }
}

fn bounded_activity_line(event: &AuditLogEvent, width: usize) -> String {
    ellipsize(&activity_line(event), width)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_activity_cursors_are_rejected() {
        let cursor = Uuid::now_v7();
        let mut seen = HashSet::new();
        assert_eq!(
            checked_next_cursor(Some(cursor), &mut seen).expect("first cursor"),
            Some(cursor)
        );
        let error = checked_next_cursor(Some(cursor), &mut seen).expect_err("repeat rejected");
        assert_eq!(error.code(), "response_json_schema");
    }

    #[test]
    fn poll_interval_bounds_are_operator_safe() {
        assert!(MIN_POLL_INTERVAL <= Duration::from_secs(5));
        assert!(MAX_POLL_INTERVAL >= Duration::from_secs(5));
        assert!(MIN_POLL_INTERVAL < MAX_POLL_INTERVAL);
    }

    #[test]
    fn empty_activity_pages_cannot_extend_pagination() {
        let page = AuditLogPage {
            events: Vec::new(),
            next_cursor: Some(Uuid::now_v7()),
        };
        let error =
            validate_activity_page(&page, 1, &mut HashSet::new()).expect_err("empty continuation");
        assert_eq!(error.code(), "response_json_schema");
    }

    #[test]
    fn activity_page_count_is_bounded_even_with_distinct_cursors() {
        let page = AuditLogPage {
            events: Vec::new(),
            next_cursor: None,
        };
        let error = validate_activity_page(&page, MAX_ACTIVITY_PAGES + 1, &mut HashSet::new())
            .expect_err("page cap");
        assert_eq!(error.code(), "response_json_schema");
    }

    #[test]
    fn duplicate_activity_event_ids_are_rejected() {
        let event_id = Uuid::now_v7();
        let page = AuditLogPage {
            events: vec![activity_event(event_id), activity_event(event_id)],
            next_cursor: None,
        };
        let error =
            validate_activity_page(&page, 1, &mut HashSet::new()).expect_err("duplicate event");
        assert_eq!(error.code(), "response_json_schema");
    }

    #[test]
    fn retained_live_activity_lines_are_sanitized_and_width_bounded() {
        let mut event = activity_event(Uuid::now_v7());
        event.actor_user_name = Some(format!("operator\u{1b}[31m{}", "x".repeat(100)));

        let line = bounded_activity_line(&event, 24);

        assert!(!line.contains('\u{1b}'));
        assert!(crate::table::display_width(&line) <= 24);
    }

    fn activity_event(id: Uuid) -> AuditLogEvent {
        AuditLogEvent {
            id,
            workspace_id: Uuid::now_v7(),
            work_list_id: Some(Uuid::now_v7()),
            task_id: Some(Uuid::now_v7()),
            comment_id: None,
            entity_type: "task".to_string(),
            entity_id: Uuid::now_v7(),
            action: "updated".to_string(),
            scope_level: "task".to_string(),
            actor_user_id: None,
            actor_user_name: None,
            actor_membership_id: None,
            actor_type: "system".to_string(),
            source_kind: "api".to_string(),
            target_version: None,
            client_version: None,
            occurred_at: Utc::now(),
            changes: Vec::new(),
            payload_present: false,
        }
    }
}
