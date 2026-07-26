use std::{collections::VecDeque, fmt, time::Duration};

use chrono::{DateTime, Utc};
use futures_util::{StreamExt, stream::BoxStream};
use reqwest::header::{ACCEPT, CONTENT_TYPE};
use sealtask_client_core::{PublicError, PublicResult, ResponseFailureKind, TransportFailureKind};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::transport::{
    PublicApiClient, build_control_plane_stream_http_client, parse_retry_after,
};

pub const EVENT_STREAM_IDLE_TIMEOUT: Duration = Duration::from_secs(35);
pub const MAX_EVENT_STREAM_EVENT_BYTES: usize = 64 * 1024;

const MAX_EVENT_STREAM_PENDING_EVENTS: usize = 256;
const MAX_SSE_TOKEN_BYTES: usize = 4 * 1024;
const MAX_SSE_TOKEN_RESPONSE_BYTES: usize = 16 * 1024;

/// A short-lived credential scoped to one project event stream.
///
/// The credential has no public accessor and its `Debug` implementation is
/// deliberately redacted. Callers should request a fresh value for each connection.
pub struct SseToken {
    token: Zeroizing<String>,
    expires_in: u64,
}

impl SseToken {
    #[must_use]
    pub const fn expires_in(&self) -> u64 {
        self.expires_in
    }

    fn validate(&self) -> PublicResult<()> {
        if self.token.is_empty() || self.token.len() > MAX_SSE_TOKEN_BYTES {
            return Err(PublicError::response(
                ResponseFailureKind::JsonSchema,
                "API SSE token response does not match the expected schema",
            ));
        }
        if self.expires_in == 0 {
            return Err(PublicError::response(
                ResponseFailureKind::JsonSchema,
                "API SSE token response contains an invalid lifetime",
            ));
        }
        Ok(())
    }
}

impl fmt::Debug for SseToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SseToken")
            .field("token", &"<redacted>")
            .field("expires_in", &self.expires_in)
            .finish()
    }
}

impl<'de> Deserialize<'de> for SseToken {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let response = SseTokenResponse::deserialize(deserializer)?;
        Ok(Self {
            token: Zeroizing::new(response.token),
            expires_in: response.expires_in,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum BoardEvent {
    TaskCreated {
        #[serde(rename = "taskId")]
        task_id: Uuid,
        #[serde(rename = "sectionId")]
        section_id: Option<Uuid>,
    },
    TaskUpdated {
        #[serde(rename = "taskId")]
        task_id: Uuid,
        #[serde(default)]
        fields: Vec<String>,
    },
    TaskDeleted {
        #[serde(rename = "taskId")]
        task_id: Uuid,
    },
    TaskMoved {
        #[serde(rename = "taskId")]
        task_id: Uuid,
        #[serde(rename = "fromSectionId")]
        from_section_id: Option<Uuid>,
        #[serde(rename = "toSectionId")]
        to_section_id: Option<Uuid>,
    },
    TaskArchived {
        #[serde(rename = "taskId")]
        task_id: Uuid,
        #[serde(rename = "sectionId")]
        section_id: Option<Uuid>,
    },
    TaskUnarchived {
        #[serde(rename = "taskId")]
        task_id: Uuid,
        #[serde(rename = "sectionId")]
        section_id: Option<Uuid>,
    },
    SectionsSynced {
        #[serde(rename = "sectionIds")]
        section_ids: Vec<Uuid>,
    },
    SectionSorted {
        #[serde(rename = "sectionId")]
        section_id: Option<Uuid>,
        #[serde(rename = "taskIds")]
        task_ids: Vec<Uuid>,
    },
    CommentCreated {
        #[serde(rename = "taskId")]
        task_id: Uuid,
        #[serde(rename = "commentId")]
        comment_id: Uuid,
    },
    CommentUpdated {
        #[serde(rename = "taskId")]
        task_id: Uuid,
        #[serde(rename = "commentId")]
        comment_id: Uuid,
    },
    CommentDeleted {
        #[serde(rename = "taskId")]
        task_id: Uuid,
        #[serde(rename = "commentId")]
        comment_id: Uuid,
    },
    MembershipCreated {
        #[serde(rename = "membershipId")]
        membership_id: Uuid,
    },
    MembershipUpdated {
        #[serde(rename = "membershipId")]
        membership_id: Uuid,
    },
    MembershipDeleted {
        #[serde(rename = "membershipId")]
        membership_id: Uuid,
    },
    DelegationUpserted {
        #[serde(rename = "taskId")]
        task_id: Uuid,
        #[serde(rename = "delegationId")]
        delegation_id: Uuid,
    },
    DelegationDeleted {
        #[serde(rename = "taskId")]
        task_id: Uuid,
        #[serde(rename = "delegationId")]
        delegation_id: Uuid,
    },
    RecurrenceCreated {
        #[serde(rename = "recurrenceId")]
        recurrence_id: Uuid,
    },
    RecurrenceUpdated {
        #[serde(rename = "recurrenceId")]
        recurrence_id: Uuid,
    },
    RecurrenceDeleted {
        #[serde(rename = "recurrenceId")]
        recurrence_id: Uuid,
    },
    WorkListUpdated {
        #[serde(default)]
        fields: Vec<String>,
    },
    NoteCreated {
        #[serde(rename = "noteId")]
        note_id: Uuid,
    },
    NoteUpdated {
        #[serde(rename = "noteId")]
        note_id: Uuid,
    },
    NoteDeleted {
        #[serde(rename = "noteId")]
        note_id: Uuid,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardEventEnvelope {
    pub event_id: Uuid,
    pub work_list_id: Uuid,
    pub actor_user_id: Option<Uuid>,
    pub actor_membership_id: Option<Uuid>,
    pub actor_instance_id: Option<String>,
    pub occurred_at: DateTime<Utc>,
    #[serde(flatten)]
    pub event: BoardEvent,
}

impl fmt::Debug for BoardEventEnvelope {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoardEventEnvelope")
            .field("event_id", &self.event_id)
            .field("work_list_id", &self.work_list_id)
            .field("actor_user_id", &self.actor_user_id)
            .field("actor_membership_id", &self.actor_membership_id)
            .field(
                "actor_instance_id",
                &self.actor_instance_id.as_ref().map(|_| "<redacted>"),
            )
            .field("occurred_at", &self.occurred_at)
            .field("event", &self.event)
            .finish()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BoardStreamEvent {
    Board(BoardEventEnvelope),
    Resync { missed_events: u64 },
}

/// A connected project event feed.
///
/// The response is intentionally kept private and this type's `Debug`
/// implementation never renders the request URL, which contains the stream token.
pub struct BoardEventStream {
    body: BoxStream<'static, PublicResult<Vec<u8>>>,
    parser: SseParser,
    finished: bool,
}

impl BoardEventStream {
    pub async fn next_event(&mut self) -> Option<PublicResult<BoardStreamEvent>> {
        loop {
            if let Some(event) = self.parser.pop_event() {
                return Some(Ok(event));
            }
            if self.finished {
                return None;
            }

            match self.body.next().await {
                Some(Ok(chunk)) => {
                    if let Err(error) = self.parser.push(&chunk) {
                        self.finished = true;
                        return Some(Err(error));
                    }
                }
                None => {
                    self.finished = true;
                    if let Err(error) = self.parser.finish() {
                        return Some(Err(error));
                    }
                }
                Some(Err(error)) => {
                    self.finished = true;
                    return Some(Err(error));
                }
            }
        }
    }
}

impl fmt::Debug for BoardEventStream {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoardEventStream")
            .field("finished", &self.finished)
            .field("pending_events", &self.parser.pending.len())
            .finish_non_exhaustive()
    }
}

impl PublicApiClient {
    pub async fn issue_project_sse_token(&mut self, work_list_id: Uuid) -> PublicResult<SseToken> {
        let token: SseToken = self
            .post_bounded(
                &format!("/work-lists/{work_list_id}/sse-token"),
                &IssueSseTokenRequest {},
                MAX_SSE_TOKEN_RESPONSE_BYTES,
            )
            .await?;
        token.validate()?;
        Ok(token)
    }

    pub async fn connect_project_events(
        &self,
        work_list_id: Uuid,
        token: &SseToken,
    ) -> PublicResult<BoardEventStream> {
        token.validate()?;

        let endpoint = format!("{}/work-lists/{work_list_id}/events", self.base_url());
        let mut url = reqwest::Url::parse(&endpoint)
            .map_err(|_| PublicError::validation("API event stream URL is invalid"))?;
        url.query_pairs_mut().append_pair("token", &token.token);

        let client = build_control_plane_stream_http_client(
            self.transport_options(),
            EVENT_STREAM_IDLE_TIMEOUT,
        )?;
        let response = client
            .get(url)
            .header(ACCEPT, "text/event-stream")
            .send()
            .await
            .map_err(|error| map_stream_transport_error(&error))?;
        let status = response.status();
        if !status.is_success() {
            let retry_after = parse_retry_after(response.headers());
            return Err(PublicError::http(status.as_u16(), None, retry_after));
        }
        if !is_event_stream_response(response.headers()) {
            return Err(PublicError::response(
                ResponseFailureKind::BodyRead,
                "API event stream returned an unexpected content type",
            ));
        }

        // Consume the response immediately after validating the handshake. `reqwest::Response`
        // retains its request URL, so keeping it for the lifetime of the stream would also retain
        // the query-string credential.
        let body = response
            .bytes_stream()
            .map(|chunk| {
                chunk
                    .map(|bytes| bytes.to_vec())
                    .map_err(|error| map_stream_transport_error(&error))
            })
            .boxed();

        Ok(BoardEventStream {
            body,
            parser: SseParser::for_work_list(work_list_id),
            finished: false,
        })
    }
}

#[derive(Serialize)]
struct IssueSseTokenRequest {}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SseTokenResponse {
    token: String,
    expires_in: u64,
}

fn is_event_stream_response(headers: &reqwest::header::HeaderMap) -> bool {
    headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("text/event-stream"))
}

fn map_stream_transport_error(error: &reqwest::Error) -> PublicError {
    let kind = if error.is_timeout() {
        TransportFailureKind::Timeout
    } else if error.is_connect() {
        TransportFailureKind::Connect
    } else if error.is_body() {
        TransportFailureKind::Body
    } else {
        TransportFailureKind::Other
    };
    PublicError::transport(kind)
}

struct SseParser {
    line: Vec<u8>,
    pending_cr: bool,
    at_stream_start: bool,
    expected_work_list_id: Option<Uuid>,
    event_name: String,
    data: Vec<u8>,
    pending: VecDeque<BoardStreamEvent>,
}

impl Default for SseParser {
    fn default() -> Self {
        Self {
            line: Vec::new(),
            pending_cr: false,
            at_stream_start: true,
            expected_work_list_id: None,
            event_name: String::new(),
            data: Vec::new(),
            pending: VecDeque::new(),
        }
    }
}

impl SseParser {
    fn for_work_list(work_list_id: Uuid) -> Self {
        Self {
            expected_work_list_id: Some(work_list_id),
            ..Self::default()
        }
    }

    fn push(&mut self, chunk: &[u8]) -> PublicResult<()> {
        for &byte in chunk {
            if self.pending_cr {
                self.pending_cr = false;
                self.process_buffered_line()?;
                if byte == b'\n' {
                    continue;
                }
            }

            if byte == b'\r' {
                self.pending_cr = true;
                continue;
            }
            if byte == b'\n' {
                self.process_buffered_line()?;
                continue;
            }

            let buffered = self
                .line
                .len()
                .checked_add(self.event_name.len())
                .and_then(|size| size.checked_add(self.data.len()))
                .and_then(|size| size.checked_add(1))
                .ok_or_else(event_too_large)?;
            if buffered > MAX_EVENT_STREAM_EVENT_BYTES {
                return Err(event_too_large());
            }
            self.line.push(byte);
        }
        Ok(())
    }

    fn finish(&mut self) -> PublicResult<()> {
        if self.pending_cr {
            self.pending_cr = false;
            self.process_buffered_line()?;
        } else if !self.line.is_empty() {
            self.process_buffered_line()?;
        }
        if !self.event_name.is_empty() || !self.data.is_empty() {
            self.dispatch()?;
        }
        Ok(())
    }

    fn process_buffered_line(&mut self) -> PublicResult<()> {
        let line = std::mem::take(&mut self.line);
        self.process_line(&line)
    }

    fn process_line(&mut self, line: &[u8]) -> PublicResult<()> {
        let line = std::str::from_utf8(line).map_err(|_| {
            PublicError::response(
                ResponseFailureKind::JsonMalformed,
                "API event stream contains invalid UTF-8",
            )
        })?;
        let line = if self.at_stream_start {
            self.at_stream_start = false;
            line.strip_prefix('\u{feff}').unwrap_or(line)
        } else {
            line
        };
        if line.is_empty() {
            if !self.event_name.is_empty() || !self.data.is_empty() {
                self.dispatch()?;
            }
            return Ok(());
        }
        if line.starts_with(':') {
            return Ok(());
        }

        let (field, raw_value) = line.split_once(':').unwrap_or((line, ""));
        let value = raw_value.strip_prefix(' ').unwrap_or(raw_value);
        match field {
            "event" => {
                if value
                    .len()
                    .checked_add(self.data.len())
                    .is_none_or(|size| size > MAX_EVENT_STREAM_EVENT_BYTES)
                {
                    return Err(event_too_large());
                }
                self.event_name.clear();
                self.event_name.push_str(value);
            }
            "data" => {
                let new_len = self
                    .data
                    .len()
                    .checked_add(value.len())
                    .and_then(|size| size.checked_add(1))
                    .ok_or_else(event_too_large)?;
                if new_len
                    .checked_add(self.event_name.len())
                    .is_none_or(|size| size > MAX_EVENT_STREAM_EVENT_BYTES)
                {
                    return Err(event_too_large());
                }
                self.data.extend_from_slice(value.as_bytes());
                self.data.push(b'\n');
            }
            _ => {}
        }
        Ok(())
    }

    fn dispatch(&mut self) -> PublicResult<()> {
        if self.pending.len() >= MAX_EVENT_STREAM_PENDING_EVENTS {
            return Err(PublicError::response(
                ResponseFailureKind::BodyTooLarge,
                "API event stream produced too many events in one network frame",
            ));
        }
        if self.data.last() == Some(&b'\n') {
            self.data.pop();
        }
        if self.data.is_empty() && matches!(self.event_name.as_str(), "board_event" | "resync") {
            self.reset_event();
            return Err(PublicError::response(
                ResponseFailureKind::JsonMalformed,
                "API event stream event is missing data",
            ));
        }
        if self.data.is_empty() {
            self.reset_event();
            return Ok(());
        }

        let event = match self.event_name.as_str() {
            "board_event" => {
                let envelope: BoardEventEnvelope = decode_event_json(&self.data)?;
                if self
                    .expected_work_list_id
                    .is_some_and(|expected| envelope.work_list_id != expected)
                {
                    self.reset_event();
                    return Err(PublicError::response(
                        ResponseFailureKind::JsonSchema,
                        "API event stream board identifier does not match the subscription",
                    ));
                }
                BoardStreamEvent::Board(envelope)
            }
            "resync" => {
                let payload: ResyncPayload = decode_event_json(&self.data)?;
                BoardStreamEvent::Resync {
                    missed_events: payload.missed_events,
                }
            }
            _ => BoardStreamEvent::Resync { missed_events: 0 },
        };
        self.reset_event();
        self.pending.push_back(event);
        Ok(())
    }

    fn pop_event(&mut self) -> Option<BoardStreamEvent> {
        self.pending.pop_front()
    }

    fn reset_event(&mut self) {
        self.event_name.clear();
        self.data.clear();
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResyncPayload {
    missed_events: u64,
}

fn decode_event_json<T: for<'de> Deserialize<'de>>(data: &[u8]) -> PublicResult<T> {
    serde_json::from_slice(data).map_err(|error| {
        let (kind, message) = match error.classify() {
            serde_json::error::Category::Data => (
                ResponseFailureKind::JsonSchema,
                "API event stream JSON does not match the expected schema",
            ),
            serde_json::error::Category::Eof | serde_json::error::Category::Syntax => (
                ResponseFailureKind::JsonMalformed,
                "API event stream contains malformed JSON",
            ),
            serde_json::error::Category::Io => (
                ResponseFailureKind::BodyRead,
                "API event stream data could not be read",
            ),
        };
        PublicError::response(kind, message)
    })
}

fn event_too_large() -> PublicError {
    PublicError::response(
        ResponseFailureKind::BodyTooLarge,
        format!(
            "API event stream event exceeds the {MAX_EVENT_STREAM_EVENT_BYTES}-byte safety limit"
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    use chrono::Duration as ChronoDuration;
    use sealtask_client_auth::Credentials;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    const TOKEN_CANARY: &str = "SSE_TOKEN_CANARY_31f58ee3";

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

    fn token() -> SseToken {
        SseToken {
            token: Zeroizing::new(TOKEN_CANARY.to_string()),
            expires_in: 120,
        }
    }

    fn board_event_json(
        event_id: Uuid,
        work_list_id: Uuid,
        task_id: Uuid,
        actor_instance_id: &str,
    ) -> String {
        serde_json::json!({
            "eventId": event_id,
            "workListId": work_list_id,
            "actorInstanceId": actor_instance_id,
            "occurredAt": Utc::now(),
            "type": "task_created",
            "taskId": task_id
        })
        .to_string()
    }

    #[test]
    fn test_should_redact_token_and_actor_instance_debug_output() {
        let token = token();
        let event: BoardEventEnvelope = serde_json::from_str(&board_event_json(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            TOKEN_CANARY,
        ))
        .expect("board event");

        assert_eq!(token.expires_in(), 120);
        assert!(std::mem::needs_drop::<SseToken>());
        assert!(!format!("{token:?}").contains(TOKEN_CANARY));
        assert!(!format!("{event:?}").contains(TOKEN_CANARY));
        assert!(format!("{token:?}").contains("<redacted>"));
        assert!(format!("{event:?}").contains("<redacted>"));
    }

    #[test]
    fn test_should_parse_arbitrary_utf8_chunks_crlf_comments_and_multiline_data() {
        let event_id = Uuid::now_v7();
        let work_list_id = Uuid::now_v7();
        let task_id = Uuid::now_v7();
        let board_json = board_event_json(event_id, work_list_id, task_id, "žluťoučký");
        let wire = format!(
            ":ping\r\nevent: board_event\r\ndata: {board_json}\r\n\r\n\
             : another keepalive\r\nevent: resync\r\ndata: {{\"missedEvents\":\r\ndata: 7}}\r\n\r\n"
        );
        let mut parser = SseParser::default();

        for byte in wire.as_bytes() {
            parser.push(std::slice::from_ref(byte)).expect("SSE byte");
        }

        let board = parser.pop_event().expect("board event");
        let BoardStreamEvent::Board(board) = board else {
            panic!("expected board event");
        };
        assert_eq!(board.event_id, event_id);
        assert_eq!(board.work_list_id, work_list_id);
        assert_eq!(
            board.event,
            BoardEvent::TaskCreated {
                task_id,
                section_id: None
            }
        );
        assert_eq!(
            parser.pop_event(),
            Some(BoardStreamEvent::Resync { missed_events: 7 })
        );
        assert!(parser.pop_event().is_none());
    }

    #[test]
    fn test_should_parse_initial_utf8_bom_and_bare_cr_across_chunk_boundaries() {
        let wire = b"\xef\xbb\xbf:ping\revent: resync\rdata: {\"missedEvents\":9}\r\r";
        let mut parser = SseParser::default();

        for byte in wire {
            parser.push(std::slice::from_ref(byte)).expect("SSE byte");
        }
        parser.finish().expect("finish stream");

        assert_eq!(
            parser.pop_event(),
            Some(BoardStreamEvent::Resync { missed_events: 9 })
        );
        assert!(parser.pop_event().is_none());
    }

    #[test]
    fn test_should_preserve_envelope_and_refetch_for_unknown_inner_board_event() {
        let event_id = Uuid::now_v7();
        let work_list_id = Uuid::now_v7();
        let actor_user_id = Uuid::now_v7();
        let occurred_at = Utc::now();
        let unknown_payload_canary = "UNKNOWN-BOARD-PAYLOAD-CANARY";
        let data = serde_json::json!({
            "eventId": event_id,
            "workListId": work_list_id,
            "actorUserId": actor_user_id,
            "occurredAt": occurred_at,
            "type": "future_board_event",
            "futurePayload": unknown_payload_canary
        });
        let wire = format!("event: board_event\ndata: {data}\n\n");
        let mut parser = SseParser::for_work_list(work_list_id);

        parser.push(wire.as_bytes()).expect("future board event");

        let BoardStreamEvent::Board(envelope) = parser.pop_event().expect("advisory board event")
        else {
            panic!("unknown board event must trigger an authoritative board refetch");
        };
        assert_eq!(envelope.event_id, event_id);
        assert_eq!(envelope.work_list_id, work_list_id);
        assert_eq!(envelope.actor_user_id, Some(actor_user_id));
        assert_eq!(envelope.occurred_at, occurred_at);
        assert_eq!(envelope.event, BoardEvent::Unknown);
        assert!(!format!("{envelope:?}").contains(unknown_payload_canary));
    }

    #[test]
    fn test_should_fail_closed_when_board_event_targets_another_work_list() {
        let expected_work_list_id = Uuid::now_v7();
        let event_json =
            board_event_json(Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7(), TOKEN_CANARY);
        let wire = format!("event: board_event\ndata: {event_json}\n\n");
        let mut parser = SseParser::for_work_list(expected_work_list_id);

        let error = parser
            .push(wire.as_bytes())
            .expect_err("mismatched work list must fail closed");

        assert_eq!(
            error.response_failure_kind(),
            Some(ResponseFailureKind::JsonSchema)
        );
        for rendered in [error.to_string(), format!("{error:?}")] {
            assert!(!rendered.contains(TOKEN_CANARY));
            assert!(!rendered.contains(&expected_work_list_id.to_string()));
        }
        assert!(parser.pop_event().is_none());
    }

    #[test]
    fn test_should_convert_unknown_named_sse_events_to_advisory_resync() {
        let mut parser = SseParser::default();

        parser
            .push(b"event: future_event\ndata: UNKNOWN-TOP-LEVEL-CANARY\n\n")
            .expect("future named event");
        assert_eq!(
            parser.pop_event(),
            Some(BoardStreamEvent::Resync { missed_events: 0 })
        );

        parser
            .push(b"event: empty_future_event\n\n")
            .expect("empty future named event");
        assert!(parser.pop_event().is_none());
    }

    #[test]
    fn test_should_dispatch_a_complete_unterminated_event_at_eof() {
        let mut parser = SseParser::default();
        parser
            .push(b"event: resync\ndata: {\"missedEvents\":2}")
            .expect("partial stream");
        assert!(parser.pop_event().is_none());

        parser.finish().expect("finish stream");

        assert_eq!(
            parser.pop_event(),
            Some(BoardStreamEvent::Resync { missed_events: 2 })
        );
    }

    #[test]
    fn test_should_reject_malformed_missing_and_oversized_events_without_echoing_data() {
        let cases: [(&[u8], ResponseFailureKind); 2] = [
            (
                b"event: board_event\ndata: {TOKEN_CANARY_MALFORMED}\n\n",
                ResponseFailureKind::JsonMalformed,
            ),
            (
                b"event: board_event\n\n",
                ResponseFailureKind::JsonMalformed,
            ),
        ];

        for (wire, expected_kind) in cases {
            let error = SseParser::default()
                .push(wire)
                .expect_err("invalid stream event");
            assert_eq!(error.response_failure_kind(), Some(expected_kind));
            for rendered in [error.to_string(), format!("{error:?}")] {
                assert!(!rendered.contains("TOKEN_CANARY"));
            }
        }

        let error = SseParser::default()
            .push(&vec![b'x'; MAX_EVENT_STREAM_EVENT_BYTES + 1])
            .expect_err("oversized stream event");
        assert_eq!(
            error.response_failure_kind(),
            Some(ResponseFailureKind::BodyTooLarge)
        );
    }

    #[test]
    fn test_should_bound_pending_events_from_one_network_chunk() {
        let event = "event: resync\ndata: {\"missedEvents\":0}\n\n";
        let wire = event.repeat(MAX_EVENT_STREAM_PENDING_EVENTS + 1);
        let error = SseParser::default()
            .push(wire.as_bytes())
            .expect_err("too many pending events");

        assert_eq!(
            error.response_failure_kind(),
            Some(ResponseFailureKind::BodyTooLarge)
        );
    }

    #[tokio::test]
    async fn test_should_issue_token_connect_to_expected_endpoint_and_outlive_request_timeout() {
        let work_list_id = Uuid::now_v7();
        let event_id = Uuid::now_v7();
        let task_id = Uuid::now_v7();
        let event_json = board_event_json(event_id, work_list_id, task_id, TOKEN_CANARY);
        let (api_url, server) =
            serve_token_then_delayed_stream(work_list_id, event_json, Duration::from_millis(90))
                .await;
        let options = crate::ApiTransportOptions::new(
            Duration::from_millis(30),
            Duration::from_millis(30),
            Duration::from_millis(30),
        )
        .expect("transport options");
        let mut client = PublicApiClient::with_credentials_and_options(
            &api_url,
            test_credentials(&api_url),
            options,
        )
        .expect("API client");

        let token = client
            .issue_project_sse_token(work_list_id)
            .await
            .expect("SSE token");
        let mut stream = client
            .connect_project_events(work_list_id, &token)
            .await
            .expect("event stream");
        assert!(!format!("{token:?}").contains(TOKEN_CANARY));
        assert!(!format!("{stream:?}").contains(TOKEN_CANARY));

        let event = stream
            .next_event()
            .await
            .expect("stream item")
            .expect("valid event");
        let BoardStreamEvent::Board(event) = event else {
            panic!("expected board event");
        };
        assert_eq!(event.event_id, event_id);
        assert_eq!(
            event.event,
            BoardEvent::TaskCreated {
                task_id,
                section_id: None
            }
        );
        assert!(!format!("{event:?}").contains(TOKEN_CANARY));
        assert!(stream.next_event().await.is_none());

        let requests = server.await.expect("server");
        assert!(requests[0].starts_with(&format!(
            "POST /work-lists/{work_list_id}/sse-token HTTP/1.1"
        )));
        assert!(
            requests[0]
                .to_ascii_lowercase()
                .contains("authorization: bearer test-access")
        );
        assert!(requests[1].starts_with(&format!(
            "GET /work-lists/{work_list_id}/events?token={TOKEN_CANARY} HTTP/1.1"
        )));
        assert!(
            requests[1]
                .to_ascii_lowercase()
                .contains("accept: text/event-stream")
        );
    }

    #[tokio::test]
    async fn test_should_not_leak_token_bearing_url_or_response_body_on_http_failure() {
        let work_list_id = Uuid::now_v7();
        let (api_url, server) = serve_single_stream_response(
            "401 Unauthorized",
            "application/json",
            format!(r#"{{"error":"{TOKEN_CANARY}"}}"#).into_bytes(),
        )
        .await;
        let client = PublicApiClient::new(&api_url).expect("API client");

        let error = client
            .connect_project_events(work_list_id, &token())
            .await
            .expect_err("HTTP failure");

        assert_eq!(error.http_status(), Some(401));
        assert_eq!(error.backend_error_code(), None);
        for rendered in [error.to_string(), format!("{error:?}")] {
            assert!(!rendered.contains(TOKEN_CANARY));
            assert!(!rendered.contains("/events?token="));
        }
        let request = server.await.expect("server");
        assert!(request.contains(TOKEN_CANARY));
    }

    #[tokio::test]
    async fn test_should_not_leak_token_bearing_url_on_transport_failure() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        drop(listener);
        let client = PublicApiClient::new(api_url).expect("API client");

        let error = client
            .connect_project_events(Uuid::now_v7(), &token())
            .await
            .expect_err("transport failure");

        assert!(error.transport_failure_kind().is_some());
        for rendered in [error.to_string(), format!("{error:?}")] {
            assert!(!rendered.contains(TOKEN_CANARY));
            assert!(!rendered.contains("/events?token="));
        }
    }

    #[tokio::test]
    async fn test_should_reject_wrong_content_type_without_reading_or_echoing_body() {
        let work_list_id = Uuid::now_v7();
        let (api_url, server) = serve_single_stream_response(
            "200 OK",
            "application/json",
            TOKEN_CANARY.as_bytes().to_vec(),
        )
        .await;
        let client = PublicApiClient::new(&api_url).expect("API client");

        let error = client
            .connect_project_events(work_list_id, &token())
            .await
            .expect_err("content type failure");

        assert_eq!(
            error.response_failure_kind(),
            Some(ResponseFailureKind::BodyRead)
        );
        assert!(!error.to_string().contains(TOKEN_CANARY));
        assert!(!format!("{error:?}").contains(TOKEN_CANARY));
        server.await.expect("server");
    }

    async fn serve_token_then_delayed_stream(
        work_list_id: Uuid,
        event_json: String,
        event_delay: Duration,
    ) -> (String, tokio::task::JoinHandle<Vec<String>>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let server = tokio::spawn(async move {
            let mut requests = Vec::new();

            let (mut token_stream, _) = listener.accept().await.expect("token connection");
            requests.push(read_request_headers(&mut token_stream).await);
            let body = format!(r#"{{"token":"{TOKEN_CANARY}","expiresIn":120}}"#).into_bytes();
            write_response(&mut token_stream, "200 OK", "application/json", &body).await;

            let (mut event_stream, _) = listener.accept().await.expect("event connection");
            requests.push(read_request_headers(&mut event_stream).await);
            event_stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream; charset=utf-8\r\nConnection: close\r\n\r\n",
                )
                .await
                .expect("stream response headers");
            tokio::time::sleep(event_delay).await;
            let body = format!(":ping\n\nevent: board_event\ndata: {event_json}\n\n");
            event_stream
                .write_all(body.as_bytes())
                .await
                .expect("stream event");

            assert!(requests[0].contains(&work_list_id.to_string()));
            requests
        });
        (api_url, server)
    }

    async fn serve_single_stream_response(
        status: &'static str,
        content_type: &'static str,
        body: Vec<u8>,
    ) -> (String, tokio::task::JoinHandle<String>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("connection");
            let request = read_request_headers(&mut stream).await;
            write_response(&mut stream, status, content_type, &body).await;
            request
        });
        (api_url, server)
    }

    async fn write_response(
        stream: &mut tokio::net::TcpStream,
        status: &str,
        content_type: &str,
        body: &[u8],
    ) {
        let headers = format!(
            "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream
            .write_all(headers.as_bytes())
            .await
            .expect("response headers");
        stream.write_all(body).await.expect("response body");
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
