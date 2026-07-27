use std::{collections::HashSet, time::Duration};

use sealtask_client_api::note_transport::{
    DeleteNoteResponse, EncodedNoteRequest, EncodedNoteResponse, NoteResponsePayload,
};
use sealtask_client_api::{
    CreateNoteRequest, MAX_NOTE_COLLECTION_ENCODED_BYTES, MAX_NOTE_PAGE_ITEMS, NoteResponse,
    PublicApiClient, UpdateNoteRequest,
};
#[cfg(test)]
use sealtask_client_api::{MAX_NOTE_COLLECTION_ITEMS, MAX_NOTE_COLLECTION_PAGES};
use sealtask_client_core::{HttpFailureKind, PublicError, PublicResult};
use tokio::time::Instant;
use uuid::Uuid;

use crate::blocking_crypto::{BlockingCryptoAdmission, LargePayloadPermit};
use crate::models::AgentNote;
use crate::reconciliation::{
    ReconciliationCause, classify_reconciliation_error, mutation_outcome_is_ambiguous,
    outcome_ambiguous, sanitized_error_cause,
};

use super::super::{RuntimeClient, UnlockedWorkListContext};

pub(super) const MUTATION_RECONCILIATION_TIMEOUT: Duration = Duration::from_secs(5);
const CREATE_RECONCILIATION_INITIAL_BACKOFF: Duration = Duration::from_millis(25);
const CREATE_RECONCILIATION_MAX_BACKOFF: Duration = Duration::from_millis(250);

enum NoteResponseProcessing<T> {
    Decoded(PublicResult<T>),
    LocalFailure(PublicError),
}

async fn process_note_response<M, T, F>(
    blocking_crypto: &BlockingCryptoAdmission,
    payload_permit: LargePayloadPermit,
    response: EncodedNoteResponse<M>,
    process: F,
    failure_message: &'static str,
) -> (LargePayloadPermit, bool, NoteResponseProcessing<T>)
where
    M: NoteResponsePayload + Send + 'static,
    T: Send + 'static,
    F: FnOnce(EncodedNoteResponse<M>) -> PublicResult<T> + Send + 'static,
{
    let commit_confirmed = response.is_success_status();
    let (payload_permit, result) = blocking_crypto
        .run_with_large_payload_preserving(
            payload_permit,
            move || Ok(process(response)),
            failure_message,
        )
        .await;
    let processing = match result {
        Ok(decoded) => NoteResponseProcessing::Decoded(decoded),
        Err(error) => NoteResponseProcessing::LocalFailure(error),
    };
    (payload_permit, commit_confirmed, processing)
}

fn committed_note_processing_failure(
    operation: &str,
    committed_resource: String,
    primary: &PublicError,
    reconciliation: ReconciliationCause,
    description: &str,
) -> PublicError {
    PublicError::committed_but_local_processing_failed(
        operation,
        committed_resource,
        format!(
            "local_failure={}; reconciliation={}; {description}",
            sanitized_error_cause(primary),
            reconciliation.label()
        ),
    )
}

fn note_reconciliation_failure(
    operation: &str,
    committed_resource: String,
    primary: &PublicError,
    commit_confirmed: bool,
    reconciliation: ReconciliationCause,
    description: &str,
) -> PublicError {
    if commit_confirmed {
        committed_note_processing_failure(
            operation,
            committed_resource,
            primary,
            reconciliation,
            description,
        )
    } else {
        outcome_ambiguous(operation, primary, reconciliation, description)
    }
}

impl RuntimeClient {
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn finish_create_note_response_with<F>(
        &self,
        client: &mut PublicApiClient,
        work_list_id: Uuid,
        request: CreateNoteRequest,
        context: &UnlockedWorkListContext,
        response: EncodedNoteResponse<NoteResponse>,
        payload_permit: LargePayloadPermit,
        process: F,
    ) -> PublicResult<AgentNote>
    where
        F: FnOnce(EncodedNoteResponse<NoteResponse>) -> PublicResult<AgentNote> + Send + 'static,
    {
        let (payload_permit, commit_confirmed, processing) = process_note_response(
            &self.blocking_crypto,
            payload_permit,
            response,
            process,
            "note creation response task failed",
        )
        .await;
        match processing {
            NoteResponseProcessing::Decoded(Ok(created)) => Ok(created),
            NoteResponseProcessing::Decoded(Err(primary))
                if commit_confirmed || mutation_outcome_is_ambiguous(&primary) =>
            {
                self.reconcile_create_note(
                    client,
                    work_list_id,
                    request,
                    context,
                    primary,
                    commit_confirmed,
                    MUTATION_RECONCILIATION_TIMEOUT,
                    payload_permit,
                )
                .await
            }
            NoteResponseProcessing::LocalFailure(primary) => {
                self.reconcile_create_note(
                    client,
                    work_list_id,
                    request,
                    context,
                    primary,
                    commit_confirmed,
                    MUTATION_RECONCILIATION_TIMEOUT,
                    payload_permit,
                )
                .await
            }
            NoteResponseProcessing::Decoded(Err(primary)) => Err(primary),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) async fn finish_update_note_response_with<F>(
        &self,
        client: &mut PublicApiClient,
        work_list_id: Uuid,
        note_id: Uuid,
        current: NoteResponse,
        request: UpdateNoteRequest,
        context: &UnlockedWorkListContext,
        response: EncodedNoteResponse<NoteResponse>,
        payload_permit: LargePayloadPermit,
        process: F,
    ) -> PublicResult<AgentNote>
    where
        F: FnOnce(EncodedNoteResponse<NoteResponse>) -> PublicResult<AgentNote> + Send + 'static,
    {
        let (payload_permit, commit_confirmed, processing) = process_note_response(
            &self.blocking_crypto,
            payload_permit,
            response,
            process,
            "note update response task failed",
        )
        .await;
        match processing {
            NoteResponseProcessing::Decoded(Ok(updated)) => Ok(updated),
            NoteResponseProcessing::Decoded(Err(primary))
                if commit_confirmed || mutation_outcome_is_ambiguous(&primary) =>
            {
                self.reconcile_update_note(
                    client,
                    work_list_id,
                    note_id,
                    current,
                    request,
                    context,
                    primary,
                    commit_confirmed,
                    MUTATION_RECONCILIATION_TIMEOUT,
                    payload_permit,
                )
                .await
            }
            NoteResponseProcessing::LocalFailure(primary) => {
                self.reconcile_update_note(
                    client,
                    work_list_id,
                    note_id,
                    current,
                    request,
                    context,
                    primary,
                    commit_confirmed,
                    MUTATION_RECONCILIATION_TIMEOUT,
                    payload_permit,
                )
                .await
            }
            NoteResponseProcessing::Decoded(Err(primary)) => Err(primary),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) async fn finish_delete_note_response_with<F>(
        &self,
        client: &mut PublicApiClient,
        work_list_id: Uuid,
        note_id: Uuid,
        current: NoteResponse,
        response: EncodedNoteResponse<DeleteNoteResponse>,
        payload_permit: LargePayloadPermit,
        process: F,
    ) -> PublicResult<()>
    where
        F: FnOnce(EncodedNoteResponse<DeleteNoteResponse>) -> PublicResult<()> + Send + 'static,
    {
        let (payload_permit, commit_confirmed, processing) = process_note_response(
            &self.blocking_crypto,
            payload_permit,
            response,
            process,
            "note deletion response task failed",
        )
        .await;
        match processing {
            NoteResponseProcessing::Decoded(Ok(())) => Ok(()),
            NoteResponseProcessing::Decoded(Err(primary))
                if commit_confirmed || mutation_outcome_is_ambiguous(&primary) =>
            {
                if commit_confirmed {
                    return Err(committed_note_processing_failure(
                        "note delete",
                        format!("note:{note_id}"),
                        &primary,
                        ReconciliationCause::Projection,
                        "the server confirmed deletion; do not retry the mutation",
                    ));
                }
                reconcile_delete_note(
                    &self.blocking_crypto,
                    client,
                    work_list_id,
                    note_id,
                    current,
                    primary,
                    false,
                    MUTATION_RECONCILIATION_TIMEOUT,
                    payload_permit,
                )
                .await
            }
            NoteResponseProcessing::LocalFailure(primary) => {
                if commit_confirmed {
                    return Err(committed_note_processing_failure(
                        "note delete",
                        format!("note:{note_id}"),
                        &primary,
                        ReconciliationCause::Projection,
                        "the server confirmed deletion; do not retry the mutation",
                    ));
                }
                reconcile_delete_note(
                    &self.blocking_crypto,
                    client,
                    work_list_id,
                    note_id,
                    current,
                    primary,
                    false,
                    MUTATION_RECONCILIATION_TIMEOUT,
                    payload_permit,
                )
                .await
            }
            NoteResponseProcessing::Decoded(Err(primary)) => Err(primary),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) async fn reconcile_create_note(
        &self,
        client: &mut PublicApiClient,
        work_list_id: Uuid,
        request: CreateNoteRequest,
        context: &UnlockedWorkListContext,
        primary: PublicError,
        commit_confirmed: bool,
        timeout: Duration,
        payload_permit: LargePayloadPermit,
    ) -> PublicResult<AgentNote> {
        // The create key is durable on the server, so reconciliation retries
        // the same operation directly. A bounded semantic scan cannot prove
        // absence after an ambiguous commit and can miss an older created row.
        let deadline = Instant::now() + timeout;
        let response = tokio::time::timeout_at(deadline, async {
            let mut payload_permit = payload_permit;
            let mut backoff = CREATE_RECONCILIATION_INITIAL_BACKOFF;
            loop {
                let attempt = request.clone();
                let (next_permit, encoded) = self
                    .blocking_crypto
                    .run_with_large_payload_preserving(
                        payload_permit,
                        move || EncodedNoteRequest::encode(&attempt),
                        "note create reconciliation encoding task failed",
                    )
                    .await;
                payload_permit = next_permit;
                let encoded = encoded?;
                let response = match client.create_note_encoded(work_list_id, encoded).await {
                    Ok(response) => response,
                    Err(error) if create_reconciliation_error_is_retryable(&error) => {
                        if !wait_before_create_reconciliation_retry(&error, backoff, deadline).await
                        {
                            return Err(error);
                        }
                        backoff = backoff
                            .saturating_mul(2)
                            .min(CREATE_RECONCILIATION_MAX_BACKOFF);
                        continue;
                    }
                    Err(error) => return Err(error),
                };
                let runtime = self.clone();
                let context = context.clone();
                let (next_permit, projected) = self
                    .blocking_crypto
                    .run_with_large_payload_preserving(
                        payload_permit,
                        move || {
                            let note = response.decode()?;
                            Ok(runtime.project_note(note, &context))
                        },
                        "note create reconciliation response task failed",
                    )
                    .await;
                payload_permit = next_permit;
                match projected {
                    Ok(note) => return Ok(note),
                    Err(error) if create_reconciliation_error_is_retryable(&error) => {
                        if !wait_before_create_reconciliation_retry(&error, backoff, deadline).await
                        {
                            return Err(error);
                        }
                        backoff = backoff
                            .saturating_mul(2)
                            .min(CREATE_RECONCILIATION_MAX_BACKOFF);
                    }
                    Err(error) => return Err(error),
                }
            }
        })
        .await;
        match response {
            Ok(Ok(created)) => Ok(created),
            Ok(Err(error)) => {
                let cause = classify_reconciliation_error(&error, ReconciliationCause::ApiRead);
                let description = format!(
                    "the durable idempotent retry could not return the created note; follow_up={}",
                    sanitized_error_cause(&error)
                );
                Err(note_reconciliation_failure(
                    "note create",
                    format!("work-list:{work_list_id}"),
                    &primary,
                    commit_confirmed,
                    cause,
                    &description,
                ))
            }
            Err(_) => Err(note_reconciliation_failure(
                "note create",
                format!("work-list:{work_list_id}"),
                &primary,
                commit_confirmed,
                ReconciliationCause::Timeout,
                "the durable idempotent retry timed out before returning the created note",
            )),
        }
    }

    #[cfg(test)]
    pub(super) async fn project_reconciled_create_note_with<F>(
        &self,
        payload_permit: LargePayloadPermit,
        created: NoteResponse,
        project: F,
    ) -> PublicResult<AgentNote>
    where
        F: FnOnce(NoteResponse) -> AgentNote + Send + 'static,
    {
        let created_id = created.id;
        let (_, projected) = self
            .blocking_crypto
            .run_with_large_payload_preserving(
                payload_permit,
                move || Ok(project(created)),
                "reconciled note decryption task failed",
            )
            .await;
        projected.map_err(|failure| {
            committed_note_processing_failure(
                "note create",
                format!("note:{created_id}"),
                &failure,
                ReconciliationCause::Projection,
                "the created note was identified, but its local projection failed; fetch it by ID instead of retrying creation",
            )
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) async fn reconcile_update_note(
        &self,
        client: &mut PublicApiClient,
        work_list_id: Uuid,
        note_id: Uuid,
        current: NoteResponse,
        request: UpdateNoteRequest,
        context: &UnlockedWorkListContext,
        primary: PublicError,
        commit_confirmed: bool,
        timeout: Duration,
        payload_permit: LargePayloadPermit,
    ) -> PublicResult<AgentNote> {
        let runtime = self.clone();
        let context = context.clone();
        let response = tokio::time::timeout(timeout, async {
            let note_response = client.get_note_encoded(work_list_id, note_id).await?;
            self.blocking_crypto
                .run_with_large_payload(
                    payload_permit,
                    move || {
                        let note = note_response.decode()?;
                        if note_matches_update_request(&note, &request) {
                            return Ok(UpdateReconciliation::Requested(Box::new(
                                runtime.project_note(note, &context),
                            )));
                        }
                        if note_matches_snapshot(&note, &current) {
                            Ok(UpdateReconciliation::Prior)
                        } else {
                            Ok(UpdateReconciliation::Divergent)
                        }
                    },
                    "note update reconciliation task failed",
                )
                .await
        })
        .await;
        match response {
            Ok(Ok((_, UpdateReconciliation::Requested(note)))) => Ok(*note),
            Ok(Ok((_, UpdateReconciliation::Prior))) => Err(note_reconciliation_failure(
                "note update",
                format!("note:{note_id}"),
                &primary,
                commit_confirmed,
                ReconciliationCause::Divergent,
                "the follow-up read still shows the prior revision, but the request may execute later",
            )),
            Ok(Ok((_, UpdateReconciliation::Divergent))) => Err(note_reconciliation_failure(
                "note update",
                format!("note:{note_id}"),
                &primary,
                commit_confirmed,
                ReconciliationCause::Divergent,
                "the note now has a divergent revision",
            )),
            Ok(Err(error)) => {
                let cause = classify_reconciliation_error(&error, ReconciliationCause::ApiRead);
                let description = format!(
                    "the follow-up read could not establish which revision is stored; follow_up={}",
                    sanitized_error_cause(&error)
                );
                Err(note_reconciliation_failure(
                    "note update",
                    format!("note:{note_id}"),
                    &primary,
                    commit_confirmed,
                    cause,
                    &description,
                ))
            }
            Err(_) => Err(note_reconciliation_failure(
                "note update",
                format!("note:{note_id}"),
                &primary,
                commit_confirmed,
                ReconciliationCause::Timeout,
                "the follow-up read timed out before it could establish which revision is stored",
            )),
        }
    }
}

enum UpdateReconciliation {
    Requested(Box<AgentNote>),
    Prior,
    Divergent,
}

fn note_matches_update_request(note: &NoteResponse, request: &UpdateNoteRequest) -> bool {
    request
        .title_ciphertext
        .as_ref()
        .is_none_or(|value| note.title_ciphertext == *value)
        && request
            .payload_ciphertext
            .as_ref()
            .is_none_or(|value| note.payload_ciphertext == *value)
        && request
            .is_private
            .is_none_or(|value| note.is_private == value)
        && request
            .note_key_ciphertext
            .as_ref()
            .is_none_or(|value| note.note_key_ciphertext.as_ref() == value.as_ref())
}

fn note_matches_snapshot(note: &NoteResponse, snapshot: &NoteResponse) -> bool {
    note.id == snapshot.id
        && note.work_list_id == snapshot.work_list_id
        && note.created_by_membership_id == snapshot.created_by_membership_id
        && note.title_ciphertext == snapshot.title_ciphertext
        && note.payload_ciphertext == snapshot.payload_ciphertext
        && note.is_private == snapshot.is_private
        && note.note_key_ciphertext == snapshot.note_key_ciphertext
        && note.updated_at == snapshot.updated_at
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn reconcile_delete_note(
    blocking_crypto: &BlockingCryptoAdmission,
    client: &mut PublicApiClient,
    work_list_id: Uuid,
    note_id: Uuid,
    current: NoteResponse,
    primary: PublicError,
    commit_confirmed: bool,
    timeout: Duration,
    payload_permit: LargePayloadPermit,
) -> PublicResult<()> {
    let response = tokio::time::timeout(timeout, async {
        let note_response = client.get_note_encoded(work_list_id, note_id).await?;
        blocking_crypto
            .run_with_large_payload(
                payload_permit,
                move || {
                    let note = note_response.decode()?;
                    Ok(if note_matches_snapshot(&note, &current) {
                        DeleteReconciliation::Prior
                    } else {
                        DeleteReconciliation::Divergent
                    })
                },
                "note deletion reconciliation task failed",
            )
            .await
    })
    .await;
    match response {
        Ok(Err(error))
            if matches!(&error, PublicError::NotFound(_)) || error.http_status() == Some(404) =>
        {
            Err(note_reconciliation_failure(
                "note delete",
                format!("note:{note_id}"),
                &primary,
                commit_confirmed,
                ReconciliationCause::ApiRead,
                "the scoped follow-up lookup cannot distinguish deletion from lost project access",
            ))
        }
        Ok(Ok((_, DeleteReconciliation::Prior))) => Err(note_reconciliation_failure(
            "note delete",
            format!("note:{note_id}"),
            &primary,
            commit_confirmed,
            ReconciliationCause::Divergent,
            "the follow-up read still shows the prior revision, but the request may execute later",
        )),
        Ok(Ok((_, DeleteReconciliation::Divergent))) => Err(note_reconciliation_failure(
            "note delete",
            format!("note:{note_id}"),
            &primary,
            commit_confirmed,
            ReconciliationCause::Divergent,
            "the note now has a divergent revision",
        )),
        Ok(Err(error)) => {
            let cause = classify_reconciliation_error(&error, ReconciliationCause::ApiRead);
            let description = format!(
                "the follow-up read could not establish whether the note still exists; follow_up={}",
                sanitized_error_cause(&error)
            );
            Err(note_reconciliation_failure(
                "note delete",
                format!("note:{note_id}"),
                &primary,
                commit_confirmed,
                cause,
                &description,
            ))
        }
        Err(_) => Err(note_reconciliation_failure(
            "note delete",
            format!("note:{note_id}"),
            &primary,
            commit_confirmed,
            ReconciliationCause::Timeout,
            "the follow-up read timed out before it could establish whether the note still exists",
        )),
    }
}

enum DeleteReconciliation {
    Prior,
    Divergent,
}

fn create_reconciliation_error_is_retryable(error: &PublicError) -> bool {
    match error {
        PublicError::RateLimited(_) | PublicError::RequestTimeout(_) => true,
        PublicError::Transport(_) => true,
        PublicError::Http(failure) => {
            matches!(
                failure.kind(),
                HttpFailureKind::RateLimited
                    | HttpFailureKind::RequestTimeout
                    | HttpFailureKind::Server
            ) || (failure.status() == 409
                && matches!(
                    failure.backend_error_code(),
                    Some("operation_pending" | "idempotency_pending")
                ))
        }
        PublicError::Response {
            kind:
                sealtask_client_core::ResponseFailureKind::BodyRead
                | sealtask_client_core::ResponseFailureKind::BodyTruncated
                | sealtask_client_core::ResponseFailureKind::Transport
                | sealtask_client_core::ResponseFailureKind::JsonMalformed
                | sealtask_client_core::ResponseFailureKind::JsonSchema,
            ..
        } => true,
        _ => false,
    }
}

async fn wait_before_create_reconciliation_retry(
    error: &PublicError,
    local_backoff: Duration,
    deadline: Instant,
) -> bool {
    let delay = error.retry_after().map_or(local_backoff, |server_delay| {
        server_delay.max(local_backoff)
    });
    if delay >= deadline.saturating_duration_since(Instant::now()) {
        return false;
    }
    tokio::time::sleep(delay).await;
    true
}

#[cfg(test)]
pub(super) async fn find_note_in_bounded_pages(
    blocking_crypto: &BlockingCryptoAdmission,
    client: &mut PublicApiClient,
    work_list_id: Uuid,
    predicate: impl Fn(&NoteResponse) -> bool + Send + Sync + 'static,
    mut payload_permit: LargePayloadPermit,
) -> PublicResult<(LargePayloadPermit, Option<NoteResponse>)> {
    let predicate = std::sync::Arc::new(predicate);
    let mut cursor = None;
    let mut seen_cursors = HashSet::new();
    let mut item_count = 0usize;
    let mut encoded_bytes = 0usize;
    for _ in 0..MAX_NOTE_COLLECTION_PAGES {
        let response = client
            .list_notes_page_encoded(work_list_id, cursor.as_deref(), MAX_NOTE_PAGE_ITEMS)
            .await?;
        add_received_note_page_to_encoded_budget(&mut encoded_bytes, response.encoded_len())?;
        let predicate = predicate.clone();
        let (next_permit, page) = blocking_crypto
            .run_with_large_payload(
                payload_permit,
                move || {
                    let page = response.decode()?;
                    validate_note_page_size(page.notes.len())?;
                    let page_was_empty = page.notes.is_empty();
                    let page_item_count = page.notes.len();
                    let matched = page.notes.into_iter().find(|note| predicate(note));
                    Ok((matched, page.next_cursor, page_was_empty, page_item_count))
                },
                "note reconciliation page task failed",
            )
            .await?;
        payload_permit = next_permit;
        let (matched, next_cursor, page_was_empty, page_item_count) = page;
        item_count = item_count
            .checked_add(page_item_count)
            .ok_or_else(|| PublicError::unexpected("note list item count overflowed"))?;
        if item_count > MAX_NOTE_COLLECTION_ITEMS {
            return Err(PublicError::unexpected(format!(
                "note list exceeds the {MAX_NOTE_COLLECTION_ITEMS}-item safety limit"
            )));
        }
        if let Some(note) = matched {
            return Ok((payload_permit, Some(note)));
        }
        match validate_next_note_cursor(next_cursor, page_was_empty, &mut seen_cursors)? {
            Some(next_cursor) => cursor = Some(next_cursor),
            None => return Ok((payload_permit, None)),
        }
    }

    Err(PublicError::unexpected(format!(
        "note list exceeds the {MAX_NOTE_COLLECTION_PAGES}-page safety limit"
    )))
}

pub(super) fn add_received_note_page_to_encoded_budget(
    encoded_bytes: &mut usize,
    received_bytes: usize,
) -> PublicResult<()> {
    add_received_note_page_to_encoded_budget_with_limit(
        encoded_bytes,
        received_bytes,
        MAX_NOTE_COLLECTION_ENCODED_BYTES,
    )
}

pub(super) fn add_received_note_page_to_encoded_budget_with_limit(
    encoded_bytes: &mut usize,
    received_bytes: usize,
    maximum_encoded_bytes: usize,
) -> PublicResult<()> {
    *encoded_bytes = encoded_bytes
        .checked_add(received_bytes)
        .ok_or_else(|| PublicError::unexpected("note collection byte count overflowed"))?;
    if *encoded_bytes > maximum_encoded_bytes {
        return Err(PublicError::unexpected(format!(
            "note list exceeds the {maximum_encoded_bytes}-byte encoded safety limit"
        )));
    }
    Ok(())
}

pub(super) fn validate_note_page_size(page_len: usize) -> PublicResult<()> {
    if page_len > usize::try_from(MAX_NOTE_PAGE_ITEMS).unwrap_or(usize::MAX) {
        return Err(PublicError::unexpected(
            "server returned more notes than the requested page limit",
        ));
    }
    Ok(())
}

pub(super) fn validate_next_note_cursor(
    next_cursor: Option<String>,
    page_was_empty: bool,
    seen_cursors: &mut HashSet<String>,
) -> PublicResult<Option<String>> {
    let Some(next_cursor) = next_cursor else {
        return Ok(None);
    };
    if next_cursor.is_empty() || page_was_empty {
        return Err(PublicError::unexpected(
            "server returned an invalid notes pagination cursor",
        ));
    }
    if !seen_cursors.insert(next_cursor.clone()) {
        return Err(PublicError::unexpected(
            "server repeated a notes pagination cursor",
        ));
    }
    Ok(Some(next_cursor))
}

#[cfg(test)]
mod retry_tests {
    use super::*;

    #[test]
    fn generic_conflicts_are_definitive_and_only_pending_slugs_are_retryable() {
        for code in [None, Some("conflict".to_string())] {
            assert!(!create_reconciliation_error_is_retryable(
                &PublicError::http(409, code, None)
            ));
        }
        for code in ["operation_pending", "idempotency_pending"] {
            assert!(create_reconciliation_error_is_retryable(
                &PublicError::http(409, Some(code.to_string()), None)
            ));
        }
    }

    #[test]
    fn reconciliation_retry_policy_ignores_backend_message_copy() {
        let retryable = PublicError::http(503, Some("unexpected_error".to_string()), None);
        let definitive = PublicError::http(409, Some("conflict".to_string()), None);

        assert!(create_reconciliation_error_is_retryable(&retryable));
        assert!(!create_reconciliation_error_is_retryable(&definitive));
    }

    #[tokio::test(start_paused = true)]
    async fn server_retry_after_prevents_an_early_reconciliation_retry() {
        let error = PublicError::http(
            429,
            Some("rate_limited".to_string()),
            Some(Duration::from_secs(1)),
        );
        let deadline = Instant::now() + Duration::from_secs(5);
        let wait = wait_before_create_reconciliation_retry(
            &error,
            CREATE_RECONCILIATION_INITIAL_BACKOFF,
            deadline,
        );
        tokio::pin!(wait);

        tokio::select! {
            result = &mut wait => panic!("retry wait completed early: {result}"),
            () = tokio::task::yield_now() => {}
        }
        tokio::time::advance(Duration::from_millis(999)).await;
        tokio::select! {
            result = &mut wait => panic!("retry wait ignored Retry-After: {result}"),
            () = tokio::task::yield_now() => {}
        }
        tokio::time::advance(Duration::from_millis(1)).await;

        assert!(wait.await);
    }

    #[tokio::test(start_paused = true)]
    async fn retry_after_beyond_the_absolute_deadline_stops_without_waiting() {
        let start = Instant::now();
        let error =
            PublicError::rate_limited_with_retry_after("retry later", Duration::from_secs(60));

        assert!(
            !wait_before_create_reconciliation_retry(
                &error,
                CREATE_RECONCILIATION_INITIAL_BACKOFF,
                start + MUTATION_RECONCILIATION_TIMEOUT,
            )
            .await
        );
        assert_eq!(Instant::now(), start);
    }
}
