use std::fmt;

use chrono::{DateTime, Utc};
use sealtask_client_core::{PublicError, PublicResult, ResponseFailureKind};
use serde::{Deserialize, Deserializer, Serialize, de::IgnoredAny};
use uuid::Uuid;

use crate::transport::PublicApiClient;

pub const MAX_AUDIT_LOG_PAGE_ITEMS: u32 = 100;
pub const MAX_AUDIT_LOG_CHANGES_PER_EVENT: usize = 100;
pub const MAX_AUDIT_LOG_SECTIONS_PER_CHANGE: usize = 100;
pub const MAX_AUDIT_LOG_RESPONSE_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogPage {
    pub events: Vec<AuditLogEvent>,
    pub next_cursor: Option<Uuid>,
}

impl fmt::Debug for AuditLogPage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuditLogPage")
            .field("events", &self.events)
            .field("next_cursor", &self.next_cursor)
            .finish()
    }
}

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogEvent {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub work_list_id: Option<Uuid>,
    pub task_id: Option<Uuid>,
    pub comment_id: Option<Uuid>,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub action: String,
    pub scope_level: String,
    pub actor_user_id: Option<Uuid>,
    pub actor_user_name: Option<String>,
    pub actor_membership_id: Option<Uuid>,
    pub actor_agent_id: Option<Uuid>,
    pub actor_agent_handle: Option<String>,
    pub actor_agent_display_name: Option<String>,
    pub actor_type: String,
    pub source_kind: String,
    pub target_version: Option<i64>,
    pub client_version: Option<String>,
    pub occurred_at: DateTime<Utc>,
    pub changes: Vec<AuditLogChange>,
    #[serde(
        default,
        rename(deserialize = "payload", serialize = "payloadPresent"),
        deserialize_with = "deserialize_payload_presence"
    )]
    pub payload_present: bool,
}

impl fmt::Debug for AuditLogEvent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuditLogEvent")
            .field("id", &self.id)
            .field("workspace_id", &self.workspace_id)
            .field("work_list_id", &self.work_list_id)
            .field("task_id", &self.task_id)
            .field("comment_id", &self.comment_id)
            .field("entity_type", &self.entity_type)
            .field("entity_id", &self.entity_id)
            .field("action", &self.action)
            .field("scope_level", &self.scope_level)
            .field("actor_user_id", &self.actor_user_id)
            .field(
                "actor_user_name",
                &self.actor_user_name.as_ref().map(|_| "<redacted>"),
            )
            .field("actor_membership_id", &self.actor_membership_id)
            .field("actor_agent_id", &self.actor_agent_id)
            .field(
                "actor_agent_handle",
                &self.actor_agent_handle.as_ref().map(|_| "<redacted>"),
            )
            .field(
                "actor_agent_display_name",
                &self.actor_agent_display_name.as_ref().map(|_| "<redacted>"),
            )
            .field("actor_type", &self.actor_type)
            .field("source_kind", &self.source_kind)
            .field("target_version", &self.target_version)
            .field(
                "client_version",
                &self.client_version.as_ref().map(|_| "<redacted>"),
            )
            .field("occurred_at", &self.occurred_at)
            .field("changes", &self.changes)
            .field("payload_present", &self.payload_present)
            .finish()
    }
}

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogChange {
    pub field_key: String,
    pub change_kind: String,
    pub before_value: Option<AuditLogValue>,
    pub after_value: Option<AuditLogValue>,
    pub before_ciphertext_digest: Option<String>,
    pub after_ciphertext_digest: Option<String>,
}

impl fmt::Debug for AuditLogChange {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuditLogChange")
            .field("field_key", &self.field_key)
            .field("change_kind", &self.change_kind)
            .field("before_value", &self.before_value)
            .field("after_value", &self.after_value)
            .field(
                "before_ciphertext_digest",
                &self.before_ciphertext_digest.as_ref().map(|_| "<redacted>"),
            )
            .field(
                "after_ciphertext_digest",
                &self.after_ciphertext_digest.as_ref().map(|_| "<redacted>"),
            )
            .finish()
    }
}

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuditLogValue {
    String {
        value: String,
    },
    Integer {
        value: i64,
    },
    Boolean {
        value: bool,
    },
    Timestamp {
        value: DateTime<Utc>,
    },
    Uuid {
        value: Uuid,
    },
    #[serde(rename_all = "camelCase")]
    SectionRef {
        section_id: Option<Uuid>,
    },
    #[serde(rename_all = "camelCase")]
    TaskMove {
        to_section_id: Option<Uuid>,
    },
    #[serde(rename_all = "camelCase")]
    SectionSort {
        section_id: Option<Uuid>,
        sort_field: String,
    },
    #[serde(rename_all = "camelCase")]
    AutoArchive {
        enabled: bool,
        after_days: Option<i32>,
    },
    #[serde(rename_all = "camelCase")]
    SectionsSync {
        applied_snapshots_count: usize,
        removed_sections_count: usize,
        applied_snapshots: Option<Vec<AuditSectionSnapshot>>,
        removed_section_ids: Option<Vec<Uuid>>,
    },
}

impl fmt::Debug for AuditLogValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String { .. } => formatter
                .debug_struct("String")
                .field("value", &"<redacted>")
                .finish(),
            Self::Integer { value } => formatter
                .debug_struct("Integer")
                .field("value", value)
                .finish(),
            Self::Boolean { value } => formatter
                .debug_struct("Boolean")
                .field("value", value)
                .finish(),
            Self::Timestamp { value } => formatter
                .debug_struct("Timestamp")
                .field("value", value)
                .finish(),
            Self::Uuid { value } => formatter
                .debug_struct("Uuid")
                .field("value", value)
                .finish(),
            Self::SectionRef { section_id } => formatter
                .debug_struct("SectionRef")
                .field("section_id", section_id)
                .finish(),
            Self::TaskMove { to_section_id } => formatter
                .debug_struct("TaskMove")
                .field("to_section_id", to_section_id)
                .finish(),
            Self::SectionSort {
                section_id,
                sort_field: _,
            } => formatter
                .debug_struct("SectionSort")
                .field("section_id", section_id)
                .field("sort_field", &"<redacted>")
                .finish(),
            Self::AutoArchive {
                enabled,
                after_days,
            } => formatter
                .debug_struct("AutoArchive")
                .field("enabled", enabled)
                .field("after_days", after_days)
                .finish(),
            Self::SectionsSync {
                applied_snapshots_count,
                removed_sections_count,
                applied_snapshots,
                removed_section_ids,
            } => formatter
                .debug_struct("SectionsSync")
                .field("applied_snapshots_count", applied_snapshots_count)
                .field("removed_sections_count", removed_sections_count)
                .field(
                    "applied_snapshots",
                    &applied_snapshots.as_ref().map(Vec::len),
                )
                .field(
                    "removed_section_ids",
                    &removed_section_ids.as_ref().map(Vec::len),
                )
                .finish(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditSectionSnapshot {
    pub id: Uuid,
    pub position: i32,
    pub auto_archive_enabled: bool,
    pub auto_archive_after_days: Option<i32>,
}

impl PublicApiClient {
    pub async fn get_my_activity(
        &mut self,
        cursor: Option<Uuid>,
        limit: u32,
    ) -> PublicResult<AuditLogPage> {
        validate_audit_limit(limit)?;
        let path = audit_path("/me/activity", cursor, limit);
        self.get_audit_page(&path, limit).await
    }

    pub async fn get_work_list_audit_log(
        &mut self,
        work_list_id: Uuid,
        cursor: Option<Uuid>,
        limit: u32,
    ) -> PublicResult<AuditLogPage> {
        validate_audit_limit(limit)?;
        let path = audit_path(
            &format!("/work-lists/{work_list_id}/audit-log"),
            cursor,
            limit,
        );
        self.get_audit_page(&path, limit).await
    }

    async fn get_audit_page(&mut self, path: &str, limit: u32) -> PublicResult<AuditLogPage> {
        let page: AuditLogPage = self.get_bounded(path, MAX_AUDIT_LOG_RESPONSE_BYTES).await?;
        validate_audit_page(&page, limit)?;
        Ok(page)
    }
}

fn audit_path(base: &str, cursor: Option<Uuid>, limit: u32) -> String {
    match cursor {
        Some(cursor) => format!("{base}?cursor={cursor}&limit={limit}"),
        None => format!("{base}?limit={limit}"),
    }
}

fn validate_audit_limit(limit: u32) -> PublicResult<()> {
    if !(1..=MAX_AUDIT_LOG_PAGE_ITEMS).contains(&limit) {
        return Err(PublicError::validation(format!(
            "audit log limit must be between 1 and {MAX_AUDIT_LOG_PAGE_ITEMS}"
        )));
    }
    Ok(())
}

fn validate_audit_page(page: &AuditLogPage, requested_limit: u32) -> PublicResult<()> {
    let requested_limit = usize::try_from(requested_limit)
        .map_err(|_| PublicError::unexpected("audit log limit exceeds the supported range"))?;
    if page.events.len() > requested_limit
        || page.events.len()
            > usize::try_from(MAX_AUDIT_LOG_PAGE_ITEMS)
                .map_err(|_| PublicError::unexpected("audit log item limit is invalid"))?
    {
        return Err(PublicError::response(
            ResponseFailureKind::JsonSchema,
            "API audit log response exceeds the requested page size",
        ));
    }
    if page
        .events
        .iter()
        .any(|event| event.changes.len() > MAX_AUDIT_LOG_CHANGES_PER_EVENT)
    {
        return Err(PublicError::response(
            ResponseFailureKind::JsonSchema,
            "API audit log event contains too many field changes",
        ));
    }
    if page.events.iter().any(|event| {
        event.changes.iter().any(|change| {
            [&change.before_value, &change.after_value]
                .into_iter()
                .flatten()
                .any(audit_value_has_too_many_sections)
        })
    }) {
        return Err(PublicError::response(
            ResponseFailureKind::JsonSchema,
            "API audit log field change contains too many section entries",
        ));
    }
    Ok(())
}

fn audit_value_has_too_many_sections(value: &AuditLogValue) -> bool {
    let AuditLogValue::SectionsSync {
        applied_snapshots,
        removed_section_ids,
        ..
    } = value
    else {
        return false;
    };

    applied_snapshots
        .as_ref()
        .is_some_and(|sections| sections.len() > MAX_AUDIT_LOG_SECTIONS_PER_CHANGE)
        || removed_section_ids
            .as_ref()
            .is_some_and(|sections| sections.len() > MAX_AUDIT_LOG_SECTIONS_PER_CHANGE)
}

fn deserialize_payload_presence<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<IgnoredAny>::deserialize(deserializer).map(|payload| payload.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration as ChronoDuration;
    use sealtask_client_auth::Credentials;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    const ACTOR_NAME_CANARY: &str = "AUDIT-ACTOR-NAME-CANARY";
    const ACTOR_AGENT_HANDLE_CANARY: &str = "AUDIT-AGENT-HANDLE-CANARY";
    const ACTOR_AGENT_NAME_CANARY: &str = "AUDIT-AGENT-NAME-CANARY";
    const SCALAR_CANARY: &str = "AUDIT-SCALAR-CANARY";
    const PAYLOAD_CANARY: &str = "AUDIT-PAYLOAD-CANARY";

    fn test_credentials(api_url: &str) -> Credentials {
        Credentials {
            api_url: api_url.to_string(),
            access_token: "test-access".to_string(),
            refresh_token: "test-refresh".to_string(),
            access_expires_at: Utc::now() + ChronoDuration::hours(1),
            refresh_expires_at: Utc::now() + ChronoDuration::hours(2),
            user_id: Uuid::now_v7(),
            email: "operator@example.test".to_string(),
            data_key_ciphertext: "test-key".to_string(),
        }
    }

    fn sample_event() -> AuditLogEvent {
        serde_json::from_value(serde_json::json!({
            "id": Uuid::now_v7(),
            "workspaceId": Uuid::now_v7(),
            "workListId": Uuid::now_v7(),
            "taskId": Uuid::now_v7(),
            "commentId": null,
            "entityType": "task",
            "entityId": Uuid::now_v7(),
            "action": "updated",
            "scopeLevel": "task",
            "actorUserId": Uuid::now_v7(),
            "actorUserName": ACTOR_NAME_CANARY,
            "actorMembershipId": Uuid::now_v7(),
            "actorAgentId": Uuid::now_v7(),
            "actorAgentHandle": ACTOR_AGENT_HANDLE_CANARY,
            "actorAgentDisplayName": ACTOR_AGENT_NAME_CANARY,
            "actorType": "user",
            "sourceKind": "api",
            "targetVersion": 3,
            "clientVersion": "AUDIT-CLIENT-VERSION-CANARY",
            "occurredAt": Utc::now(),
            "changes": [{
                "fieldKey": "title",
                "changeKind": "updated",
                "beforeValue": {"type": "string", "value": SCALAR_CANARY},
                "afterValue": {"type": "integer", "value": 2},
                "beforeCiphertextDigest": "AUDIT-DIGEST-CANARY",
                "afterCiphertextDigest": null
            }],
            "payload": {
                "payloadCiphertext": PAYLOAD_CANARY,
                "payloadHmac": PAYLOAD_CANARY,
                "payloadVersion": 1,
                "fieldDescriptors": []
            }
        }))
        .expect("sample audit event")
    }

    #[tokio::test]
    async fn test_should_reject_invalid_limits_before_authentication_or_network_io() {
        let mut client = PublicApiClient::new("http://127.0.0.1:1").expect("API client");

        for limit in [0, MAX_AUDIT_LOG_PAGE_ITEMS + 1] {
            let error = client
                .get_my_activity(None, limit)
                .await
                .expect_err("invalid limit");
            assert_eq!(error.code(), "validation");
            assert!(error.to_string().contains("between 1 and 100"));
        }
    }

    #[test]
    fn test_should_omit_encrypted_payload_and_redact_sensitive_debug_values() {
        let event = sample_event();
        assert!(event.payload_present);
        assert_eq!(
            event.actor_agent_handle.as_deref(),
            Some(ACTOR_AGENT_HANDLE_CANARY)
        );
        assert_eq!(
            event.actor_agent_display_name.as_deref(),
            Some(ACTOR_AGENT_NAME_CANARY)
        );
        let mut encoded = serde_json::to_value(&event).expect("encode audit event");
        let debug = format!("{event:?}");

        assert!(encoded.get("payload").is_none());
        assert_eq!(
            encoded.get("payloadPresent"),
            Some(&serde_json::Value::Bool(true))
        );
        for canary in [
            ACTOR_NAME_CANARY,
            ACTOR_AGENT_HANDLE_CANARY,
            ACTOR_AGENT_NAME_CANARY,
            SCALAR_CANARY,
            PAYLOAD_CANARY,
            "AUDIT-CLIENT-VERSION-CANARY",
            "AUDIT-DIGEST-CANARY",
        ] {
            assert!(!debug.contains(canary), "Debug leaked {canary}");
        }
        assert!(debug.contains("<redacted>"));

        let object = encoded.as_object_mut().expect("audit event object");
        object.remove("payloadPresent");
        object.insert("payload".to_string(), serde_json::Value::Null);
        let without_payload: AuditLogEvent =
            serde_json::from_value(encoded.clone()).expect("null payload");
        assert!(!without_payload.payload_present);

        encoded
            .as_object_mut()
            .expect("audit event object")
            .remove("payload");
        let missing_payload: AuditLogEvent =
            serde_json::from_value(encoded).expect("missing payload");
        assert!(!missing_payload.payload_present);
    }

    #[test]
    fn test_should_reject_oversized_page_and_nested_collections() {
        let event = sample_event();
        let page = AuditLogPage {
            events: vec![event.clone(); 2],
            next_cursor: None,
        };
        let error = validate_audit_page(&page, 1).expect_err("page larger than requested");
        assert_eq!(
            error.response_failure_kind(),
            Some(ResponseFailureKind::JsonSchema)
        );

        let mut event = event;
        event.changes = vec![event.changes[0].clone(); MAX_AUDIT_LOG_CHANGES_PER_EVENT + 1];
        let error = validate_audit_page(
            &AuditLogPage {
                events: vec![event],
                next_cursor: None,
            },
            1,
        )
        .expect_err("too many changes");
        assert_eq!(
            error.response_failure_kind(),
            Some(ResponseFailureKind::JsonSchema)
        );

        let mut event = sample_event();
        event.changes[0].after_value = Some(AuditLogValue::SectionsSync {
            applied_snapshots_count: MAX_AUDIT_LOG_SECTIONS_PER_CHANGE + 1,
            removed_sections_count: 0,
            applied_snapshots: Some(vec![
                AuditSectionSnapshot {
                    id: Uuid::now_v7(),
                    position: 0,
                    auto_archive_enabled: false,
                    auto_archive_after_days: None,
                };
                MAX_AUDIT_LOG_SECTIONS_PER_CHANGE + 1
            ]),
            removed_section_ids: None,
        });
        let error = validate_audit_page(
            &AuditLogPage {
                events: vec![event],
                next_cursor: None,
            },
            1,
        )
        .expect_err("too many sections");
        assert_eq!(
            error.response_failure_kind(),
            Some(ResponseFailureKind::JsonSchema)
        );
    }

    #[tokio::test]
    async fn test_should_request_typed_bounded_activity_and_project_pages() {
        let cursor = Uuid::now_v7();
        let work_list_id = Uuid::now_v7();
        let response_body = serde_json::to_vec(&serde_json::json!({
            "events": [sample_event()],
            "nextCursor": cursor
        }))
        .expect("response JSON");
        let (api_url, server) = serve_responses(vec![response_body.clone(), response_body]).await;
        let mut client = PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
            .expect("API client");

        let activity = client
            .get_my_activity(Some(cursor), 17)
            .await
            .expect("activity");
        let project = client
            .get_work_list_audit_log(work_list_id, None, 25)
            .await
            .expect("project audit log");

        assert_eq!(activity.events.len(), 1);
        assert_eq!(activity.next_cursor, Some(cursor));
        assert_eq!(project.events.len(), 1);
        let requests = server.await.expect("server");
        assert!(requests[0].starts_with(&format!(
            "GET /me/activity?cursor={cursor}&limit=17 HTTP/1.1"
        )));
        assert!(requests[1].starts_with(&format!(
            "GET /work-lists/{work_list_id}/audit-log?limit=25 HTTP/1.1"
        )));
        assert!(requests.iter().all(|request| {
            request
                .to_ascii_lowercase()
                .contains("authorization: bearer test-access")
        }));
    }

    async fn serve_responses(
        responses: Vec<Vec<u8>>,
    ) -> (String, tokio::task::JoinHandle<Vec<String>>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let server = tokio::spawn(async move {
            let mut requests = Vec::new();
            for body in responses {
                let (mut stream, _) = listener.accept().await.expect("connection");
                let request = read_request_headers(&mut stream).await;
                let headers = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                stream
                    .write_all(headers.as_bytes())
                    .await
                    .expect("response headers");
                stream.write_all(&body).await.expect("response body");
                requests.push(request);
            }
            requests
        });
        (api_url, server)
    }

    async fn read_request_headers(stream: &mut tokio::net::TcpStream) -> String {
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1024];
        while !request.windows(4).any(|window| window == b"\r\n\r\n") {
            let read = stream.read(&mut buffer).await.expect("request");
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
        }
        String::from_utf8(request).expect("UTF-8 request")
    }
}
