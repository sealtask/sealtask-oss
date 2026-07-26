use crate::output::{CliResult, OutputFormat, print_json};
use crate::table::{Alignment, Column, Table, sanitize_cell, short_unique_ids};
use sealtask_client_api::{AuditLogChange, AuditLogEvent, AuditLogPage, AuditLogValue};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AuditPageV1<'a> {
    schema_version: u8,
    project_id: Uuid,
    events: Vec<AuditEventV1<'a>>,
    next_cursor: Option<Uuid>,
}

/// An intentionally explicit projection of public audit metadata.
///
/// In particular, this type cannot acquire the backend's encrypted payload
/// field through flattening or by serializing a transport response wholesale.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AuditEventV1<'a> {
    id: Uuid,
    workspace_id: Uuid,
    work_list_id: Option<Uuid>,
    task_id: Option<Uuid>,
    comment_id: Option<Uuid>,
    entity_type: &'a str,
    entity_id: Uuid,
    action: &'a str,
    scope_level: &'a str,
    actor_user_id: Option<Uuid>,
    actor_user_name: Option<&'a str>,
    actor_membership_id: Option<Uuid>,
    actor_type: &'a str,
    source_kind: &'a str,
    target_version: Option<i64>,
    client_version: Option<&'a str>,
    occurred_at: &'a chrono::DateTime<chrono::Utc>,
    changes: Vec<AuditChangeV1<'a>>,
    payload_present: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AuditChangeV1<'a> {
    field_key: &'a str,
    change_kind: &'a str,
    before_value: Option<&'a AuditLogValue>,
    after_value: Option<&'a AuditLogValue>,
    before_ciphertext_digest: Option<&'a str>,
    after_ciphertext_digest: Option<&'a str>,
}

impl<'a> From<&'a AuditLogEvent> for AuditEventV1<'a> {
    fn from(event: &'a AuditLogEvent) -> Self {
        Self {
            id: event.id,
            workspace_id: event.workspace_id,
            work_list_id: event.work_list_id,
            task_id: event.task_id,
            comment_id: event.comment_id,
            entity_type: &event.entity_type,
            entity_id: event.entity_id,
            action: &event.action,
            scope_level: &event.scope_level,
            actor_user_id: event.actor_user_id,
            actor_user_name: event.actor_user_name.as_deref(),
            actor_membership_id: event.actor_membership_id,
            actor_type: &event.actor_type,
            source_kind: &event.source_kind,
            target_version: event.target_version,
            client_version: event.client_version.as_deref(),
            occurred_at: &event.occurred_at,
            changes: event.changes.iter().map(Into::into).collect(),
            payload_present: event.payload_present,
        }
    }
}

impl<'a> From<&'a AuditLogChange> for AuditChangeV1<'a> {
    fn from(change: &'a AuditLogChange) -> Self {
        Self {
            field_key: &change.field_key,
            change_kind: &change.change_kind,
            before_value: change.before_value.as_ref(),
            after_value: change.after_value.as_ref(),
            before_ciphertext_digest: change.before_ciphertext_digest.as_deref(),
            after_ciphertext_digest: change.after_ciphertext_digest.as_deref(),
        }
    }
}

pub(super) fn audit_event_v1(event: &AuditLogEvent) -> AuditEventV1<'_> {
    event.into()
}

pub(super) fn print_audit_page(
    project_id: Uuid,
    page: &AuditLogPage,
    format: OutputFormat,
) -> CliResult<()> {
    match format {
        OutputFormat::Json | OutputFormat::JsonPretty | OutputFormat::Jsonl => print_json(
            &AuditPageV1 {
                schema_version: 1,
                project_id,
                events: page.events.iter().map(Into::into).collect(),
                next_cursor: page.next_cursor,
            },
            format,
            "serializing safe audit page should succeed",
        ),
        OutputFormat::Table => print_audit_table(project_id, page),
    }
}

fn print_audit_table(project_id: Uuid, page: &AuditLogPage) -> CliResult<()> {
    if page.events.is_empty() {
        println!("No audit entries found.");
        print_next_cursor(project_id, page.next_cursor)?;
        return Ok(());
    }

    let ids = short_unique_ids(&page.events.iter().map(|event| event.id).collect::<Vec<_>>());
    let mut table = Table::new([
        Column::required("Time", 20, 25),
        Column::optional("Actor", 10, 24, 30).flex(2),
        Column::required("Action", 8, 24).flex(2),
        Column::required("Entity", 10, 28).flex(2),
        Column::optional("Changes", 7, 7, 20).align(Alignment::Right),
        Column::optional("ID", 11, 39, 10).preserve(),
    ]);
    for (event, id) in page.events.iter().zip(ids) {
        table.push_row([
            event.occurred_at.to_rfc3339(),
            audit_actor(event),
            event.action.clone(),
            format!("{} id:{}", event.entity_type, short_id(event.entity_id)),
            event.changes.len().to_string(),
            id,
        ]);
    }

    print!("{}", table.render());
    println!("Total: {} audit event(s)", page.events.len());
    print_next_cursor(project_id, page.next_cursor)
}

fn print_next_cursor(project_id: Uuid, next_cursor: Option<Uuid>) -> CliResult<()> {
    if let Some(cursor) = next_cursor {
        println!(
            "Next: sealtask projects audit id:{} --cursor {cursor}",
            project_id.simple()
        );
    }
    Ok(())
}

pub(super) fn activity_line(event: &AuditLogEvent) -> String {
    sanitize_cell(&format!(
        "{}  {:<20}  {:<18}  {} id:{}",
        event.occurred_at.to_rfc3339(),
        audit_actor(event),
        event.action,
        event.entity_type,
        short_id(event.entity_id),
    ))
}

fn audit_actor(event: &AuditLogEvent) -> String {
    event
        .actor_user_name
        .as_deref()
        .filter(|name| !name.trim().is_empty())
        .map(sanitize_cell)
        .or_else(|| {
            event
                .actor_user_id
                .map(|id| format!("user:{}", short_id(id)))
        })
        .unwrap_or_else(|| sanitize_cell(&event.actor_type))
}

fn short_id(id: Uuid) -> String {
    id.simple().to_string()[..8].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn safe_audit_projection_never_contains_transport_payload_fields() {
        let event = AuditLogEvent {
            id: Uuid::now_v7(),
            workspace_id: Uuid::now_v7(),
            work_list_id: Some(Uuid::now_v7()),
            task_id: Some(Uuid::now_v7()),
            comment_id: None,
            entity_type: "task".to_string(),
            entity_id: Uuid::now_v7(),
            action: "updated".to_string(),
            scope_level: "task".to_string(),
            actor_user_id: Some(Uuid::now_v7()),
            actor_user_name: Some("Operator".to_string()),
            actor_membership_id: Some(Uuid::now_v7()),
            actor_type: "user".to_string(),
            source_kind: "api".to_string(),
            target_version: Some(2),
            client_version: Some("test".to_string()),
            occurred_at: Utc::now(),
            changes: Vec::new(),
            payload_present: true,
        };

        let value = serde_json::to_value(AuditEventV1::from(&event)).expect("serialize audit");
        assert!(value.get("payload").is_none());
        assert!(value.get("payloadCiphertext").is_none());
        assert_eq!(value["payloadPresent"], true);
        assert_eq!(value["entityType"], "task");
    }

    #[test]
    fn activity_lines_strip_terminal_controls() {
        let event = AuditLogEvent {
            id: Uuid::now_v7(),
            workspace_id: Uuid::now_v7(),
            work_list_id: None,
            task_id: None,
            comment_id: None,
            entity_type: "task\u{1b}[2J".to_string(),
            entity_id: Uuid::now_v7(),
            action: "updated\nforged".to_string(),
            scope_level: "task".to_string(),
            actor_user_id: None,
            actor_user_name: Some("name\rforged".to_string()),
            actor_membership_id: None,
            actor_type: "user".to_string(),
            source_kind: "api".to_string(),
            target_version: None,
            client_version: None,
            occurred_at: Utc::now(),
            changes: Vec::new(),
            payload_present: false,
        };

        let line = activity_line(&event);
        assert!(!line.contains('\n'));
        assert!(!line.contains('\r'));
        assert!(!line.contains('\u{1b}'));
    }
}
