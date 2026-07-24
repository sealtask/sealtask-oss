use chrono::{DateTime, Utc};
use sealtask_client_crypto::{ChecklistItemPayload, TaskPayloadRichText};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMembership {
    pub id: Uuid,
    pub user_id: Uuid,
    pub user_email: String,
    pub user_name: String,
    pub user_avatar_color: String,
    pub role: String,
    pub status: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentWorkListSummary {
    pub id: Uuid,
    pub owner_user_id: Uuid,
    pub workspace_id: Uuid,
    pub timezone: String,
    pub section_snapshots: Vec<sealtask_client_api::SectionSnapshotPayload>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
    pub membership: AgentMembership,
    pub title: Option<String>,
    pub description: Option<String>,
    pub payload: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_error: Option<ReadError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentWorkListDetail {
    #[serde(flatten)]
    pub work_list: AgentWorkListSummary,
    pub members: Vec<AgentMembership>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentDelegation {
    pub id: Uuid,
    pub task_id: Uuid,
    pub membership_id: Uuid,
    pub role: String,
    pub status: String,
    pub note_present: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentAttachment {
    pub id: Uuid,
    pub file_name: String,
    pub content_type: String,
    pub size_bytes: u64,
    #[serde(skip_serializing, skip_deserializing, default)]
    pub(crate) blob_key: Vec<u8>,
}

impl fmt::Debug for AgentAttachment {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AgentAttachment")
            .field("id", &self.id)
            .field("file_name", &Redacted)
            .field("content_type", &self.content_type)
            .field("size_bytes", &self.size_bytes)
            .field("blob_key", &Redacted)
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTaskSummary {
    pub id: Uuid,
    pub work_list_id: Uuid,
    pub work_list_title: Option<String>,
    pub created_by_membership_id: Uuid,
    pub section_id: Option<Uuid>,
    pub priority: Option<i8>,
    pub position: Option<String>,
    pub due_at: Option<DateTime<Utc>>,
    pub start_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub archived_at: Option<DateTime<Utc>>,
    pub is_completed: bool,
    pub recurrence_id: Option<Uuid>,
    pub recurrence_schedule: Option<String>,
    pub recurrence_iteration: Option<i64>,
    pub materialized_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub comment_count: i64,
    pub title: Option<String>,
    pub body_markdown: Option<String>,
    pub body_rich_text: Option<TaskPayloadRichText>,
    pub checklist: Option<Vec<ChecklistItemPayload>>,
    pub attachments: Option<Vec<AgentAttachment>>,
    pub references: Option<Vec<Value>>,
    pub mentions: Option<Vec<String>>,
    pub client_meta: Option<Value>,
    pub recurrence_state: Option<Value>,
    pub delegations: Vec<AgentDelegation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_error: Option<ReadError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentComment {
    pub id: Uuid,
    pub task_id: Uuid,
    pub author_membership_id: Uuid,
    pub body_markdown: Option<String>,
    pub content: Option<TaskPayloadRichText>,
    pub mentions: Option<Vec<String>>,
    pub attachments: Option<Vec<AgentAttachment>>,
    pub client_meta: Option<Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_error: Option<ReadError>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentNote {
    pub id: Uuid,
    pub work_list_id: Uuid,
    pub created_by_membership_id: Uuid,
    pub is_private: bool,
    pub title: Option<String>,
    pub body_markdown: Option<String>,
    pub content: Option<TaskPayloadRichText>,
    pub mentions: Option<Vec<String>>,
    pub attachments: Option<Vec<AgentAttachment>>,
    pub client_meta: Option<Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_error: Option<ReadError>,
}

impl fmt::Debug for AgentNote {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AgentNote")
            .field("id", &self.id)
            .field("work_list_id", &self.work_list_id)
            .field("created_by_membership_id", &self.created_by_membership_id)
            .field("is_private", &self.is_private)
            .field("title_present", &self.title.is_some())
            .field("body_markdown_present", &self.body_markdown.is_some())
            .field("content_present", &self.content.is_some())
            .field("mention_count", &self.mentions.as_ref().map_or(0, Vec::len))
            .field(
                "attachment_count",
                &self.attachments.as_ref().map_or(0, Vec::len),
            )
            .field("client_meta_present", &self.client_meta.is_some())
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .field("read_error_present", &self.read_error.is_some())
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTaskDetail {
    #[serde(flatten)]
    pub task: AgentTaskSummary,
    pub comments: Vec<AgentComment>,
}

#[derive(Clone)]
pub struct DownloadedAttachment {
    pub attachment: AgentAttachment,
    pub bytes: Vec<u8>,
}

impl fmt::Debug for DownloadedAttachment {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DownloadedAttachment")
            .field("attachment", &self.attachment)
            .field("bytes", &Redacted)
            .field("byte_len", &self.bytes.len())
            .finish()
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadableAttachmentContentFormat {
    Text,
    Markdown,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadableAttachmentSourceKind {
    PlainText,
    DocxRendered,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadableAttachment {
    pub attachment: AgentAttachment,
    pub text: String,
    pub content_format: ReadableAttachmentContentFormat,
    pub source_kind: ReadableAttachmentSourceKind,
}

impl fmt::Debug for ReadableAttachment {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ReadableAttachment")
            .field("attachment", &self.attachment)
            .field("text", &Redacted)
            .field("text_len", &self.text.len())
            .field("content_format", &self.content_format)
            .field("source_kind", &self.source_kind)
            .finish()
    }
}

struct Redacted;

impl fmt::Debug for Redacted {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("<redacted>")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sealtask_client_crypto::RichTextBlock;

    #[test]
    fn attachment_debug_redacts_key_binary_plaintext_and_text_plaintext() {
        const FILE_NAME: &str = "attachment-file-name-debug-canary.txt";
        const TEXT: &str = "attachment-readable-text-debug-canary";
        let blob_key = vec![222, 173, 190, 239, 17];
        let downloaded_bytes = vec![193, 250, 206, 88, 99, 41];
        let raw_blob_key_debug = format!("{blob_key:?}");
        let raw_downloaded_bytes_debug = format!("{downloaded_bytes:?}");
        let attachment = AgentAttachment {
            id: Uuid::now_v7(),
            file_name: FILE_NAME.to_string(),
            content_type: "text/plain".to_string(),
            size_bytes: downloaded_bytes.len() as u64,
            blob_key,
        };
        let downloaded = DownloadedAttachment {
            attachment: attachment.clone(),
            bytes: downloaded_bytes,
        };
        let readable = ReadableAttachment {
            attachment: attachment.clone(),
            text: TEXT.to_string(),
            content_format: ReadableAttachmentContentFormat::Text,
            source_kind: ReadableAttachmentSourceKind::PlainText,
        };

        let attachment_debug = format!("{attachment:?}");
        assert!(!attachment_debug.contains(FILE_NAME));
        assert!(!attachment_debug.contains(&raw_blob_key_debug));
        assert!(attachment_debug.contains("<redacted>"));

        let downloaded_debug = format!("{downloaded:?}");
        assert!(!downloaded_debug.contains(FILE_NAME));
        assert!(!downloaded_debug.contains(&raw_blob_key_debug));
        assert!(!downloaded_debug.contains(&raw_downloaded_bytes_debug));
        assert!(downloaded_debug.contains("byte_len: 6"));
        assert!(downloaded_debug.contains("<redacted>"));

        let readable_debug = format!("{readable:?}");
        assert!(!readable_debug.contains(FILE_NAME));
        assert!(!readable_debug.contains(&raw_blob_key_debug));
        assert!(!readable_debug.contains(TEXT));
        assert!(readable_debug.contains(&format!("text_len: {}", TEXT.len())));
        assert!(readable_debug.contains("<redacted>"));
    }

    #[test]
    fn agent_note_debug_redacts_every_plaintext_bearing_field() {
        let note = AgentNote {
            id: Uuid::now_v7(),
            work_list_id: Uuid::now_v7(),
            created_by_membership_id: Uuid::now_v7(),
            is_private: true,
            title: Some("agent-note-title-canary".to_string()),
            body_markdown: Some("agent-note-body-canary".to_string()),
            content: Some(TaskPayloadRichText {
                format: "agent-note-format-canary".to_string(),
                version: 1,
                blocks: vec![RichTextBlock {
                    block_type: "agent-note-block-type-canary".to_string(),
                    text: "agent-note-block-text-canary".to_string(),
                }],
            }),
            mentions: Some(vec!["agent-note-mention-canary".to_string()]),
            attachments: Some(vec![AgentAttachment {
                id: Uuid::now_v7(),
                file_name: "agent-note-file-canary.txt".to_string(),
                content_type: "agent-note-content-type-canary".to_string(),
                size_bytes: 7,
                blob_key: b"agent-note-blob-key-canary".to_vec(),
            }]),
            client_meta: Some(serde_json::json!({"secret": "agent-note-meta-canary"})),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            read_error: Some(ReadError {
                code: "agent-note-read-code-canary".to_string(),
                message: "agent-note-read-message-canary".to_string(),
            }),
        };

        let debug = format!("{note:?}");

        for canary in [
            "agent-note-title-canary",
            "agent-note-body-canary",
            "agent-note-format-canary",
            "agent-note-block-type-canary",
            "agent-note-block-text-canary",
            "agent-note-mention-canary",
            "agent-note-file-canary",
            "agent-note-content-type-canary",
            "agent-note-blob-key-canary",
            "agent-note-meta-canary",
            "agent-note-read-code-canary",
            "agent-note-read-message-canary",
        ] {
            assert!(!debug.contains(canary), "Debug leaked canary {canary}");
        }
        assert!(debug.contains("title_present: true"));
        assert!(debug.contains("body_markdown_present: true"));
        assert!(debug.contains("content_present: true"));
        assert!(debug.contains("mention_count: 1"));
        assert!(debug.contains("attachment_count: 1"));
        assert!(debug.contains("read_error_present: true"));
    }
}
