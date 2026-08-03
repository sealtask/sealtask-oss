use std::{fs, io::Read as _, path::Path};

use base64::{Engine as _, engine::general_purpose::STANDARD_NO_PAD};
use serde::Serialize;

use crate::{
    args::AgentsCommand,
    output::{CliResult, OutputFormat, mutation_output_enabled, print_json, print_simple_result},
    resolver::{ProjectLifecycle, resolve_project},
    terminal::with_progress,
};
use sealtask_client_api::{
    AgentApiClient, AgentEnrollmentResponse, AgentResponse, ApproveAgentRequest,
    RegisterAgentEnrollmentRequest,
};
use sealtask_client_auth::{
    AgentEnrollmentRegistration, LocalAgentStatus, PrepareAgentEnrollmentDraft,
    SavePendingAgentIdentity, activate_agent_identity, agent_fingerprint,
    canonicalize_agent_display_name, canonicalize_agent_handle,
    list_agent_identities_with_failures, load_agent_identity, mark_agent_identity_revoked,
    prepare_agent_enrollment_draft, save_pending_agent_identity,
};
use sealtask_client_core::{PublicError, PublicResult};
use sealtask_client_crypto::{
    MAX_AGENT_INSTRUCTIONS_PLAINTEXT_BYTES, derive_agent_enrollment_token,
};
use sealtask_client_runtime::{PrepareAgentApprovalGrant, RuntimeClient};

const MAX_ENROLLMENT_CODE_FILE_BYTES: u64 = 4 * 1024;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentEnrollmentOutput<'a> {
    agent_id: uuid::Uuid,
    enrollment_code: &'a str,
    fingerprint: &'a str,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
    status: &'a str,
    project_id: uuid::Uuid,
    repository_root: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentAssignmentMutationOutput {
    agent_id: uuid::Uuid,
    task_id: uuid::Uuid,
    project_id: uuid::Uuid,
    assigned: bool,
}

pub(crate) async fn run_agents(
    runtime: &RuntimeClient,
    format: OutputFormat,
    command: AgentsCommand,
) -> CliResult<()> {
    match command {
        AgentsCommand::Register {
            proposed_handle,
            project,
            work_list_id,
            repository,
            password_stdin,
        } => {
            let proposed_handle = proposed_handle
                .as_deref()
                .map(canonicalize_agent_handle)
                .transpose()?;
            let project = resolve_project(
                runtime,
                project.as_ref(),
                work_list_id,
                password_stdin,
                ProjectLifecycle::Active,
            )
            .await?;
            let mut draft = prepare_agent_enrollment_draft(PrepareAgentEnrollmentDraft {
                api_url: runtime.api_url(),
                proposed_handle: proposed_handle.clone(),
                work_list_id: project.id,
                repository_root: &repository,
            })?;
            let enrollment_code = draft.key_material().enrollment_code()?;
            let enrollment_token = derive_agent_enrollment_token(&enrollment_code)?;
            if let Some(identity) = draft.matching_local_identity()? {
                print_enrollment(
                    format,
                    AgentEnrollmentOutput {
                        agent_id: identity.agent_id,
                        enrollment_code: &enrollment_code,
                        fingerprint: &identity.fingerprint,
                        expires_at: identity.enrollment_expires_at,
                        status: local_agent_status_label(&identity.status),
                        project_id: identity.project.work_list_id,
                        repository_root: identity.project.repository_root.display().to_string(),
                    },
                )?;
                draft.complete()?;
                return Ok(());
            }
            let api = AgentApiClient::unauthenticated(
                runtime.api_url(),
                runtime.api_transport_options(),
            )?;
            let api = match runtime.api_cancellation_token() {
                Some(token) => api.with_cancellation_token(token),
                None => api,
            };
            let request = RegisterAgentEnrollmentRequest {
                proposed_handle: proposed_handle.clone(),
                auth_public_key: STANDARD_NO_PAD.encode(draft.key_material().auth_public_key()),
                recipient_public_key: STANDARD_NO_PAD
                    .encode(draft.key_material().recipient_public_key()),
                enrollment_token,
            };
            let enrollment = if let Some(registration) = draft.registration().cloned() {
                AgentEnrollmentResponse {
                    agent_id: registration.agent_id,
                    proposed_handle: registration.proposed_handle,
                    auth_public_key: registration.auth_public_key,
                    recipient_public_key: registration.recipient_public_key,
                    fingerprint: registration.fingerprint,
                    expires_at: registration.expires_at,
                }
            } else {
                let response = if draft.is_resumed() {
                    with_progress(
                        "Resuming agent enrollment…",
                        api.resume_or_register_enrollment(&request),
                    )
                    .await?
                } else {
                    with_progress(
                        "Registering agent identity…",
                        api.register_enrollment(&request),
                    )
                    .await?
                };
                let enrollment = response.enrollment.clone();
                draft.record_registration(AgentEnrollmentRegistration {
                    agent_id: enrollment.agent_id,
                    proposed_handle: enrollment.proposed_handle.clone(),
                    auth_public_key: enrollment.auth_public_key.clone(),
                    recipient_public_key: enrollment.recipient_public_key.clone(),
                    fingerprint: enrollment.fingerprint.clone(),
                    expires_at: enrollment.expires_at,
                })?;
                enrollment
            };
            if enrollment.proposed_handle != proposed_handle {
                return Err(PublicError::crypto(
                    "agent enrollment response changed the requested identity metadata",
                )
                .into());
            }
            let identity = persist_pending_identity(
                runtime,
                project.id,
                &repository,
                proposed_handle,
                &enrollment,
                draft.key_material(),
            )?;
            print_enrollment(
                format,
                AgentEnrollmentOutput {
                    agent_id: enrollment.agent_id,
                    enrollment_code: &enrollment_code,
                    fingerprint: &enrollment.fingerprint,
                    expires_at: Some(enrollment.expires_at),
                    status: "pending",
                    project_id: identity.project.work_list_id,
                    repository_root: identity.project.repository_root.display().to_string(),
                },
            )?;
            draft.complete()?;
            Ok(())
        }
        AgentsCommand::Approve {
            enrollment_code_file,
            fingerprint,
            handle,
            display_name,
            instructions_file,
            project,
            work_list_id,
            password_stdin,
        } => {
            let code_uses_stdin = enrollment_code_file == Path::new("-");
            let instructions_use_stdin = instructions_file == Path::new("-");
            if (password_stdin && (code_uses_stdin || instructions_use_stdin))
                || (code_uses_stdin && instructions_use_stdin)
            {
                return Err(PublicError::validation(
                    "agent approval can consume stdin for only one input; use files for the enrollment code and instructions when --password-stdin is set",
                )
                .into());
            }
            let enrollment_code = read_utf8_input(
                &enrollment_code_file,
                MAX_ENROLLMENT_CODE_FILE_BYTES,
                "agent enrollment code",
            )?;
            let enrollment_code = enrollment_code.trim();
            if enrollment_code.is_empty() {
                return Err(
                    PublicError::validation("agent enrollment code cannot be empty").into(),
                );
            }
            let enrollment_token = derive_agent_enrollment_token(enrollment_code)?;
            let instructions = read_utf8_input(
                &instructions_file,
                MAX_AGENT_INSTRUCTIONS_PLAINTEXT_BYTES as u64,
                "agent instructions",
            )?;
            if instructions.trim().is_empty() {
                return Err(PublicError::validation("agent instructions cannot be empty").into());
            }
            let api = AgentApiClient::unauthenticated(
                runtime.api_url(),
                runtime.api_transport_options(),
            )?;
            let enrollment = with_progress(
                "Looking up agent enrollment…",
                api.lookup_enrollment(&enrollment_token),
            )
            .await?;
            let auth_public_key =
                decode_agent_public_key("authentication", &enrollment.auth_public_key)?;
            let recipient_public_key =
                decode_agent_public_key("recipient", &enrollment.recipient_public_key)?;
            let computed_fingerprint = agent_fingerprint(&auth_public_key, &recipient_public_key);
            if enrollment.fingerprint != computed_fingerprint
                || computed_fingerprint != fingerprint.trim()
            {
                return Err(PublicError::validation(
                    "the supplied fingerprint does not match this enrollment",
                )
                .into());
            }
            let project = resolve_project(
                runtime,
                project.as_ref(),
                work_list_id,
                password_stdin,
                ProjectLifecycle::Active,
            )
            .await?;
            let handle = canonicalize_agent_handle(&handle)?;
            let display_name =
                canonicalize_agent_display_name(display_name.as_deref().unwrap_or(&handle))?;
            let grant = with_progress(
                "Encrypting project grant and instructions…",
                runtime.prepare_agent_approval_grant(PrepareAgentApprovalGrant {
                    enrollment_code,
                    agent_id: enrollment.agent_id,
                    auth_public_key: &enrollment.auth_public_key,
                    recipient_public_key: &enrollment.recipient_public_key,
                    work_list_id: project.id,
                    handle: &handle,
                    display_name: &display_name,
                    instructions: instructions.as_bytes(),
                    instructions_revision: 1,
                    password_stdin,
                }),
            )
            .await?;
            let instructions_revision = grant.instructions_revision;
            let mut client = runtime.authenticated_api_client()?;
            let approved = with_progress(
                "Approving agent identity…",
                client.approve_agent(&ApproveAgentRequest {
                    enrollment_token,
                    fingerprint: computed_fingerprint.clone(),
                    handle: handle.clone(),
                    display_name: display_name.clone(),
                    work_list_id: project.id,
                    permission_preset: "assigned_task_worker".to_string(),
                    key_ciphertext: grant.key_ciphertext,
                    instructions_ciphertext: grant.instructions_ciphertext,
                    grant_signature: grant.grant_signature,
                    instructions_revision: grant.instructions_revision,
                }),
            )
            .await?;
            validate_approved_agent_response(
                &approved,
                &enrollment,
                &computed_fingerprint,
                &handle,
                &display_name,
                project.id,
                instructions_revision,
            )?;
            if load_agent_identity(enrollment.agent_id)?.is_some() {
                activate_agent_identity(
                    enrollment.agent_id,
                    handle,
                    display_name,
                    project.id,
                    instructions_revision,
                )?;
            }
            print_agent_mutation(format, &approved, "Agent approved.")
        }
        AgentsCommand::List { local } => {
            if local {
                let listing = list_agent_identities_with_failures()?;
                for failure in &listing.failures {
                    eprintln!(
                        "sealtask: warning: skipped local agent {}: {}",
                        failure.agent_id, failure.message
                    );
                }
                let identities = listing.identities;
                match format {
                    OutputFormat::Json | OutputFormat::JsonPretty | OutputFormat::Jsonl => {
                        print_json(
                            &identities,
                            format,
                            "serializing local agents should succeed",
                        )
                    }
                    OutputFormat::Table => {
                        if identities.is_empty() {
                            println!("No local agent identities.");
                        } else {
                            for identity in identities {
                                let status = match identity.status {
                                    LocalAgentStatus::Pending => "pending",
                                    LocalAgentStatus::Expired => "expired",
                                    LocalAgentStatus::Active => "active",
                                    LocalAgentStatus::Revoked => "revoked",
                                };
                                println!(
                                    "{}\t{}\t{}\t{}",
                                    identity.agent_id,
                                    identity.handle.as_deref().unwrap_or("-"),
                                    status,
                                    identity.project.repository_root.display()
                                );
                            }
                        }
                        Ok(())
                    }
                }
            } else {
                let mut client = runtime.authenticated_api_client()?;
                let agents = with_progress("Loading agents…", client.list_agents()).await?;
                print_agents(format, &agents)
            }
        }
        AgentsCommand::Revoke { agent_id, reason } => {
            let mut client = runtime.authenticated_api_client()?;
            let revoked = with_progress(
                "Revoking agent identity…",
                client.revoke_agent(agent_id, reason),
            )
            .await?;
            if load_agent_identity(agent_id)?.is_some() {
                mark_agent_identity_revoked(agent_id)?;
            }
            print_agent_mutation(format, &revoked, "Agent revoked.")
        }
        AgentsCommand::Assign {
            agent_id,
            task_id,
            project,
            work_list_id,
            password_stdin,
        } => {
            let project = resolve_project(
                runtime,
                project.as_ref(),
                work_list_id,
                password_stdin,
                ProjectLifecycle::Active,
            )
            .await?;
            let mut client = runtime.authenticated_api_client()?;
            with_progress(
                "Assigning task to agent…",
                client.assign_agent_to_task(project.id, task_id, agent_id),
            )
            .await?;
            print_assignment_mutation(format, agent_id, task_id, project.id, true)
        }
        AgentsCommand::Unassign {
            agent_id,
            task_id,
            project,
            work_list_id,
            password_stdin,
        } => {
            let project = resolve_project(
                runtime,
                project.as_ref(),
                work_list_id,
                password_stdin,
                ProjectLifecycle::Active,
            )
            .await?;
            let mut client = runtime.authenticated_api_client()?;
            with_progress(
                "Removing agent assignment…",
                client.unassign_agent_from_task(project.id, task_id, agent_id),
            )
            .await?;
            print_assignment_mutation(format, agent_id, task_id, project.id, false)
        }
    }
}

fn decode_agent_public_key(field: &str, value: &str) -> PublicResult<[u8; 32]> {
    let bytes = STANDARD_NO_PAD
        .decode(value.trim())
        .map_err(|_| PublicError::validation(format!("invalid agent {field} public key")))?;
    bytes
        .try_into()
        .map_err(|_| PublicError::validation(format!("invalid agent {field} public key length")))
}

fn validate_approved_agent_response(
    approved: &AgentResponse,
    enrollment: &AgentEnrollmentResponse,
    expected_fingerprint: &str,
    expected_handle: &str,
    expected_display_name: &str,
    expected_work_list_id: uuid::Uuid,
    expected_instructions_revision: i64,
) -> PublicResult<()> {
    let grant = approved.grant.as_ref().ok_or_else(|| {
        PublicError::crypto("agent approval response omitted the authenticated project grant")
    })?;
    let approved_auth_public_key =
        decode_agent_public_key("authentication", &approved.auth_public_key)?;
    let enrollment_auth_public_key =
        decode_agent_public_key("authentication", &enrollment.auth_public_key)?;
    let approved_recipient_public_key =
        decode_agent_public_key("recipient", &approved.recipient_public_key)?;
    let enrollment_recipient_public_key =
        decode_agent_public_key("recipient", &enrollment.recipient_public_key)?;
    if approved.id != enrollment.agent_id
        || approved.owner_user_id.is_none()
        || approved.status != "active"
        || approved.handle.as_deref() != Some(expected_handle)
        || approved.display_name.as_deref() != Some(expected_display_name)
        || approved.fingerprint != expected_fingerprint
        || approved_auth_public_key != enrollment_auth_public_key
        || approved_recipient_public_key != enrollment_recipient_public_key
        || grant.work_list_id != expected_work_list_id
        || grant.permission_preset != "assigned_task_worker"
        || grant.instructions_revision != expected_instructions_revision
    {
        return Err(PublicError::crypto(
            "agent approval response does not match the owner-authenticated identity and grant",
        ));
    }
    Ok(())
}

fn persist_pending_identity(
    runtime: &RuntimeClient,
    work_list_id: uuid::Uuid,
    repository: &Path,
    proposed_handle: Option<String>,
    enrollment: &AgentEnrollmentResponse,
    keys: &sealtask_client_auth::AgentKeyMaterial,
) -> PublicResult<sealtask_client_auth::AgentIdentity> {
    save_pending_agent_identity(
        SavePendingAgentIdentity {
            agent_id: enrollment.agent_id,
            api_url: runtime.api_url(),
            proposed_handle,
            auth_public_key: &enrollment.auth_public_key,
            recipient_public_key: &enrollment.recipient_public_key,
            fingerprint: &enrollment.fingerprint,
            enrollment_expires_at: enrollment.expires_at,
            work_list_id,
            repository_root: repository,
        },
        keys,
    )
}

fn read_utf8_input(
    path: &Path,
    max_bytes: u64,
    label: &str,
) -> PublicResult<zeroize::Zeroizing<String>> {
    let mut bytes = Vec::new();
    if path == Path::new("-") {
        std::io::stdin()
            .take(max_bytes + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| PublicError::unexpected(format!("failed to read {label}: {error}")))?;
    } else {
        let metadata = fs::metadata(path).map_err(|error| {
            PublicError::validation(format!("failed to inspect {label} file: {error}"))
        })?;
        if !metadata.is_file() || metadata.len() > max_bytes {
            return Err(PublicError::validation(format!(
                "invalid {label} file size or type"
            )));
        }
        bytes = fs::read(path).map_err(|error| {
            PublicError::validation(format!("failed to read {label} file: {error}"))
        })?;
    }
    if bytes.len() as u64 > max_bytes {
        return Err(PublicError::validation(format!(
            "{label} exceeds the supported size"
        )));
    }
    let bytes = zeroize::Zeroizing::new(bytes);
    std::str::from_utf8(&bytes)
        .map(|value| zeroize::Zeroizing::new(value.to_string()))
        .map_err(|_| PublicError::validation(format!("{label} must be valid UTF-8")))
}

fn print_enrollment(format: OutputFormat, output: AgentEnrollmentOutput<'_>) -> CliResult<()> {
    match format {
        OutputFormat::Json | OutputFormat::JsonPretty | OutputFormat::Jsonl => print_json(
            &output,
            format,
            "serializing agent enrollment should succeed",
        ),
        OutputFormat::Table => {
            println!("Agent ID: {}", output.agent_id);
            println!("Status: {}", output.status);
            println!("Enrollment code: {}", output.enrollment_code);
            println!("Fingerprint: {}", output.fingerprint);
            if let Some(expires_at) = output.expires_at {
                println!("Expires: {}", expires_at.to_rfc3339());
            }
            println!("Repository: {}", output.repository_root);
            println!(
                "This is the identity's long-lived grant-signing secret. Protect it for the identity's lifetime and confirm the fingerprint out of band."
            );
            Ok(())
        }
    }
}

fn local_agent_status_label(status: &LocalAgentStatus) -> &'static str {
    match status {
        LocalAgentStatus::Pending => "pending",
        LocalAgentStatus::Expired => "expired",
        LocalAgentStatus::Active => "active",
        LocalAgentStatus::Revoked => "revoked",
    }
}

fn print_agents(format: OutputFormat, agents: &[AgentResponse]) -> CliResult<()> {
    match format {
        OutputFormat::Json | OutputFormat::JsonPretty | OutputFormat::Jsonl => {
            print_json(agents, format, "serializing agents should succeed")
        }
        OutputFormat::Table => {
            if agents.is_empty() {
                println!("No agent identities.");
            } else {
                for agent in agents {
                    println!(
                        "{}\t{}\t{}\t{}",
                        agent.id,
                        agent.handle.as_deref().unwrap_or("-"),
                        agent.status,
                        agent
                            .grant
                            .as_ref()
                            .map(|grant| grant.work_list_id.to_string())
                            .unwrap_or_else(|| "-".to_string())
                    );
                }
            }
            Ok(())
        }
    }
}

fn print_agent_mutation(
    format: OutputFormat,
    agent: &AgentResponse,
    message: &str,
) -> CliResult<()> {
    if !mutation_output_enabled(format) {
        return Ok(());
    }
    match format {
        OutputFormat::Json | OutputFormat::JsonPretty | OutputFormat::Jsonl => {
            print_json(agent, format, "serializing agent should succeed")
        }
        OutputFormat::Table => {
            print_simple_result(format, agent, "serializing agent should succeed", message)
        }
    }
}

fn print_assignment_mutation(
    format: OutputFormat,
    agent_id: uuid::Uuid,
    task_id: uuid::Uuid,
    project_id: uuid::Uuid,
    assigned: bool,
) -> CliResult<()> {
    if !mutation_output_enabled(format) {
        return Ok(());
    }
    print_simple_result(
        format,
        &AgentAssignmentMutationOutput {
            agent_id,
            task_id,
            project_id,
            assigned,
        },
        "serializing agent assignment should succeed",
        if assigned {
            "Task assigned to agent."
        } else {
            "Agent assignment removed."
        },
    )
}
