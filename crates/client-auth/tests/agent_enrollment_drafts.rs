use std::fs;

use base64::{Engine as _, engine::general_purpose::STANDARD_NO_PAD};
use chrono::{Duration, Utc};
use tempfile::tempdir;
use uuid::Uuid;

use sealtask_client_auth::{
    AgentEnrollmentRegistration, PrepareAgentEnrollmentDraft, SavePendingAgentIdentity,
    configure_local_state, prepare_agent_enrollment_draft, save_pending_agent_identity,
};

const NON_CANONICAL_API_URL: &str = "HTTPS://API.SEALTASK.EXAMPLE:443/";

#[test]
fn enrollment_draft_survives_restart_and_local_persistence_retry() {
    let temporary = tempdir().expect("temporary enrollment registry");
    let config = temporary.path().join("config");
    let repository = temporary.path().join("repository");
    fs::create_dir_all(&repository).expect("repository directory");
    configure_local_state(Some(config), Some("agent-enrollment"))
        .expect("configure isolated profile");

    let work_list_id = Uuid::now_v7();
    let first = prepare_draft(work_list_id, &repository);
    assert!(!first.is_resumed());
    let enrollment_code = first
        .key_material()
        .enrollment_code()
        .expect("derive enrollment code");
    let fingerprint = first.key_material().fingerprint();
    drop(first);

    let resumed = prepare_draft(work_list_id, &repository);
    assert!(resumed.is_resumed());
    assert_eq!(resumed.key_material().fingerprint(), fingerprint);
    assert_eq!(
        resumed
            .key_material()
            .enrollment_code()
            .expect("derive resumed enrollment code"),
        enrollment_code
    );

    let agent_id = Uuid::now_v7();
    let expires_at = Utc::now() + Duration::minutes(15);
    let auth_public_key = STANDARD_NO_PAD.encode(resumed.key_material().auth_public_key());
    let recipient_public_key =
        STANDARD_NO_PAD.encode(resumed.key_material().recipient_public_key());
    let persisted = save_pending_agent_identity(
        SavePendingAgentIdentity {
            agent_id,
            api_url: NON_CANONICAL_API_URL,
            proposed_handle: Some("implementer".to_string()),
            auth_public_key: &auth_public_key,
            recipient_public_key: &recipient_public_key,
            fingerprint: &fingerprint,
            enrollment_expires_at: expires_at,
            work_list_id,
            repository_root: &repository,
        },
        resumed.key_material(),
    )
    .expect("persist recovered identity");
    assert_eq!(persisted.api_url, "https://api.sealtask.example");
    drop(resumed);

    let resumed_after_local_failure = prepare_draft(work_list_id, &repository);
    assert!(resumed_after_local_failure.is_resumed());
    assert_eq!(
        resumed_after_local_failure
            .matching_local_identity()
            .expect("reconcile persisted identity"),
        Some(persisted.clone()),
        "an old draft must resolve to its already-persisted identity without another POST"
    );
    let replayed = save_pending_agent_identity(
        SavePendingAgentIdentity {
            agent_id,
            api_url: NON_CANONICAL_API_URL,
            proposed_handle: Some("implementer".to_string()),
            auth_public_key: &auth_public_key,
            recipient_public_key: &recipient_public_key,
            fingerprint: &fingerprint,
            enrollment_expires_at: expires_at,
            work_list_id,
            repository_root: &repository,
        },
        resumed_after_local_failure.key_material(),
    )
    .expect("replay identical local persistence");
    assert_eq!(replayed, persisted);
    resumed_after_local_failure
        .complete()
        .expect("remove completed enrollment draft");

    let replacement = prepare_draft(work_list_id, &repository);
    assert!(!replacement.is_resumed());
    assert_ne!(replacement.key_material().fingerprint(), fingerprint);
    let replacement_fingerprint = replacement.key_material().fingerprint();
    let replacement_auth = STANDARD_NO_PAD.encode(replacement.key_material().auth_public_key());
    let replacement_recipient =
        STANDARD_NO_PAD.encode(replacement.key_material().recipient_public_key());
    let replacement_agent_id = Uuid::now_v7();
    let replacement_expiry = Utc::now() + Duration::minutes(15);
    let mut replacement = replacement;
    replacement
        .record_registration(AgentEnrollmentRegistration {
            agent_id: replacement_agent_id,
            proposed_handle: Some("implementer".to_string()),
            auth_public_key: replacement_auth.clone(),
            recipient_public_key: replacement_recipient.clone(),
            fingerprint: replacement_fingerprint.clone(),
            expires_at: replacement_expiry,
        })
        .expect("persist registration receipt");
    drop(replacement);

    let resumed_with_receipt = prepare_draft(work_list_id, &repository);
    let receipt = resumed_with_receipt
        .registration()
        .expect("registration receipt survives restart");
    assert_eq!(receipt.agent_id, replacement_agent_id);
    assert_eq!(receipt.auth_public_key, replacement_auth);
    assert_eq!(receipt.recipient_public_key, replacement_recipient);
    assert_eq!(receipt.fingerprint, replacement_fingerprint);
    resumed_with_receipt
        .complete()
        .expect("remove replacement draft and receipt");
}

fn prepare_draft(
    work_list_id: Uuid,
    repository_root: &std::path::Path,
) -> sealtask_client_auth::AgentEnrollmentDraft {
    prepare_agent_enrollment_draft(PrepareAgentEnrollmentDraft {
        api_url: NON_CANONICAL_API_URL,
        proposed_handle: Some("implementer".to_string()),
        work_list_id,
        repository_root,
    })
    .expect("prepare enrollment draft")
}
