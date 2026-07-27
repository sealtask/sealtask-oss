mod crypto;
mod projection;
mod reconciliation;

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use sealtask_client_api::note_transport::EncodedNoteRequest;
use sealtask_client_api::{
    MAX_NOTE_COLLECTION_ITEMS, MAX_NOTE_COLLECTION_PAGES, MAX_NOTE_PAGE_ITEMS,
};
use sealtask_client_core::{PublicError, PublicResult};
use uuid::Uuid;

use crate::inputs::{CreateNoteArgs, DeleteNoteArgs, UpdateNoteArgs};
use crate::models::AgentNote;
use crate::reconciliation::mutation_outcome_is_ambiguous;

use self::crypto::{PreparedNoteCreate, PreparedNoteDelete, PreparedNoteUpdate};
use self::reconciliation::{
    MUTATION_RECONCILIATION_TIMEOUT, ProjectedNotePage, add_received_note_page_to_encoded_budget,
    reconcile_delete_note, validate_next_note_cursor, validate_note_page_size,
};
use super::RuntimeClient;

#[cfg(test)]
use std::time::Duration;

#[cfg(test)]
use sealtask_client_api::{CreateNoteRequest, NoteResponse, PublicApiClient, UpdateNoteRequest};
#[cfg(test)]
use sealtask_client_crypto::{
    FlexibleValue, NotePayloadBody, RichTextBlock, SymmetricKey, TaskPayloadRichText,
    build_note_payload_envelope,
};

#[cfg(test)]
use crate::blocking_crypto::BlockingCryptoAdmission;
#[cfg(test)]
use crate::inputs::{NoteCreateInput, NoteUpdateInput};

#[cfg(test)]
use self::crypto::{
    MAX_NOTE_PLAINTEXT_BYTES, MAX_RICH_TEXT_BLOCK_CHARS, MAX_RICH_TEXT_BLOCKS,
    markdown_note_content, validate_note_plaintext_size,
};
#[cfg(test)]
use self::reconciliation::{
    add_received_note_page_to_encoded_budget_with_limit, find_note_in_bounded_pages,
};
#[cfg(test)]
use super::UnlockedWorkListContext;

impl RuntimeClient {
    pub async fn list_notes(
        &self,
        work_list_id: Uuid,
        password_stdin: bool,
    ) -> PublicResult<Vec<AgentNote>> {
        let (mut client, context) = self
            .load_unlocked_work_list_context(
                work_list_id,
                password_stdin,
                "Password required to decrypt note data.",
            )
            .await?;
        let mut notes = Vec::new();
        let mut cursor = None;
        let mut seen_cursors = HashSet::new();
        let mut encoded_bytes = 0usize;
        let mut payload_permit = self.blocking_crypto.admit_large_payload().await?;
        for _ in 0..MAX_NOTE_COLLECTION_PAGES {
            let response = client
                .list_notes_page_encoded(work_list_id, cursor.as_deref(), MAX_NOTE_PAGE_ITEMS)
                .await?;
            add_received_note_page_to_encoded_budget(&mut encoded_bytes, response.encoded_len())?;
            let runtime = self.clone();
            let page_context = context.clone();
            let (next_permit, page) = self
                .blocking_crypto
                .run_with_large_payload(
                    payload_permit,
                    move || {
                        let page = response.decode()?;
                        validate_note_page_size(page.notes.len())?;
                        let item_count = page.notes.len();
                        let notes = page
                            .notes
                            .into_iter()
                            .map(|note| runtime.project_note(note, &page_context))
                            .collect();
                        Ok(ProjectedNotePage {
                            notes,
                            next_cursor: page.next_cursor,
                            item_count,
                        })
                    },
                    "note page parsing and decryption task failed",
                )
                .await?;
            payload_permit = next_permit;
            if notes
                .len()
                .checked_add(page.item_count)
                .is_none_or(|count| count > MAX_NOTE_COLLECTION_ITEMS)
            {
                return Err(PublicError::unexpected(format!(
                    "note list exceeds the {MAX_NOTE_COLLECTION_ITEMS}-item safety limit"
                )));
            }
            let page_was_empty = page.notes.is_empty();
            notes.extend(page.notes);

            match validate_next_note_cursor(page.next_cursor, page_was_empty, &mut seen_cursors)? {
                Some(next_cursor) => cursor = Some(next_cursor),
                None => return Ok(notes),
            }
        }

        Err(PublicError::unexpected(format!(
            "note list exceeds the {MAX_NOTE_COLLECTION_PAGES}-page safety limit"
        )))
    }

    pub async fn get_note(
        &self,
        work_list_id: Uuid,
        note_id: Uuid,
        password_stdin: bool,
    ) -> PublicResult<AgentNote> {
        let (mut client, context) = self
            .load_unlocked_work_list_context(
                work_list_id,
                password_stdin,
                "Password required to decrypt note data.",
            )
            .await?;
        let payload_permit = self.blocking_crypto.admit_large_payload().await?;
        let response = client.get_note_encoded(work_list_id, note_id).await?;
        let runtime = self.clone();
        let (_, note) = self
            .blocking_crypto
            .run_with_large_payload(
                payload_permit,
                move || {
                    let note = response.decode()?;
                    Ok(runtime.project_note(note, &context))
                },
                "note parsing and decryption task failed",
            )
            .await?;
        Ok(note)
    }

    pub async fn create_note(&self, args: CreateNoteArgs) -> PublicResult<AgentNote> {
        let CreateNoteArgs {
            work_list_id,
            input,
            password_stdin,
        } = args;
        let (mut client, context) = self
            .load_unlocked_work_list_context(
                work_list_id,
                password_stdin,
                "Password required to create encrypted notes.",
            )
            .await?;
        let payload_permit = self.blocking_crypto.admit_large_payload().await?;
        let runtime = self.clone();
        let crypto_context = context.clone();
        let (payload_permit, prepared) = self
            .blocking_crypto
            .run_with_large_payload(
                payload_permit,
                move || {
                    let request = runtime.prepare_create_note_request(input, &crypto_context)?;
                    let encoded = EncodedNoteRequest::encode(&request)?;
                    Ok(PreparedNoteCreate { request, encoded })
                },
                "note creation encryption and encoding task failed",
            )
            .await?;
        match client
            .create_note_encoded(work_list_id, prepared.encoded)
            .await
        {
            Ok(response) => {
                let runtime = self.clone();
                let projection_context = context.clone();
                self.finish_create_note_response_with(
                    &mut client,
                    work_list_id,
                    prepared.request,
                    &context,
                    response,
                    payload_permit,
                    move |response| {
                        response
                            .decode()
                            .map(|note| runtime.project_note(note, &projection_context))
                    },
                )
                .await
            }
            Err(primary) if mutation_outcome_is_ambiguous(&primary) => {
                self.reconcile_create_note(
                    &mut client,
                    work_list_id,
                    prepared.request,
                    &context,
                    primary,
                    false,
                    MUTATION_RECONCILIATION_TIMEOUT,
                    payload_permit,
                )
                .await
            }
            Err(primary) => Err(primary),
        }
    }

    pub async fn update_note(&self, args: UpdateNoteArgs) -> PublicResult<AgentNote> {
        self.update_note_with_expected_revision(args, None).await
    }

    pub async fn update_note_if_unchanged(
        &self,
        args: UpdateNoteArgs,
        expected_updated_at: DateTime<Utc>,
    ) -> PublicResult<AgentNote> {
        self.update_note_with_expected_revision(args, Some(expected_updated_at))
            .await
    }

    async fn update_note_with_expected_revision(
        &self,
        args: UpdateNoteArgs,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> PublicResult<AgentNote> {
        let UpdateNoteArgs {
            work_list_id,
            note_id,
            input,
            password_stdin,
        } = args;
        if input.title.is_none() && input.body.is_none() {
            return Err(PublicError::validation(
                "provide at least one note field to update",
            ));
        }

        let (mut client, context) = self
            .load_unlocked_work_list_context(
                work_list_id,
                password_stdin,
                "Password required to update encrypted notes.",
            )
            .await?;
        let payload_permit = self.blocking_crypto.admit_large_payload().await?;
        let current_response = client.get_note_encoded(work_list_id, note_id).await?;
        let runtime = self.clone();
        let crypto_context = context.clone();
        let (payload_permit, prepared) = self
            .blocking_crypto
            .run_with_large_payload(
                payload_permit,
                move || {
                    let current = current_response.decode()?;
                    let mut request =
                        runtime.prepare_update_note_request(&current, input, &crypto_context)?;
                    if let Some(expected_updated_at) = expected_updated_at {
                        request.expected_updated_at = Some(expected_updated_at);
                    }
                    let encoded = EncodedNoteRequest::encode(&request)?;
                    Ok(PreparedNoteUpdate {
                        current,
                        request,
                        encoded,
                    })
                },
                "note update parsing, encryption, and encoding task failed",
            )
            .await?;
        match client
            .update_note_encoded(work_list_id, note_id, prepared.encoded)
            .await
        {
            Ok(response) => {
                let runtime = self.clone();
                let projection_context = context.clone();
                self.finish_update_note_response_with(
                    &mut client,
                    work_list_id,
                    note_id,
                    prepared.current,
                    prepared.request,
                    &context,
                    response,
                    payload_permit,
                    move |response| {
                        response
                            .decode()
                            .map(|note| runtime.project_note(note, &projection_context))
                    },
                )
                .await
            }
            Err(primary) if mutation_outcome_is_ambiguous(&primary) => {
                self.reconcile_update_note(
                    &mut client,
                    work_list_id,
                    note_id,
                    prepared.current,
                    prepared.request,
                    &context,
                    primary,
                    false,
                    MUTATION_RECONCILIATION_TIMEOUT,
                    payload_permit,
                )
                .await
            }
            Err(primary) => Err(primary),
        }
    }

    pub async fn delete_note(&self, args: DeleteNoteArgs) -> PublicResult<()> {
        let mut client = self.authenticated_api_client()?;
        let payload_permit = self.blocking_crypto.admit_large_payload().await?;
        let current_response = client
            .get_note_encoded(args.work_list_id, args.note_id)
            .await?;
        let (payload_permit, prepared) = self
            .blocking_crypto
            .run_with_large_payload(
                payload_permit,
                move || {
                    let current = current_response.decode()?;
                    let encoded = EncodedNoteRequest::encode(&args.input)?;
                    Ok(PreparedNoteDelete { current, encoded })
                },
                "note deletion parsing and encoding task failed",
            )
            .await?;
        match client
            .delete_note_encoded(args.work_list_id, args.note_id, prepared.encoded)
            .await
        {
            Ok(response) => {
                self.finish_delete_note_response_with(
                    &mut client,
                    args.work_list_id,
                    args.note_id,
                    prepared.current,
                    response,
                    payload_permit,
                    |response| response.decode().map(|_| ()),
                )
                .await
            }
            Err(primary) if mutation_outcome_is_ambiguous(&primary) => {
                reconcile_delete_note(
                    &self.blocking_crypto,
                    &mut client,
                    args.work_list_id,
                    args.note_id,
                    prepared.current,
                    primary,
                    false,
                    MUTATION_RECONCILIATION_TIMEOUT,
                    payload_permit,
                )
                .await
            }
            Err(primary) => Err(primary),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::WorkListContext;
    use chrono::{TimeDelta, Utc};
    use sealtask_client_api::{
        DeleteNoteRequest, MAX_NOTE_DECOMPRESSED_PAGE_BYTES, MAX_NOTE_MUTATION_REQUEST_BYTES,
        NotePage,
    };
    use sealtask_client_auth::Credentials;
    use sealtask_client_crypto::{KEY_SIZE, encrypt_note_payload};
    use std::sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    enum InitialFailure {
        LostResponse,
        MalformedResponse,
    }

    #[derive(Clone, Copy)]
    enum ConfirmedCreateBodyFailure {
        Oversized,
        Truncated,
    }

    impl ConfirmedCreateBodyFailure {
        const fn label(self) -> &'static str {
            match self {
                Self::Oversized => "oversized",
                Self::Truncated => "truncated",
            }
        }
    }

    fn test_credentials(api_url: &str) -> Credentials {
        Credentials {
            api_url: api_url.to_string(),
            access_token: "test-access".to_string(),
            refresh_token: "test-refresh".to_string(),
            access_expires_at: Utc::now() + TimeDelta::hours(1),
            refresh_expires_at: Utc::now() + TimeDelta::hours(2),
            user_id: Uuid::now_v7(),
            email: "agent@example.com".to_string(),
            data_key_ciphertext: "unused".to_string(),
        }
    }

    async fn read_http_request(stream: &mut tokio::net::TcpStream) -> Vec<u8> {
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1024];
        while !request.windows(4).any(|window| window == b"\r\n\r\n") {
            let read = stream.read(&mut buffer).await.expect("request bytes");
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
        }
        let header_end = request
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .map(|position| position + 4)
            .unwrap_or(request.len());
        let content_length = String::from_utf8_lossy(&request[..header_end])
            .lines()
            .find_map(|line| {
                line.split_once(':').and_then(|(name, value)| {
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().expect("content length"))
                })
            })
            .unwrap_or(0);
        while request.len().saturating_sub(header_end) < content_length {
            let read = stream.read(&mut buffer).await.expect("request body bytes");
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
        }
        request
    }

    fn http_request_body(request: &[u8]) -> &[u8] {
        request
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .map_or(&[], |position| &request[position + 4..])
    }

    async fn write_http_response(stream: &mut tokio::net::TcpStream, body: &[u8]) {
        write_http_status_response(stream, "200 OK", body).await;
    }

    async fn write_http_status_response(
        stream: &mut tokio::net::TcpStream,
        status: &str,
        body: &[u8],
    ) {
        let headers = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream
            .write_all(headers.as_bytes())
            .await
            .expect("response headers");
        stream.write_all(body).await.expect("response body");
    }

    async fn write_confirmed_create_body_failure(
        stream: &mut tokio::net::TcpStream,
        failure: ConfirmedCreateBodyFailure,
    ) {
        let (declared_len, body) = match failure {
            ConfirmedCreateBodyFailure::Oversized => {
                let body = vec![b'a'; MAX_NOTE_DECOMPRESSED_PAGE_BYTES + 1];
                (body.len(), body)
            }
            ConfirmedCreateBodyFailure::Truncated => (128, b"{".to_vec()),
        };
        let headers = format!(
            "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nContent-Length: {declared_len}\r\nConnection: close\r\n\r\n"
        );
        stream
            .write_all(headers.as_bytes())
            .await
            .expect("response headers");
        let _ = stream.write_all(&body).await;
    }

    fn note_fixture(
        list_key: &SymmetricKey,
        id: Uuid,
        work_list_id: Uuid,
        title: &str,
        updated_at: chrono::DateTime<Utc>,
    ) -> NoteResponse {
        let body = NotePayloadBody {
            title: title.to_string(),
            content: markdown_note_content("Body").expect("content"),
            mentions: Some(Vec::new()),
            attachments: Some(Vec::new()),
            client_meta: Some(FlexibleValue::Map(Vec::new())),
        };
        let encrypted = encrypt_note_payload(&build_note_payload_envelope(body, 1), list_key)
            .expect("encrypted note");
        NoteResponse {
            id,
            work_list_id,
            created_by_membership_id: Uuid::now_v7(),
            title_ciphertext: format!("title-ciphertext-{title}"),
            legacy_cbor_fields: Vec::new(),
            payload_ciphertext: encrypted.base64,
            is_private: false,
            note_key_ciphertext: None,
            created_at: updated_at - TimeDelta::minutes(1),
            updated_at,
        }
    }

    fn unlocked_context(list_key: SymmetricKey) -> UnlockedWorkListContext {
        UnlockedWorkListContext {
            work_list: WorkListContext {
                work_list_title: None,
                work_list_timezone: "UTC".to_string(),
                list_key: Some(list_key),
                task_reference_schemes: Vec::new(),
                current_task_reference_scheme_revision: None,
                current_task_reference_scheme_revision_id: None,
                read_error: None,
            },
            data_key: SymmetricKey::new([0x41; KEY_SIZE]),
            membership_id: Uuid::now_v7(),
        }
    }

    fn create_request_for(note: &NoteResponse) -> CreateNoteRequest {
        CreateNoteRequest {
            idempotency_key: format!("note:{}", Uuid::now_v7()),
            idempotency_commitment: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string(),
            title_ciphertext: note.title_ciphertext.clone(),
            title_ciphertext_proof: "title-proof".to_string(),
            payload_ciphertext: note.payload_ciphertext.clone(),
            payload_ciphertext_proof: "payload-proof".to_string(),
            is_private: note.is_private,
            note_key_ciphertext: note.note_key_ciphertext.clone(),
            audit_patch: None,
        }
    }

    async fn fill_blocking_admission(
        admission: &BlockingCryptoAdmission,
        gate: Arc<(Mutex<bool>, Condvar)>,
    ) -> Vec<tokio::task::JoinHandle<PublicResult<()>>> {
        let mut tasks = Vec::new();
        for _ in 0..10 {
            let admission = admission.clone();
            let worker_gate = gate.clone();
            tasks.push(tokio::spawn(async move {
                admission
                    .run(
                        move || {
                            let (lock, condition) = &*worker_gate;
                            let mut released = lock
                                .lock()
                                .unwrap_or_else(std::sync::PoisonError::into_inner);
                            while !*released {
                                released = condition
                                    .wait(released)
                                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                            }
                            Ok(())
                        },
                        "note saturation task failed",
                    )
                    .await
            }));
        }
        tokio::time::timeout(Duration::from_secs(1), async {
            while admission.waiting_count() < 8 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("fill bounded blocking wait queue");
        tasks
    }

    fn release_blocking_gate(gate: &Arc<(Mutex<bool>, Condvar)>) {
        let (lock, condition) = &**gate;
        *lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = true;
        condition.notify_all();
    }

    #[test]
    fn encoded_note_collection_budget_counts_serialized_utf8_bytes() {
        let list_key = SymmetricKey::new([0x57; KEY_SIZE]);
        let mut note = note_fixture(
            &list_key,
            Uuid::now_v7(),
            Uuid::now_v7(),
            "Unicode",
            Utc::now(),
        );
        note.title_ciphertext = "é🙂".to_string();
        let encoded_size = serde_json::to_vec(&note).expect("encoded note").len();

        let mut exact = 0;
        add_received_note_page_to_encoded_budget_with_limit(&mut exact, encoded_size, encoded_size)
            .expect("exact UTF-8 byte budget");
        assert_eq!(exact, encoded_size);

        let mut too_small = 0;
        let error = add_received_note_page_to_encoded_budget_with_limit(
            &mut too_small,
            encoded_size,
            encoded_size - 1,
        )
        .expect_err("one encoded byte over budget");
        assert!(error.to_string().contains("encoded safety limit"));
    }

    #[test]
    fn plaintext_limit_counts_utf8_bytes_not_characters() {
        let title = "Title";
        let exact = TaskPayloadRichText {
            format: "markdown".to_string(),
            version: 1,
            blocks: vec![RichTextBlock {
                block_type: "paragraph".to_string(),
                text: "a".repeat(MAX_NOTE_PLAINTEXT_BYTES - title.len()),
            }],
        };
        validate_note_plaintext_size(title, &exact).expect("exact byte boundary");

        let unicode_over = TaskPayloadRichText {
            format: "markdown".to_string(),
            version: 1,
            blocks: vec![RichTextBlock {
                block_type: "paragraph".to_string(),
                text: format!(
                    "{}é",
                    "a".repeat(MAX_NOTE_PLAINTEXT_BYTES - title.len() - 1)
                ),
            }],
        };
        let error = validate_note_plaintext_size(title, &unicode_over)
            .expect_err("UTF-8 expansion must count toward the byte limit");
        assert!(matches!(error, PublicError::PayloadTooLarge(_)));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn note_page_projection_keeps_the_tokio_worker_responsive() {
        let runtime = RuntimeClient::new("http://127.0.0.1:9").expect("runtime");
        let blocking_crypto = runtime.blocking_crypto.clone();
        let list_key = SymmetricKey::new([0x4f; KEY_SIZE]);
        let work_list_id = Uuid::now_v7();
        let context = unlocked_context(list_key.clone());
        let notes = (0..32)
            .map(|index| {
                note_fixture(
                    &list_key,
                    Uuid::now_v7(),
                    work_list_id,
                    &format!("Note {index}"),
                    Utc::now(),
                )
            })
            .collect();
        let projection_runtime = runtime.clone();
        let projection =
            tokio::spawn(async move { projection_runtime.project_note_page(notes, context).await });

        tokio::time::timeout(Duration::from_secs(1), blocking_crypto.wait_for_start())
            .await
            .expect("note page projection starts on the blocking pool");
        let heartbeat = Arc::new(AtomicBool::new(false));
        let heartbeat_seen = heartbeat.clone();
        tokio::spawn(async move {
            heartbeat_seen.store(true, Ordering::Release);
        })
        .await
        .expect("single-worker heartbeat");
        assert!(
            heartbeat.load(Ordering::Acquire),
            "note page decryption must not run on the Tokio worker"
        );

        let projected = projection
            .await
            .expect("projection task joins")
            .expect("projection succeeds");
        assert_eq!(projected.len(), 32);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn maximum_note_json_encode_and_parse_keep_the_worker_responsive() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let mut response_body = serde_json::to_vec(&NotePage {
            notes: Vec::new(),
            next_cursor: None,
        })
        .expect("note page");
        response_body.resize(MAX_NOTE_DECOMPRESSED_PAGE_BYTES, b' ');
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("connection");
            read_http_request(&mut stream).await;
            write_http_response(&mut stream, &response_body).await;
        });
        let mut client = PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
            .expect("API client");
        let response = client
            .list_notes_page_encoded(Uuid::now_v7(), None, MAX_NOTE_PAGE_ITEMS)
            .await
            .expect("exact-limit response");
        server.await.expect("server");

        let admission = BlockingCryptoAdmission::default();
        let payload_permit = admission
            .admit_large_payload()
            .await
            .expect("payload admission");
        let baseline = serde_json::to_vec(&UpdateNoteRequest {
            payload_ciphertext: Some(String::new()),
            ..UpdateNoteRequest::default()
        })
        .expect("baseline request");
        let request = UpdateNoteRequest {
            payload_ciphertext: Some("x".repeat(MAX_NOTE_MUTATION_REQUEST_BYTES - baseline.len())),
            ..UpdateNoteRequest::default()
        };
        let worker_admission = admission.clone();
        let work = tokio::spawn(async move {
            worker_admission
                .run_with_large_payload(
                    payload_permit,
                    move || {
                        let encoded = EncodedNoteRequest::encode(&request)?;
                        if encoded.encoded_len() != MAX_NOTE_MUTATION_REQUEST_BYTES {
                            return Err(PublicError::unexpected(
                                "test request did not reach the exact encoded limit",
                            ));
                        }
                        let _: NotePage = response.decode()?;
                        Ok(())
                    },
                    "maximum note JSON task failed",
                )
                .await
        });
        tokio::time::timeout(Duration::from_secs(1), admission.wait_for_start())
            .await
            .expect("maximum note JSON reaches the blocking pool");

        let heartbeat = Arc::new(AtomicBool::new(false));
        let heartbeat_seen = heartbeat.clone();
        tokio::spawn(async move {
            heartbeat_seen.store(true, Ordering::Release);
        })
        .await
        .expect("single-worker heartbeat");
        assert!(
            heartbeat.load(Ordering::Acquire),
            "maximum note JSON work must not run on the current-thread Tokio worker"
        );
        let (_, ()) = work
            .await
            .expect("JSON task joins")
            .expect("maximum note JSON succeeds");
        assert_eq!(admission.available_large_payload_permits(), 2);
    }

    #[tokio::test]
    async fn create_response_worker_panic_reconciles_to_the_committed_note_identity() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let runtime = RuntimeClient::new(&api_url).expect("runtime");
        let list_key = SymmetricKey::new([0x61; KEY_SIZE]);
        let context = unlocked_context(list_key.clone());
        let work_list_id = Uuid::now_v7();
        let note = note_fixture(
            &list_key,
            Uuid::now_v7(),
            work_list_id,
            "Committed create",
            Utc::now(),
        );
        let expected_id = note.id;
        let request = create_request_for(&note);
        let replayed_response = serde_json::to_vec(&note).expect("replayed note response");
        let server = tokio::spawn(async move {
            let (mut mutation, _) = listener.accept().await.expect("mutation connection");
            read_http_request(&mut mutation).await;
            write_http_response(&mut mutation, b"{}").await;

            let (mut reconciliation, _) =
                listener.accept().await.expect("reconciliation connection");
            let reconciliation_request = read_http_request(&mut reconciliation).await;
            assert!(reconciliation_request.starts_with(b"POST "));
            write_http_response(&mut reconciliation, &replayed_response).await;
        });
        let mut client = PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
            .expect("API client");
        let encoded = EncodedNoteRequest::encode(&request).expect("encode create note request");
        let response = client
            .create_note_encoded(work_list_id, encoded)
            .await
            .expect("bounded create response");
        let payload_permit = runtime
            .blocking_crypto
            .admit_large_payload()
            .await
            .expect("payload admission");

        let created = runtime
            .finish_create_note_response_with(
                &mut client,
                work_list_id,
                request,
                &context,
                response,
                payload_permit,
                |_| -> PublicResult<AgentNote> {
                    panic!("create response panic canary");
                },
            )
            .await
            .expect("panic after confirmed create must reconcile");

        assert_eq!(created.id, expected_id);
        assert_eq!(runtime.blocking_crypto.available_large_payload_permits(), 2);
        server.await.expect("server");
    }

    #[tokio::test]
    async fn confirmed_201_body_failures_recover_through_the_durable_create_key() {
        for failure in [
            ConfirmedCreateBodyFailure::Oversized,
            ConfirmedCreateBodyFailure::Truncated,
        ] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("listener");
            let api_url = format!("http://{}", listener.local_addr().expect("address"));
            let runtime = RuntimeClient::new(&api_url).expect("runtime");
            let list_key = SymmetricKey::new([0x66; KEY_SIZE]);
            let context = unlocked_context(list_key.clone());
            let work_list_id = Uuid::now_v7();
            let committed = note_fixture(
                &list_key,
                Uuid::now_v7(),
                work_list_id,
                "Recovered committed note",
                Utc::now(),
            );
            let committed_id = committed.id;
            let request = CreateNoteRequest {
                idempotency_key: format!("note:{}", Uuid::now_v7()),
                idempotency_commitment: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string(),
                title_ciphertext: "created-title".to_string(),
                title_ciphertext_proof: "title-proof".to_string(),
                payload_ciphertext: "created-payload".to_string(),
                payload_ciphertext_proof: "payload-proof".to_string(),
                is_private: false,
                note_key_ciphertext: None,
                audit_patch: None,
            };
            let committed_response =
                serde_json::to_vec(&committed).expect("committed note response");
            let server = tokio::spawn(async move {
                let (mut mutation, _) = listener.accept().await.expect("mutation connection");
                let mutation_request = read_http_request(&mut mutation).await;
                assert!(
                    mutation_request.starts_with(b"POST "),
                    "first request must be the note create"
                );
                write_confirmed_create_body_failure(&mut mutation, failure).await;
                drop(mutation);

                let (mut reconciliation, _) =
                    listener.accept().await.expect("reconciliation connection");
                let reconciliation_request = read_http_request(&mut reconciliation).await;
                assert!(
                    reconciliation_request.starts_with(b"POST "),
                    "confirmed create must retry the same durable operation identity"
                );
                write_http_response(&mut reconciliation, &committed_response).await;
            });
            let mut client =
                PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
                    .expect("API client");
            let encoded = EncodedNoteRequest::encode(&request).expect("encode create note request");
            let response = client
                .create_note_encoded(work_list_id, encoded)
                .await
                .unwrap_or_else(|error| {
                    panic!(
                        "known 201 status must reach the runtime after the {} body failure: {error}",
                        failure.label()
                    )
                });
            assert!(response.is_success_status());
            let payload_permit = runtime
                .blocking_crypto
                .admit_large_payload()
                .await
                .expect("payload admission");

            let recovered = runtime
                .finish_create_note_response_with(
                    &mut client,
                    work_list_id,
                    request,
                    &context,
                    response,
                    payload_permit,
                    |response| -> PublicResult<AgentNote> {
                        let _ = response.decode()?;
                        Err(PublicError::unexpected(
                            "body-failure response unexpectedly decoded",
                        ))
                    },
                )
                .await
                .expect("durable retry should recover the committed create");
            assert_eq!(recovered.id, committed_id);
            assert_eq!(runtime.blocking_crypto.available_large_payload_permits(), 2);
            server.await.expect("server");
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn update_response_admission_saturation_reconciles_to_requested_revision() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let runtime = RuntimeClient::new(&api_url).expect("runtime");
        let list_key = SymmetricKey::new([0x62; KEY_SIZE]);
        let context = unlocked_context(list_key.clone());
        let work_list_id = Uuid::now_v7();
        let note_id = Uuid::now_v7();
        let current = note_fixture(&list_key, note_id, work_list_id, "Old", Utc::now());
        let intended = note_fixture(
            &list_key,
            note_id,
            work_list_id,
            "New",
            current.updated_at + TimeDelta::seconds(1),
        );
        let request = UpdateNoteRequest {
            expected_updated_at: Some(current.updated_at),
            title_ciphertext: Some(intended.title_ciphertext.clone()),
            title_ciphertext_proof: Some("title-proof".to_string()),
            payload_ciphertext: Some(intended.payload_ciphertext.clone()),
            payload_ciphertext_proof: Some("payload-proof".to_string()),
            ..UpdateNoteRequest::default()
        };
        let response_body = serde_json::to_vec(&intended).expect("note response");
        let gate = Arc::new((Mutex::new(false), Condvar::new()));
        let server_gate = gate.clone();
        let server_admission = runtime.blocking_crypto.clone();
        let server = tokio::spawn(async move {
            let (mut mutation, _) = listener.accept().await.expect("mutation connection");
            read_http_request(&mut mutation).await;
            write_http_response(&mut mutation, &response_body).await;

            let (mut reconciliation, _) =
                listener.accept().await.expect("reconciliation connection");
            read_http_request(&mut reconciliation).await;
            release_blocking_gate(&server_gate);
            tokio::time::timeout(Duration::from_secs(1), async {
                while server_admission.waiting_count() != 0
                    || server_admission.available_permits() != 2
                {
                    tokio::task::yield_now().await;
                }
            })
            .await
            .expect("drain saturated blocking admission");
            write_http_response(&mut reconciliation, &response_body).await;
        });
        let mut client = PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
            .expect("API client");
        let encoded = EncodedNoteRequest::encode(&request).expect("encode update note request");
        let response = client
            .update_note_encoded(work_list_id, note_id, encoded)
            .await
            .expect("bounded update response");
        let payload_permit = runtime
            .blocking_crypto
            .admit_large_payload()
            .await
            .expect("payload admission");
        let saturation_tasks = fill_blocking_admission(&runtime.blocking_crypto, gate).await;
        let response_work_ran = Arc::new(AtomicBool::new(false));
        let response_work_ran_in_worker = response_work_ran.clone();
        let projection_runtime = runtime.clone();
        let projection_context = context.clone();

        let updated = runtime
            .finish_update_note_response_with(
                &mut client,
                work_list_id,
                note_id,
                current,
                request,
                &context,
                response,
                payload_permit,
                move |response| {
                    response_work_ran_in_worker.store(true, Ordering::Release);
                    response
                        .decode()
                        .map(|note| projection_runtime.project_note(note, &projection_context))
                },
            )
            .await
            .expect("saturated post-response admission must reconcile");

        assert_eq!(updated.id, note_id);
        assert_eq!(updated.title.as_deref(), Some("New"));
        assert!(
            !response_work_ran.load(Ordering::Acquire),
            "the rejected response worker must not execute"
        );
        for task in saturation_tasks {
            task.await
                .expect("saturation task joins")
                .expect("saturation task completes");
        }
        assert_eq!(runtime.blocking_crypto.available_large_payload_permits(), 2);
        server.await.expect("server");
    }

    #[tokio::test]
    async fn delete_response_processing_failure_reports_confirmed_commit_without_retry() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let runtime = RuntimeClient::new(&api_url).expect("runtime");
        let list_key = SymmetricKey::new([0x63; KEY_SIZE]);
        let work_list_id = Uuid::now_v7();
        let note_id = Uuid::now_v7();
        let current = note_fixture(&list_key, note_id, work_list_id, "Deleted", Utc::now());
        let server = tokio::spawn(async move {
            let (mut mutation, _) = listener.accept().await.expect("mutation connection");
            read_http_request(&mut mutation).await;
            write_http_status_response(&mut mutation, "204 No Content", &[]).await;
        });
        let mut client = PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
            .expect("API client");
        let encoded = EncodedNoteRequest::encode(&DeleteNoteRequest::default())
            .expect("encode delete request");
        let response = client
            .delete_note_encoded(work_list_id, note_id, encoded)
            .await
            .expect("bounded delete response");
        let payload_permit = runtime
            .blocking_crypto
            .admit_large_payload()
            .await
            .expect("payload admission");

        let error = runtime
            .finish_delete_note_response_with(
                &mut client,
                work_list_id,
                note_id,
                current,
                response,
                payload_permit,
                |_| {
                    Err(PublicError::unexpected(
                        "delete local processing secret canary",
                    ))
                },
            )
            .await
            .expect_err("confirmed delete processing failure must be explicit");

        assert!(matches!(
            error,
            PublicError::CommittedButLocalProcessingFailed {
                committed_resource,
                details,
                ..
            } if committed_resource == format!("note:{note_id}")
                && details.contains("local_failure=api_mutation")
                && !details.contains("secret canary")
        ));
        assert_eq!(runtime.blocking_crypto.available_large_payload_permits(), 2);
        server.await.expect("server");
    }

    #[tokio::test]
    async fn confirmed_create_projection_panic_reports_identity_and_releases_permit() {
        let runtime = RuntimeClient::new("http://127.0.0.1:9").expect("runtime");
        let list_key = SymmetricKey::new([0x64; KEY_SIZE]);
        let note = note_fixture(
            &list_key,
            Uuid::now_v7(),
            Uuid::now_v7(),
            "Committed",
            Utc::now(),
        );
        let note_id = note.id;
        let payload_permit = runtime
            .blocking_crypto
            .admit_large_payload()
            .await
            .expect("payload admission");

        let error = runtime
            .project_reconciled_create_note_with(payload_permit, note, |_| {
                panic!("confirmed create projection secret canary");
            })
            .await
            .expect_err("confirmed projection panic must be typed");

        assert!(matches!(
            error,
            PublicError::CommittedButLocalProcessingFailed {
                committed_resource,
                details,
                ..
            } if committed_resource == format!("note:{note_id}")
                && details.contains("reconciliation=projection")
                && !details.contains("secret canary")
        ));
        assert_eq!(runtime.blocking_crypto.available_large_payload_permits(), 2);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn maximum_single_note_crypto_keeps_the_worker_responsive_and_releases_admission() {
        let runtime = RuntimeClient::new("http://127.0.0.1:9").expect("runtime");
        let admission = runtime.blocking_crypto.clone();
        let list_key = SymmetricKey::new([0x5a; KEY_SIZE]);
        let context = unlocked_context(list_key);
        let work_list_id = Uuid::now_v7();
        let body = std::iter::repeat_n("x".repeat(MAX_RICH_TEXT_BLOCK_CHARS), MAX_RICH_TEXT_BLOCKS)
            .collect::<Vec<_>>()
            .join("\n\n");

        let crypto_runtime = runtime.clone();
        let prepare_context = context.clone();
        let create = tokio::spawn(async move {
            let worker_runtime = crypto_runtime.clone();
            crypto_runtime
                .blocking_crypto
                .run(
                    move || {
                        worker_runtime.prepare_create_note_request(
                            NoteCreateInput {
                                title: "Maximum note".to_string(),
                                body,
                                is_private: false,
                                idempotency_key: "note:maximum".to_string(),
                            },
                            &prepare_context,
                        )
                    },
                    "note creation encryption task failed",
                )
                .await
        });
        tokio::time::timeout(Duration::from_secs(1), admission.wait_for_start())
            .await
            .expect("maximum note creation reaches blocking pool");
        assert!(
            admission.available_permits() < 2,
            "active note crypto must own bounded admission"
        );
        let heartbeat = Arc::new(AtomicBool::new(false));
        let heartbeat_seen = heartbeat.clone();
        tokio::spawn(async move {
            heartbeat_seen.store(true, Ordering::Release);
        })
        .await
        .expect("creation heartbeat");
        assert!(
            heartbeat.load(Ordering::Acquire),
            "maximum note creation crypto must not run on the current-thread Tokio worker"
        );
        let create_request = create
            .await
            .expect("creation task")
            .expect("create request encryption");
        assert_eq!(admission.available_permits(), 2);

        let current = NoteResponse {
            id: Uuid::now_v7(),
            work_list_id,
            created_by_membership_id: context.membership_id,
            title_ciphertext: create_request.title_ciphertext,
            legacy_cbor_fields: Vec::new(),
            payload_ciphertext: create_request.payload_ciphertext,
            is_private: create_request.is_private,
            note_key_ciphertext: create_request.note_key_ciphertext,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let crypto_runtime = runtime.clone();
        let update_context = context.clone();
        let update = tokio::spawn(async move {
            let worker_runtime = crypto_runtime.clone();
            crypto_runtime
                .blocking_crypto
                .run(
                    move || {
                        let request = worker_runtime.prepare_update_note_request(
                            &current,
                            NoteUpdateInput {
                                title: Some("Maximum note updated".to_string()),
                                body: None,
                            },
                            &update_context,
                        )?;
                        Ok((current, request))
                    },
                    "note update encryption task failed",
                )
                .await
        });
        tokio::time::timeout(Duration::from_secs(1), admission.wait_for_start())
            .await
            .expect("maximum note update reaches blocking pool");
        let heartbeat = Arc::new(AtomicBool::new(false));
        let heartbeat_seen = heartbeat.clone();
        tokio::spawn(async move {
            heartbeat_seen.store(true, Ordering::Release);
        })
        .await
        .expect("update heartbeat");
        assert!(
            heartbeat.load(Ordering::Acquire),
            "maximum note update crypto must not run on the current-thread Tokio worker"
        );
        let (mut updated_note, update_request) = update
            .await
            .expect("update task")
            .expect("update request encryption");
        updated_note.title_ciphertext = update_request
            .title_ciphertext
            .expect("updated title ciphertext");
        updated_note.payload_ciphertext = update_request
            .payload_ciphertext
            .expect("updated payload ciphertext");
        updated_note.updated_at = Utc::now();
        assert_eq!(admission.available_permits(), 2);

        let projection_runtime = runtime.clone();
        let projection_context = context.clone();
        let projection = tokio::spawn(async move {
            projection_runtime
                .project_single_note(updated_note, projection_context)
                .await
        });
        tokio::time::timeout(Duration::from_secs(1), admission.wait_for_start())
            .await
            .expect("maximum note projection reaches blocking pool");
        let heartbeat = Arc::new(AtomicBool::new(false));
        let heartbeat_seen = heartbeat.clone();
        tokio::spawn(async move {
            heartbeat_seen.store(true, Ordering::Release);
        })
        .await
        .expect("projection heartbeat");
        assert!(
            heartbeat.load(Ordering::Acquire),
            "maximum note get projection must not run on the current-thread Tokio worker"
        );
        let projected = projection
            .await
            .expect("projection task")
            .expect("note projection");
        assert_eq!(projected.title.as_deref(), Some("Maximum note updated"));
        assert_eq!(admission.available_permits(), 2);
    }

    async fn serve_initial_failure_and_response(
        listener: tokio::net::TcpListener,
        failure: InitialFailure,
        response: Vec<u8>,
    ) {
        let (mut mutation, _) = listener.accept().await.expect("mutation connection");
        read_http_request(&mut mutation).await;
        if matches!(failure, InitialFailure::MalformedResponse) {
            write_http_response(&mut mutation, b"{").await;
        }
        drop(mutation);

        let (mut reconcile, _) = listener.accept().await.expect("reconcile connection");
        read_http_request(&mut reconcile).await;
        write_http_response(&mut reconcile, &response).await;
    }

    async fn serve_lost_response_rate_limit_then_response(
        listener: tokio::net::TcpListener,
        response: Vec<u8>,
    ) -> Vec<Vec<u8>> {
        let mut requests = Vec::new();

        let (mut original, _) = listener.accept().await.expect("original connection");
        requests.push(read_http_request(&mut original).await);
        drop(original);

        let (mut overlapping_retry, _) = listener.accept().await.expect("overlap connection");
        requests.push(read_http_request(&mut overlapping_retry).await);
        write_http_status_response(
            &mut overlapping_retry,
            "429 Too Many Requests",
            br#"{"error":"rate_limited","message":"operation still in progress"}"#,
        )
        .await;

        let (mut completed_retry, _) = listener.accept().await.expect("completed connection");
        requests.push(read_http_request(&mut completed_retry).await);
        write_http_response(&mut completed_retry, &response).await;

        requests
    }

    #[tokio::test]
    async fn bounded_note_scan_follows_cursors_to_later_pages() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let list_key = SymmetricKey::new([0x50; KEY_SIZE]);
        let work_list_id = Uuid::now_v7();
        let target_id = Uuid::now_v7();
        let first_page = serde_json::to_vec(&NotePage {
            notes: vec![note_fixture(
                &list_key,
                Uuid::now_v7(),
                work_list_id,
                "First",
                Utc::now(),
            )],
            next_cursor: Some("cursor-1".to_string()),
        })
        .expect("first page JSON");
        let second_page = serde_json::to_vec(&NotePage {
            notes: vec![note_fixture(
                &list_key,
                target_id,
                work_list_id,
                "Target",
                Utc::now() - TimeDelta::seconds(1),
            )],
            next_cursor: None,
        })
        .expect("second page JSON");
        let server = tokio::spawn(async move {
            for response in [first_page, second_page] {
                let (mut stream, _) = listener.accept().await.expect("page connection");
                read_http_request(&mut stream).await;
                write_http_response(&mut stream, &response).await;
            }
        });
        let mut client = PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
            .expect("API client");

        let admission = BlockingCryptoAdmission::default();
        let permit = admission
            .admit_large_payload()
            .await
            .expect("payload admission");
        let (_, found) = find_note_in_bounded_pages(
            &admission,
            &mut client,
            work_list_id,
            move |note| note.id == target_id,
            permit,
        )
        .await
        .expect("bounded page scan");
        let found = found.expect("target note");

        assert_eq!(found.id, target_id);
        server.await.expect("server");
    }

    #[tokio::test]
    async fn bounded_note_scan_rejects_repeated_cursors() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let list_key = SymmetricKey::new([0x55; KEY_SIZE]);
        let work_list_id = Uuid::now_v7();
        let pages = ["First", "Second"]
            .into_iter()
            .map(|title| {
                serde_json::to_vec(&NotePage {
                    notes: vec![note_fixture(
                        &list_key,
                        Uuid::now_v7(),
                        work_list_id,
                        title,
                        Utc::now(),
                    )],
                    next_cursor: Some("cursor-1".to_string()),
                })
                .expect("page JSON")
            })
            .collect::<Vec<_>>();
        let server = tokio::spawn(async move {
            for response in pages {
                let (mut stream, _) = listener.accept().await.expect("page connection");
                read_http_request(&mut stream).await;
                write_http_response(&mut stream, &response).await;
            }
        });
        let mut client = PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
            .expect("API client");

        let admission = BlockingCryptoAdmission::default();
        let permit = admission
            .admit_large_payload()
            .await
            .expect("payload admission");
        let error =
            find_note_in_bounded_pages(&admission, &mut client, work_list_id, |_| false, permit)
                .await
                .expect_err("repeated cursor must fail");

        assert!(error.to_string().contains("repeated"));
        server.await.expect("server");
    }

    #[tokio::test]
    async fn create_reconciles_lost_and_malformed_success_responses() {
        for failure in [
            InitialFailure::LostResponse,
            InitialFailure::MalformedResponse,
        ] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("listener");
            let api_url = format!("http://{}", listener.local_addr().expect("address"));
            let runtime = RuntimeClient::new(&api_url).expect("runtime");
            let list_key = SymmetricKey::new([0x51; KEY_SIZE]);
            let context = unlocked_context(list_key.clone());
            let work_list_id = Uuid::now_v7();
            let note = note_fixture(
                &list_key,
                Uuid::now_v7(),
                work_list_id,
                "Created",
                Utc::now(),
            );
            let expected_id = note.id;
            let request = CreateNoteRequest {
                idempotency_key: format!("note:{}", Uuid::now_v7()),
                idempotency_commitment: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string(),
                title_ciphertext: note.title_ciphertext.clone(),
                title_ciphertext_proof: "title-proof".to_string(),
                payload_ciphertext: note.payload_ciphertext.clone(),
                payload_ciphertext_proof: "payload-proof".to_string(),
                is_private: false,
                note_key_ciphertext: None,
                audit_patch: None,
            };
            let response = serde_json::to_vec(&note).expect("note JSON");
            let server = tokio::spawn(serve_initial_failure_and_response(
                listener, failure, response,
            ));
            let mut client =
                PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
                    .expect("API client");
            let encoded = EncodedNoteRequest::encode(&request).expect("encode create note request");
            let (primary, commit_confirmed) =
                match client.create_note_encoded(work_list_id, encoded).await {
                    Ok(response) => {
                        assert!(response.is_success_status());
                        let primary = response
                            .decode()
                            .expect_err("malformed success response must fail");
                        assert_eq!(primary.code(), "committed_but_local_processing_failed");
                        assert!(!mutation_outcome_is_ambiguous(&primary));
                        (primary, true)
                    }
                    Err(primary) => {
                        assert!(mutation_outcome_is_ambiguous(&primary));
                        (primary, false)
                    }
                };
            let created = runtime
                .reconcile_create_note(
                    &mut client,
                    work_list_id,
                    request,
                    &context,
                    primary,
                    commit_confirmed,
                    Duration::from_secs(1),
                    runtime
                        .blocking_crypto
                        .admit_large_payload()
                        .await
                        .expect("payload admission"),
                )
                .await
                .expect("reconciled create");
            assert_eq!(created.id, expected_id);
            server.await.expect("server");
        }
    }

    #[tokio::test]
    async fn create_reconciliation_retries_overlap_until_the_durable_result_is_available() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let runtime = RuntimeClient::new(&api_url).expect("runtime");
        let list_key = SymmetricKey::new([0x5b; KEY_SIZE]);
        let context = unlocked_context(list_key.clone());
        let work_list_id = Uuid::now_v7();
        let note = note_fixture(
            &list_key,
            Uuid::now_v7(),
            work_list_id,
            "Eventually created",
            Utc::now(),
        );
        let expected_id = note.id;
        let request = create_request_for(&note);
        let server = tokio::spawn(serve_lost_response_rate_limit_then_response(
            listener,
            serde_json::to_vec(&note).expect("note JSON"),
        ));
        let mut client = PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
            .expect("API client");
        let primary = client
            .create_note_encoded(
                work_list_id,
                EncodedNoteRequest::encode(&request).expect("encode original request"),
            )
            .await
            .expect_err("original response must be lost");

        let created = runtime
            .reconcile_create_note(
                &mut client,
                work_list_id,
                request,
                &context,
                primary,
                false,
                Duration::from_secs(1),
                runtime
                    .blocking_crypto
                    .admit_large_payload()
                    .await
                    .expect("payload admission"),
            )
            .await
            .expect("reconciliation should outlast a transient overlapping retry");
        assert_eq!(created.id, expected_id);

        let requests = server.await.expect("server");
        assert_eq!(requests.len(), 3);
        assert_eq!(
            http_request_body(&requests[0]),
            http_request_body(&requests[1])
        );
        assert_eq!(
            http_request_body(&requests[1]),
            http_request_body(&requests[2])
        );
    }

    #[tokio::test]
    async fn update_reconciles_lost_and_malformed_success_responses() {
        for failure in [
            InitialFailure::LostResponse,
            InitialFailure::MalformedResponse,
        ] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("listener");
            let api_url = format!("http://{}", listener.local_addr().expect("address"));
            let runtime = RuntimeClient::new(&api_url).expect("runtime");
            let list_key = SymmetricKey::new([0x52; KEY_SIZE]);
            let context = unlocked_context(list_key.clone());
            let work_list_id = Uuid::now_v7();
            let note_id = Uuid::now_v7();
            let current = note_fixture(&list_key, note_id, work_list_id, "Old", Utc::now());
            let intended = note_fixture(
                &list_key,
                note_id,
                work_list_id,
                "New",
                current.updated_at + TimeDelta::seconds(1),
            );
            let request = UpdateNoteRequest {
                expected_updated_at: Some(current.updated_at),
                title_ciphertext: Some(intended.title_ciphertext.clone()),
                title_ciphertext_proof: Some("title-proof".to_string()),
                payload_ciphertext: Some(intended.payload_ciphertext.clone()),
                payload_ciphertext_proof: Some("payload-proof".to_string()),
                ..UpdateNoteRequest::default()
            };
            let response = serde_json::to_vec(&intended).expect("note JSON");
            let server = tokio::spawn(serve_initial_failure_and_response(
                listener, failure, response,
            ));
            let mut client =
                PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
                    .expect("API client");
            let encoded = EncodedNoteRequest::encode(&request).expect("encode update note request");
            let (primary, commit_confirmed) = match client
                .update_note_encoded(work_list_id, note_id, encoded)
                .await
            {
                Ok(response) => {
                    assert!(response.is_success_status());
                    let primary = response
                        .decode()
                        .expect_err("malformed success response must fail");
                    assert_eq!(primary.code(), "committed_but_local_processing_failed");
                    assert!(!mutation_outcome_is_ambiguous(&primary));
                    (primary, true)
                }
                Err(primary) => {
                    assert!(mutation_outcome_is_ambiguous(&primary));
                    (primary, false)
                }
            };
            let updated = runtime
                .reconcile_update_note(
                    &mut client,
                    work_list_id,
                    note_id,
                    current,
                    request,
                    &context,
                    primary,
                    commit_confirmed,
                    Duration::from_secs(1),
                    runtime
                        .blocking_crypto
                        .admit_large_payload()
                        .await
                        .expect("payload admission"),
                )
                .await
                .expect("reconciled update");
            assert_eq!(updated.title.as_deref(), Some("New"));
            server.await.expect("server");
        }
    }

    #[tokio::test]
    async fn delete_reconciliation_keeps_scoped_not_found_ambiguous() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let list_key = SymmetricKey::new([0x53; KEY_SIZE]);
        let work_list_id = Uuid::now_v7();
        let note_id = Uuid::now_v7();
        let current = note_fixture(&list_key, note_id, work_list_id, "Deleted", Utc::now());
        let server = tokio::spawn(async move {
            let (mut mutation, _) = listener.accept().await.expect("mutation connection");
            read_http_request(&mut mutation).await;
            drop(mutation);

            let (mut reconcile, _) = listener.accept().await.expect("reconcile connection");
            let request = read_http_request(&mut reconcile).await;
            assert!(
                String::from_utf8_lossy(&request)
                    .contains(&format!("/work-lists/{work_list_id}/notes/{note_id}")),
                "delete reconciliation must use the direct note lookup"
            );
            write_http_status_response(
                &mut reconcile,
                "404 Not Found",
                br#"{"error":"not_found","message":"note not found"}"#,
            )
            .await;
        });
        let mut client = PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
            .expect("API client");
        let encoded = EncodedNoteRequest::encode(&DeleteNoteRequest::default())
            .expect("encode delete note request");
        let primary = client
            .delete_note_encoded(work_list_id, note_id, encoded)
            .await
            .expect_err("lost delete response");
        let admission = BlockingCryptoAdmission::default();
        let permit = admission
            .admit_large_payload()
            .await
            .expect("payload admission");
        let error = reconcile_delete_note(
            &admission,
            &mut client,
            work_list_id,
            note_id,
            current,
            primary,
            false,
            Duration::from_secs(1),
            permit,
        )
        .await
        .expect_err("scoped absence can also mean project access was lost");
        assert!(matches!(
            error,
            PublicError::OutcomeAmbiguous { details, .. }
                if details.contains("reconciliation=api_read")
        ));
        server.await.expect("server");
    }

    #[tokio::test]
    async fn delete_reconciliation_never_treats_a_moved_note_as_absent() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let api_url = format!("http://{}", listener.local_addr().expect("address"));
        let list_key = SymmetricKey::new([0x56; KEY_SIZE]);
        let work_list_id = Uuid::now_v7();
        let note_id = Uuid::now_v7();
        let current = note_fixture(&list_key, note_id, work_list_id, "Old", Utc::now());
        let moved = note_fixture(
            &list_key,
            note_id,
            work_list_id,
            "Concurrent update",
            current.updated_at + TimeDelta::seconds(1),
        );
        let body = serde_json::to_vec(&moved).expect("note JSON");
        let server = tokio::spawn(async move {
            let (mut reconcile, _) = listener.accept().await.expect("reconcile connection");
            let request = read_http_request(&mut reconcile).await;
            assert!(
                String::from_utf8_lossy(&request)
                    .contains(&format!("/work-lists/{work_list_id}/notes/{note_id}")),
                "reconciliation must not use paginated absence"
            );
            write_http_response(&mut reconcile, &body).await;
        });
        let mut client = PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
            .expect("API client");

        let admission = BlockingCryptoAdmission::default();
        let permit = admission
            .admit_large_payload()
            .await
            .expect("payload admission");
        let error = reconcile_delete_note(
            &admission,
            &mut client,
            work_list_id,
            note_id,
            current,
            PublicError::unexpected("delete response lost"),
            false,
            Duration::from_secs(1),
            permit,
        )
        .await
        .expect_err("concurrent update must remain ambiguous");

        assert!(matches!(
            error,
            PublicError::OutcomeAmbiguous { details, .. }
                if details.contains("reconciliation=divergent")
        ));
        server.await.expect("server");
    }

    #[tokio::test]
    async fn stalled_reconciliation_is_bounded_for_every_note_mutation() {
        let list_key = SymmetricKey::new([0x54; KEY_SIZE]);
        let context = unlocked_context(list_key.clone());
        let work_list_id = Uuid::now_v7();
        let note_id = Uuid::now_v7();
        let current = note_fixture(&list_key, note_id, work_list_id, "Old", Utc::now());
        let create_request = CreateNoteRequest {
            idempotency_key: format!("note:{}", Uuid::now_v7()),
            idempotency_commitment: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string(),
            title_ciphertext: current.title_ciphertext.clone(),
            title_ciphertext_proof: "title-proof".to_string(),
            payload_ciphertext: current.payload_ciphertext.clone(),
            payload_ciphertext_proof: "payload-proof".to_string(),
            is_private: false,
            note_key_ciphertext: None,
            audit_patch: None,
        };
        let update_request = UpdateNoteRequest {
            expected_updated_at: Some(current.updated_at),
            title_ciphertext: Some("new-title-ciphertext".to_string()),
            payload_ciphertext: Some("new-payload-ciphertext".to_string()),
            ..UpdateNoteRequest::default()
        };

        for operation in ["create", "update", "delete"] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("listener");
            let api_url = format!("http://{}", listener.local_addr().expect("address"));
            let runtime = RuntimeClient::new(&api_url).expect("runtime");
            let server = tokio::spawn(async move {
                let (_stream, _) = listener.accept().await.expect("connection");
                tokio::time::sleep(Duration::from_secs(5)).await;
            });
            let mut client =
                PublicApiClient::with_credentials(&api_url, test_credentials(&api_url))
                    .expect("API client");
            let primary = PublicError::unexpected(format!("{operation} response lost"));
            let payload_permit = runtime
                .blocking_crypto
                .admit_large_payload()
                .await
                .expect("payload admission");
            let error = match operation {
                "create" => runtime
                    .reconcile_create_note(
                        &mut client,
                        work_list_id,
                        create_request.clone(),
                        &context,
                        primary,
                        false,
                        Duration::from_millis(30),
                        payload_permit,
                    )
                    .await
                    .map(|_| ()),
                "update" => runtime
                    .reconcile_update_note(
                        &mut client,
                        work_list_id,
                        note_id,
                        current.clone(),
                        update_request.clone(),
                        &context,
                        primary,
                        false,
                        Duration::from_millis(30),
                        payload_permit,
                    )
                    .await
                    .map(|_| ()),
                "delete" => {
                    reconcile_delete_note(
                        &runtime.blocking_crypto,
                        &mut client,
                        work_list_id,
                        note_id,
                        current.clone(),
                        primary,
                        false,
                        Duration::from_millis(30),
                        payload_permit,
                    )
                    .await
                }
                _ => unreachable!(),
            }
            .expect_err("ambiguous stalled reconciliation");
            assert!(matches!(
                error,
                PublicError::OutcomeAmbiguous { details, .. }
                    if details.contains("primary=api_mutation")
                        && details.contains("reconciliation=timeout")
            ));
            server.abort();
        }
    }

    #[test]
    fn malformed_note_attachment_preserves_decrypted_note_fields() {
        let list_key = SymmetricKey::new([0x31; KEY_SIZE]);
        let body = NotePayloadBody {
            title: "Readable title".to_string(),
            content: markdown_note_content("Readable body").expect("content"),
            mentions: Some(vec!["member".to_string()]),
            attachments: Some(vec![FlexibleValue::Map(Vec::new())]),
            client_meta: Some(FlexibleValue::Map(Vec::new())),
        };
        let encrypted = encrypt_note_payload(&build_note_payload_envelope(body, 1), &list_key)
            .expect("encrypted note");
        let now = Utc::now();
        let note = NoteResponse {
            id: Uuid::now_v7(),
            work_list_id: Uuid::now_v7(),
            created_by_membership_id: Uuid::now_v7(),
            title_ciphertext: encrypted.base64.clone(),
            legacy_cbor_fields: Vec::new(),
            payload_ciphertext: encrypted.base64,
            is_private: false,
            note_key_ciphertext: None,
            created_at: now,
            updated_at: now,
        };
        let context = UnlockedWorkListContext {
            work_list: WorkListContext {
                work_list_title: None,
                work_list_timezone: "UTC".to_string(),
                list_key: Some(list_key),
                task_reference_schemes: Vec::new(),
                current_task_reference_scheme_revision: None,
                current_task_reference_scheme_revision_id: None,
                read_error: None,
            },
            data_key: SymmetricKey::new([0x41; KEY_SIZE]),
            membership_id: Uuid::now_v7(),
        };
        let runtime = RuntimeClient::new("https://api.example").expect("runtime");

        let projected = runtime.project_note(note, &context);

        assert_eq!(projected.title.as_deref(), Some("Readable title"));
        assert_eq!(projected.body_markdown.as_deref(), Some("Readable body"));
        assert!(projected.content.is_some());
        assert_eq!(projected.mentions, Some(vec!["member".to_string()]));
        assert!(projected.client_meta.is_some());
        assert!(projected.attachments.is_none());
        assert_eq!(
            projected
                .read_error
                .as_ref()
                .map(|error| error.code.as_str()),
            Some("note_attachments")
        );
    }
}
