use std::{fmt, time::Duration};

use chrono::{DateTime, Utc};
use reqwest::Method;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};

use sealtask_client_core::{PublicError, PublicResult, ResponseFailureKind, TransportFailureKind};

use crate::{
    ApiCancellationToken, ApiTransportOptions, MAX_DETAIL_RESPONSE_BYTES, PublicApiClient,
    build_control_plane_http_client, decode_bounded_json, map_api_error_with_retry_after,
    parse_retry_after,
    transport::{ActiveRequestBoundaryGuard, RequestSemantics},
};

const MAX_AGENT_SMALL_RESPONSE_BYTES: usize = 128 * 1024;
const MAX_AGENT_COLLECTION_RESPONSE_BYTES: usize = 1024 * 1024;
// A claim contains the assigned task ciphertext. Match the normal task-detail
// budget so valid, larger task bodies remain runnable.
const MAX_AGENT_CLAIM_RESPONSE_BYTES: usize = MAX_DETAIL_RESPONSE_BYTES;
const AGENT_TOKEN_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const AGENT_HEARTBEAT_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const AGENT_FINISH_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const AGENT_LIST_PAGE_SIZE: usize = 100;
const MAX_AGENT_ASSIGNMENT_PAGE_SIZE: u16 = 100;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentEnrollmentResponse {
    pub agent_id: Uuid,
    pub proposed_handle: Option<String>,
    pub auth_public_key: String,
    pub recipient_public_key: String,
    pub fingerprint: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisteredAgentEnrollmentResponse {
    #[serde(flatten)]
    pub enrollment: AgentEnrollmentResponse,
    #[serde(skip)]
    mutation_guard: Option<ActiveRequestBoundaryGuard>,
}

impl fmt::Debug for RegisteredAgentEnrollmentResponse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RegisteredAgentEnrollmentResponse")
            .field("enrollment", &self.enrollment)
            .field("mutation_guard", &self.mutation_guard.is_some())
            .finish()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentGrantResponse {
    pub work_list_id: Uuid,
    pub permission_preset: String,
    pub instructions_revision: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentResponse {
    pub id: Uuid,
    pub owner_user_id: Option<Uuid>,
    pub handle: Option<String>,
    pub display_name: Option<String>,
    pub proposed_handle: Option<String>,
    pub auth_public_key: String,
    pub recipient_public_key: String,
    pub fingerprint: String,
    pub status: String,
    pub revoked_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub activated_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub grant: Option<AgentGrantResponse>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTokenResponse {
    pub access_token: String,
    pub expires_in: u64,
    pub token_type: String,
    pub agent_id: Uuid,
}

impl Drop for AgentTokenResponse {
    fn drop(&mut self) {
        self.access_token.zeroize();
    }
}

impl fmt::Debug for AgentTokenResponse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AgentTokenResponse")
            .field("access_token", &"<redacted>")
            .field("expires_in", &self.expires_in)
            .field("token_type", &self.token_type)
            .field("agent_id", &self.agent_id)
            .finish()
    }
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMeResponse {
    pub id: Uuid,
    pub owner_user_id: Uuid,
    pub work_list_id: Uuid,
    pub permission_preset: String,
    pub instructions_revision: i64,
    pub handle: String,
    pub display_name: String,
    pub key_ciphertext: String,
    pub instructions_ciphertext: String,
    pub grant_signature: String,
}

impl fmt::Debug for AgentMeResponse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AgentMeResponse")
            .field("id", &self.id)
            .field("owner_user_id", &self.owner_user_id)
            .field("work_list_id", &self.work_list_id)
            .field("permission_preset", &self.permission_preset)
            .field("instructions_revision", &self.instructions_revision)
            .field("handle", &self.handle)
            .field("display_name", &self.display_name)
            .field("key_ciphertext", &"<redacted>")
            .field("instructions_ciphertext", &"<redacted>")
            .field("grant_signature", &"<redacted>")
            .finish()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentAssignmentResponse {
    pub id: Uuid,
    pub task_id: Uuid,
    pub work_list_id: Uuid,
    pub status: String,
    pub status_revision: i64,
    pub reference_number: Option<i64>,
    pub priority: Option<i16>,
    pub due_at: Option<DateTime<Utc>>,
    pub start_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub is_completed: bool,
    pub task_updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub latest_run_id: Option<Uuid>,
    pub latest_run_status: Option<String>,
    pub latest_run_lease_expires_at: Option<DateTime<Utc>>,
    pub claimable: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRunResponse {
    pub id: Uuid,
    pub delegation_id: Uuid,
    pub work_list_id: Uuid,
    pub task_id: Uuid,
    pub assignment_revision: i64,
    pub attempt: i32,
    pub runner_instance_id: Uuid,
    pub source_revision: Option<String>,
    pub instructions_revision: i64,
    pub lease_expires_at: DateTime<Utc>,
    pub status: String,
    pub version: i64,
    pub failure_code: Option<String>,
    pub claimed_at: DateTime<Utc>,
    pub running_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentClaimResponse {
    pub run: AgentRunResponse,
    pub lease_token: String,
    pub workspace_id: Uuid,
    pub task_title_ciphertext: String,
    pub task_payload_ciphertext: String,
    pub task_updated_at: DateTime<Utc>,
    pub key_ciphertext: String,
    pub instructions_ciphertext: String,
    pub grant_signature: String,
    pub permission_preset: String,
}

impl Drop for AgentClaimResponse {
    fn drop(&mut self) {
        self.lease_token.zeroize();
    }
}

impl fmt::Debug for AgentClaimResponse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AgentClaimResponse")
            .field("run", &self.run)
            .field("lease_token", &"<redacted>")
            .field("workspace_id", &self.workspace_id)
            .field("task_title_ciphertext", &"<redacted>")
            .field("task_payload_ciphertext", &"<redacted>")
            .field("task_updated_at", &self.task_updated_at)
            .field("key_ciphertext", &"<redacted>")
            .field("instructions_ciphertext", &"<redacted>")
            .field("grant_signature", &"<redacted>")
            .field("permission_preset", &self.permission_preset)
            .finish()
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterAgentEnrollmentRequest {
    pub proposed_handle: Option<String>,
    pub auth_public_key: String,
    pub recipient_public_key: String,
    pub enrollment_token: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LookupAgentEnrollmentRequest<'a> {
    pub enrollment_token: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApproveAgentRequest {
    pub enrollment_token: String,
    pub fingerprint: String,
    pub handle: String,
    pub display_name: String,
    pub work_list_id: Uuid,
    pub permission_preset: String,
    pub key_ciphertext: String,
    pub instructions_ciphertext: String,
    pub grant_signature: String,
    pub instructions_revision: i64,
}

impl Drop for ApproveAgentRequest {
    fn drop(&mut self) {
        self.enrollment_token.zeroize();
        self.key_ciphertext.zeroize();
        self.instructions_ciphertext.zeroize();
    }
}

impl fmt::Debug for ApproveAgentRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ApproveAgentRequest")
            .field("enrollment_token", &"<redacted>")
            .field("fingerprint", &self.fingerprint)
            .field("handle", &self.handle)
            .field("display_name", &self.display_name)
            .field("work_list_id", &self.work_list_id)
            .field("permission_preset", &self.permission_preset)
            .field("key_ciphertext", &"<redacted>")
            .field("instructions_ciphertext", &"<redacted>")
            .field("grant_signature", &"<redacted>")
            .field("instructions_revision", &self.instructions_revision)
            .finish()
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTokenRequest {
    pub assertion: String,
}

impl Drop for AgentTokenRequest {
    fn drop(&mut self) {
        self.assertion.zeroize();
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimAgentAssignmentRequest {
    pub runner_instance_id: Uuid,
    pub source_revision: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRunHeartbeatRequest<'a> {
    pub lease_token: &'a str,
    pub expected_version: i64,
    pub heartbeat_id: Uuid,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinishAgentRunRequest<'a> {
    pub lease_token: &'a str,
    pub expected_version: i64,
    pub completion_id: Uuid,
    pub status: &'a str,
    pub result_ciphertext: Option<&'a str>,
    pub failure_code: Option<&'a str>,
}

pub struct AgentApiClient {
    client: reqwest::Client,
    base_url: String,
    access_token: Option<Zeroizing<String>>,
    cancellation_token: Option<ApiCancellationToken>,
}

impl fmt::Debug for AgentApiClient {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AgentApiClient")
            .field("base_url", &self.base_url)
            .field(
                "access_token",
                &self.access_token.as_ref().map(|_| "<redacted>"),
            )
            .field("cancellation_token", &self.cancellation_token.is_some())
            .finish()
    }
}

impl Drop for AgentApiClient {
    fn drop(&mut self) {
        if let Some(token) = self.access_token.as_mut() {
            token.zeroize();
        }
    }
}

impl AgentApiClient {
    pub fn unauthenticated(
        base_url: impl Into<String>,
        options: ApiTransportOptions,
    ) -> PublicResult<Self> {
        Ok(Self {
            client: build_control_plane_http_client(options)?,
            base_url: normalize_base_url(base_url.into()),
            access_token: None,
            cancellation_token: None,
        })
    }

    pub fn authenticated(
        base_url: impl Into<String>,
        access_token: String,
        options: ApiTransportOptions,
    ) -> PublicResult<Self> {
        if access_token.trim().is_empty() {
            return Err(PublicError::validation(
                "agent access token cannot be empty",
            ));
        }
        Ok(Self {
            client: build_control_plane_http_client(options)?,
            base_url: normalize_base_url(base_url.into()),
            access_token: Some(Zeroizing::new(access_token)),
            cancellation_token: None,
        })
    }

    #[must_use]
    pub fn with_cancellation_token(mut self, cancellation_token: ApiCancellationToken) -> Self {
        self.cancellation_token = Some(cancellation_token);
        self
    }

    pub async fn register_enrollment(
        &self,
        request: &RegisterAgentEnrollmentRequest,
    ) -> PublicResult<RegisteredAgentEnrollmentResponse> {
        let mutation_guard = self.cancellation_token.as_ref().and_then(|token| {
            token.enter_mutation_request(RequestSemantics::DurableMutationNoReplay {
                operation: "register agent enrollment",
            })
        });
        self.ensure_not_cancelled()?;
        let response: PublicResult<RegisteredAgentEnrollmentResponse> = self
            .send_json(
                Method::POST,
                "/agent-enrollments",
                Some(request),
                MAX_AGENT_SMALL_RESPONSE_BYTES,
            )
            .await;
        let mut response = match response {
            Ok(response) => response,
            Err(error) if registration_may_have_committed(&error) => {
                let enrollment = self.lookup_enrollment(&request.enrollment_token).await?;
                RegisteredAgentEnrollmentResponse {
                    enrollment,
                    mutation_guard: None,
                }
            }
            Err(error) => return Err(error),
        };
        validate_registered_enrollment(&response.enrollment, request)?;
        response.mutation_guard = mutation_guard;
        Ok(response)
    }

    pub async fn resume_or_register_enrollment(
        &self,
        request: &RegisterAgentEnrollmentRequest,
    ) -> PublicResult<RegisteredAgentEnrollmentResponse> {
        match self.lookup_enrollment(&request.enrollment_token).await {
            Ok(enrollment) => {
                validate_registered_enrollment(&enrollment, request)?;
                Ok(RegisteredAgentEnrollmentResponse {
                    enrollment,
                    mutation_guard: None,
                })
            }
            Err(error) if error.http_status() == Some(404) => {
                self.register_enrollment(request).await
            }
            Err(error) => Err(error),
        }
    }

    pub async fn lookup_enrollment(
        &self,
        enrollment_token: &str,
    ) -> PublicResult<AgentEnrollmentResponse> {
        self.send_json(
            Method::POST,
            "/agent-enrollments/lookup",
            Some(&LookupAgentEnrollmentRequest { enrollment_token }),
            MAX_AGENT_SMALL_RESPONSE_BYTES,
        )
        .await
    }

    pub async fn mint_token(&self, assertion: String) -> PublicResult<AgentTokenResponse> {
        self.send_json_with_timeout(
            Method::POST,
            "/auth/agents/token",
            Some(&AgentTokenRequest { assertion }),
            MAX_AGENT_SMALL_RESPONSE_BYTES,
            Some(AGENT_TOKEN_REQUEST_TIMEOUT),
        )
        .await
    }

    pub async fn me(&self) -> PublicResult<AgentMeResponse> {
        self.send_json::<AgentMeResponse, serde_json::Value>(
            Method::GET,
            "/agent/me",
            None,
            MAX_AGENT_CLAIM_RESPONSE_BYTES,
        )
        .await
    }

    pub async fn list_assignments(
        &self,
        after: Option<Uuid>,
        limit: u16,
    ) -> PublicResult<Vec<AgentAssignmentResponse>> {
        if limit == 0 || limit > MAX_AGENT_ASSIGNMENT_PAGE_SIZE {
            return Err(PublicError::validation(
                "agent assignment page size must be between 1 and 100",
            ));
        }
        let path = after.map_or_else(
            || format!("/agent/me/assignments?limit={limit}"),
            |cursor| format!("/agent/me/assignments?limit={limit}&after={cursor}"),
        );
        let assignments = self
            .send_json::<Vec<AgentAssignmentResponse>, serde_json::Value>(
                Method::GET,
                &path,
                None,
                MAX_AGENT_COLLECTION_RESPONSE_BYTES,
            )
            .await?;
        if assignments.len() > usize::from(limit) {
            return Err(PublicError::response(
                ResponseFailureKind::JsonSchema,
                "agent API returned an oversized assignment page",
            ));
        }
        Ok(assignments)
    }

    pub async fn next_assignment(&self) -> PublicResult<Option<AgentAssignmentResponse>> {
        self.send_json::<Option<AgentAssignmentResponse>, serde_json::Value>(
            Method::GET,
            "/agent/me/assignments/next",
            None,
            MAX_AGENT_SMALL_RESPONSE_BYTES,
        )
        .await
    }

    pub async fn claim_assignment(
        &self,
        delegation_id: Uuid,
        request: &ClaimAgentAssignmentRequest,
    ) -> PublicResult<AgentClaimResponse> {
        self.send_json(
            Method::POST,
            &format!("/agent/me/assignments/{delegation_id}/claim"),
            Some(request),
            MAX_AGENT_CLAIM_RESPONSE_BYTES,
        )
        .await
    }

    pub async fn heartbeat_run(
        &self,
        run_id: Uuid,
        request: &AgentRunHeartbeatRequest<'_>,
    ) -> PublicResult<AgentRunResponse> {
        self.send_json_with_timeout(
            Method::POST,
            &format!("/agent/me/runs/{run_id}/heartbeat"),
            Some(request),
            MAX_AGENT_SMALL_RESPONSE_BYTES,
            Some(AGENT_HEARTBEAT_REQUEST_TIMEOUT),
        )
        .await
    }

    pub async fn finish_run(
        &self,
        run_id: Uuid,
        request: &FinishAgentRunRequest<'_>,
    ) -> PublicResult<AgentRunResponse> {
        self.send_json_with_timeout(
            Method::POST,
            &format!("/agent/me/runs/{run_id}/finish"),
            Some(request),
            MAX_AGENT_SMALL_RESPONSE_BYTES,
            Some(AGENT_FINISH_REQUEST_TIMEOUT),
        )
        .await
    }

    async fn send_json<T, B>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
        max_response_bytes: usize,
    ) -> PublicResult<T>
    where
        T: DeserializeOwned,
        B: Serialize + ?Sized,
    {
        self.send_json_with_timeout(method, path, body, max_response_bytes, None)
            .await
    }

    async fn send_json_with_timeout<T, B>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
        max_response_bytes: usize,
        timeout: Option<Duration>,
    ) -> PublicResult<T>
    where
        T: DeserializeOwned,
        B: Serialize + ?Sized,
    {
        let mut request = self.client.request(
            method,
            format!("{}{}", self.base_url.trim_end_matches('/'), path),
        );
        if let Some(timeout) = timeout {
            request = request.timeout(timeout);
        }
        if let Some(token) = self.access_token.as_ref() {
            request = request.bearer_auth(token.as_str());
        }
        if let Some(body) = body {
            request = request.json(body);
        }
        let mut response = request.send().await.map_err(map_transport_error)?;
        let status = response.status();
        let retry_after = parse_retry_after(response.headers());
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| {
            PublicError::response(
                ResponseFailureKind::BodyRead,
                "failed to read agent API response",
            )
        })? {
            if bytes.len().saturating_add(chunk.len()) > max_response_bytes {
                return Err(PublicError::response(
                    ResponseFailureKind::BodyTooLarge,
                    "agent API response exceeds the supported size",
                ));
            }
            bytes.extend_from_slice(&chunk);
        }
        if !status.is_success() {
            return Err(map_api_error_with_retry_after(
                status.as_u16(),
                &String::from_utf8_lossy(&bytes),
                path,
                retry_after,
            ));
        }
        decode_bounded_json(&bytes)
    }

    fn ensure_not_cancelled(&self) -> PublicResult<()> {
        if self
            .cancellation_token
            .as_ref()
            .is_some_and(ApiCancellationToken::is_cancelled)
        {
            return Err(PublicError::cancelled(
                "agent enrollment cancelled before the registration was sent",
            ));
        }
        Ok(())
    }
}

impl PublicApiClient {
    pub async fn lookup_agent_enrollment(
        &self,
        enrollment_token: &str,
    ) -> PublicResult<AgentEnrollmentResponse> {
        let client = AgentApiClient::unauthenticated(self.base_url(), self.transport_options())?;
        client.lookup_enrollment(enrollment_token).await
    }

    pub async fn approve_agent(
        &mut self,
        request: &ApproveAgentRequest,
    ) -> PublicResult<AgentResponse> {
        self.post_bounded_tracked_no_replay(
            "/agents",
            request,
            MAX_AGENT_SMALL_RESPONSE_BYTES,
            "approve agent",
        )
        .await
    }

    pub async fn list_agents(&mut self) -> PublicResult<Vec<AgentResponse>> {
        let mut agents = Vec::new();
        let mut after = None;
        loop {
            let path = after.map_or_else(
                || format!("/agents?limit={AGENT_LIST_PAGE_SIZE}"),
                |cursor| format!("/agents?limit={AGENT_LIST_PAGE_SIZE}&after={cursor}"),
            );
            let page: Vec<AgentResponse> = self
                .get_bounded(&path, MAX_AGENT_COLLECTION_RESPONSE_BYTES)
                .await?;
            if page.len() > AGENT_LIST_PAGE_SIZE {
                return Err(PublicError::response(
                    ResponseFailureKind::JsonSchema,
                    "agent API returned an oversized list page",
                ));
            }
            let page_len = page.len();
            let next_after = page.last().map(|agent| agent.id);
            if next_after.is_some_and(|next| Some(next) <= after) {
                return Err(PublicError::response(
                    ResponseFailureKind::JsonSchema,
                    "agent API returned a non-advancing list cursor",
                ));
            }
            agents.extend(page);
            if page_len < AGENT_LIST_PAGE_SIZE {
                return Ok(agents);
            }
            after = next_after;
        }
    }

    pub async fn revoke_agent(
        &mut self,
        agent_id: Uuid,
        reason: Option<String>,
    ) -> PublicResult<AgentResponse> {
        #[derive(Serialize)]
        struct RevokeAgentRequest {
            reason: Option<String>,
        }
        self.post_bounded_tracked_no_replay(
            &format!("/agents/{agent_id}/revoke"),
            &RevokeAgentRequest { reason },
            MAX_AGENT_SMALL_RESPONSE_BYTES,
            "revoke agent",
        )
        .await
    }

    pub async fn assign_agent_to_task(
        &mut self,
        work_list_id: Uuid,
        task_id: Uuid,
        agent_id: Uuid,
    ) -> PublicResult<AgentAssignmentResponse> {
        self.post_bounded_tracked_no_replay(
            &format!("/work-lists/{work_list_id}/tasks/{task_id}/agent-assignments/{agent_id}"),
            &serde_json::json!({}),
            MAX_AGENT_SMALL_RESPONSE_BYTES,
            "assign agent to task",
        )
        .await
    }

    pub async fn unassign_agent_from_task(
        &mut self,
        work_list_id: Uuid,
        task_id: Uuid,
        agent_id: Uuid,
    ) -> PublicResult<()> {
        self.delete_no_content_bounded(
            &format!("/work-lists/{work_list_id}/tasks/{task_id}/agent-assignments/{agent_id}"),
            MAX_AGENT_SMALL_RESPONSE_BYTES,
        )
        .await
    }
}

fn normalize_base_url(value: String) -> String {
    value.trim().trim_end_matches('/').to_string()
}

fn registration_may_have_committed(error: &PublicError) -> bool {
    if error.response_failure_kind().is_some() {
        return true;
    }
    if error
        .transport_failure_kind()
        .is_some_and(|kind| kind != TransportFailureKind::Connect)
    {
        return true;
    }
    if error
        .http_status()
        .is_some_and(|status| status == 408 || (500..=599).contains(&status))
    {
        return true;
    }
    matches!(
        error,
        PublicError::RequestTimeout(_)
            | PublicError::OutcomeAmbiguous { .. }
            | PublicError::CommittedButLocalProcessingFailed { .. }
    )
}

fn validate_registered_enrollment(
    enrollment: &AgentEnrollmentResponse,
    request: &RegisterAgentEnrollmentRequest,
) -> PublicResult<()> {
    if enrollment.proposed_handle != request.proposed_handle
        || enrollment.auth_public_key != request.auth_public_key
        || enrollment.recipient_public_key != request.recipient_public_key
    {
        return Err(PublicError::crypto(
            "agent enrollment response does not match the registration request",
        ));
    }
    Ok(())
}

fn map_transport_error(error: reqwest::Error) -> PublicError {
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

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[derive(Clone, Copy)]
    enum OwnerAgentMutation {
        Approve,
        Revoke,
        Assign,
    }

    #[test]
    fn sensitive_agent_debug_output_is_redacted() {
        let response = AgentTokenResponse {
            access_token: "agent-secret-token".to_string(),
            expires_in: 300,
            token_type: "Bearer".to_string(),
            agent_id: Uuid::now_v7(),
        };
        let rendered = format!("{response:?}");
        assert!(rendered.contains("<redacted>"));
        assert!(!rendered.contains("agent-secret-token"));

        let principal = AgentMeResponse {
            id: Uuid::now_v7(),
            owner_user_id: Uuid::now_v7(),
            work_list_id: Uuid::now_v7(),
            permission_preset: "assigned_task_worker".to_string(),
            instructions_revision: 1,
            handle: "implementer".to_string(),
            display_name: "Implementation Agent".to_string(),
            key_ciphertext: "PROJECT-KEY-CIPHERTEXT-CANARY".to_string(),
            instructions_ciphertext: "INSTRUCTIONS-CIPHERTEXT-CANARY".to_string(),
            grant_signature: "GRANT-SIGNATURE-CANARY".to_string(),
        };
        let rendered = format!("{principal:?}");
        assert!(rendered.contains("<redacted>"));
        assert!(!rendered.contains("PROJECT-KEY-CIPHERTEXT-CANARY"));
        assert!(!rendered.contains("INSTRUCTIONS-CIPHERTEXT-CANARY"));
        assert!(!rendered.contains("GRANT-SIGNATURE-CANARY"));
    }

    #[tokio::test]
    async fn agent_api_errors_preserve_retry_after() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind retry-after server");
        let address = listener.local_addr().expect("retry-after address");
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept agent request");
            let mut request = [0_u8; 4096];
            let bytes_read = stream.read(&mut request).await.expect("read agent request");
            assert!(
                String::from_utf8_lossy(&request[..bytes_read])
                    .starts_with("GET /agent/me/assignments/next ")
            );
            stream
                .write_all(
                    b"HTTP/1.1 429 Too Many Requests\r\nRetry-After: 17\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
                )
                .await
                .expect("write retry-after response");
        });
        let client = AgentApiClient::authenticated(
            format!("http://{address}"),
            "agent-token".to_string(),
            ApiTransportOptions::default(),
        )
        .expect("agent client");

        let error = client
            .next_assignment()
            .await
            .expect_err("rate limit response");
        assert_eq!(error.http_status(), Some(429));
        assert_eq!(error.retry_after(), Some(Duration::from_secs(17)));
        server.await.expect("retry-after server task");
    }

    #[tokio::test]
    async fn enrollment_mutation_boundary_remains_active_until_response_is_persisted() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test server");
        let address = listener.local_addr().expect("test server address");
        let agent_id = Uuid::now_v7();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept request");
            let mut request = [0_u8; 4096];
            let bytes_read = stream.read(&mut request).await.expect("read request");
            assert!(
                String::from_utf8_lossy(&request[..bytes_read])
                    .starts_with("POST /agent-enrollments ")
            );
            let body = serde_json::json!({
                "agentId": agent_id,
                "proposedHandle": "implementer",
                "authPublicKey": "auth",
                "recipientPublicKey": "recipient",
                "fingerprint": "fingerprint",
                "expiresAt": Utc::now(),
            })
            .to_string();
            let response = format!(
                "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream
                .write_all(response.as_bytes())
                .await
                .expect("write response");
        });

        let cancellation = ApiCancellationToken::new();
        let client = AgentApiClient::unauthenticated(
            format!("http://{address}"),
            ApiTransportOptions::default(),
        )
        .expect("agent client")
        .with_cancellation_token(cancellation.clone());
        let response = client
            .register_enrollment(&RegisterAgentEnrollmentRequest {
                proposed_handle: Some("implementer".to_string()),
                auth_public_key: "auth".to_string(),
                recipient_public_key: "recipient".to_string(),
                enrollment_token: "lookup-token".to_string(),
            })
            .await
            .expect("register enrollment");

        assert_eq!(response.enrollment.agent_id, agent_id);
        assert!(cancellation.mutation_request_in_flight());
        drop(response);
        assert!(!cancellation.mutation_request_in_flight());
        server.await.expect("test server task");
    }

    #[tokio::test]
    async fn truncated_successful_enrollment_is_reconciled_before_local_persistence() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind recovery server");
        let address = listener.local_addr().expect("recovery server address");
        let agent_id = Uuid::now_v7();
        let expires_at = Utc::now() + chrono::Duration::minutes(15);
        let server = tokio::spawn(async move {
            let (mut registration, _) = listener.accept().await.expect("accept registration");
            let mut request = [0_u8; 4096];
            let bytes_read = registration
                .read(&mut request)
                .await
                .expect("read registration");
            assert!(
                String::from_utf8_lossy(&request[..bytes_read])
                    .starts_with("POST /agent-enrollments ")
            );
            registration
                .write_all(
                    b"HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nContent-Length: 32\r\nConnection: close\r\n\r\n{",
                )
                .await
                .expect("write truncated registration response");
            drop(registration);

            let (mut lookup, _) = listener.accept().await.expect("accept recovery lookup");
            let bytes_read = lookup
                .read(&mut request)
                .await
                .expect("read recovery lookup");
            assert!(
                String::from_utf8_lossy(&request[..bytes_read])
                    .starts_with("POST /agent-enrollments/lookup ")
            );
            let body = serde_json::json!({
                "agentId": agent_id,
                "proposedHandle": "implementer",
                "authPublicKey": "auth",
                "recipientPublicKey": "recipient",
                "fingerprint": "fingerprint",
                "expiresAt": expires_at,
            })
            .to_string();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            lookup
                .write_all(response.as_bytes())
                .await
                .expect("write recovery lookup response");
        });

        let cancellation = ApiCancellationToken::new();
        let client = AgentApiClient::unauthenticated(
            format!("http://{address}"),
            ApiTransportOptions::default(),
        )
        .expect("agent client")
        .with_cancellation_token(cancellation.clone());
        let response = client
            .register_enrollment(&RegisterAgentEnrollmentRequest {
                proposed_handle: Some("implementer".to_string()),
                auth_public_key: "auth".to_string(),
                recipient_public_key: "recipient".to_string(),
                enrollment_token: "lookup-token".to_string(),
            })
            .await
            .expect("reconcile committed enrollment");

        assert_eq!(response.enrollment.agent_id, agent_id);
        assert!(cancellation.mutation_request_in_flight());
        drop(response);
        assert!(!cancellation.mutation_request_in_flight());
        server.await.expect("recovery server task");
    }

    #[tokio::test]
    async fn durable_enrollment_resume_looks_up_before_replaying_registration() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind resume server");
        let address = listener.local_addr().expect("resume server address");
        let agent_id = Uuid::now_v7();
        let expires_at = Utc::now() + chrono::Duration::minutes(15);
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept resume lookup");
            let mut request = [0_u8; 4096];
            let bytes_read = stream.read(&mut request).await.expect("read resume lookup");
            assert!(
                String::from_utf8_lossy(&request[..bytes_read])
                    .starts_with("POST /agent-enrollments/lookup ")
            );
            let body = serde_json::json!({
                "agentId": agent_id,
                "proposedHandle": "implementer",
                "authPublicKey": "auth",
                "recipientPublicKey": "recipient",
                "fingerprint": "fingerprint",
                "expiresAt": expires_at,
            })
            .to_string();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream
                .write_all(response.as_bytes())
                .await
                .expect("write resume response");
        });

        let cancellation = ApiCancellationToken::new();
        let client = AgentApiClient::unauthenticated(
            format!("http://{address}"),
            ApiTransportOptions::default(),
        )
        .expect("agent client")
        .with_cancellation_token(cancellation.clone());
        let response = client
            .resume_or_register_enrollment(&RegisterAgentEnrollmentRequest {
                proposed_handle: Some("implementer".to_string()),
                auth_public_key: "auth".to_string(),
                recipient_public_key: "recipient".to_string(),
                enrollment_token: "lookup-token".to_string(),
            })
            .await
            .expect("resume committed enrollment");

        assert_eq!(response.enrollment.agent_id, agent_id);
        assert!(!cancellation.mutation_request_in_flight());
        server.await.expect("resume server task");
    }

    #[tokio::test]
    async fn owner_agent_mutations_publish_the_in_flight_boundary_until_decoding_finishes() {
        for mutation in [
            OwnerAgentMutation::Approve,
            OwnerAgentMutation::Revoke,
            OwnerAgentMutation::Assign,
        ] {
            assert_owner_agent_mutation_boundary(mutation).await;
        }
    }

    #[tokio::test]
    async fn delivered_agent_assignment_with_server_failure_reports_an_ambiguous_outcome() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind assignment failure server");
        let address = listener.local_addr().expect("assignment failure address");
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept assignment");
            let mut request = [0_u8; 4096];
            let bytes_read = stream.read(&mut request).await.expect("read assignment");
            assert!(
                String::from_utf8_lossy(&request[..bytes_read]).contains("/agent-assignments/")
            );
            stream
                .write_all(
                    b"HTTP/1.1 503 Service Unavailable\r\nContent-Type: application/json\r\nContent-Length: 18\r\nConnection: close\r\n\r\n{\"error\":\"failed\"}",
                )
                .await
                .expect("write assignment failure");
        });
        let api_url = format!("http://{address}");
        let mut client = owner_client(&api_url, ApiCancellationToken::new());
        let error = client
            .assign_agent_to_task(Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7())
            .await
            .expect_err("a delivered no-replay assignment cannot be classified as failed");
        assert_eq!(error.code(), "outcome_ambiguous");
        server.await.expect("assignment failure server task");
    }

    #[tokio::test]
    async fn owner_agent_listing_follows_bounded_advancing_cursor_pages() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind agent list server");
        let address = listener.local_addr().expect("agent list address");
        let first_page = (1_u128..=100).map(agent_response_json).collect::<Vec<_>>();
        let second_page = vec![agent_response_json(101)];
        let server = tokio::spawn(async move {
            for (expected_path, page) in [
                ("/agents?limit=100".to_string(), first_page),
                (
                    format!("/agents?limit=100&after={}", Uuid::from_u128(100)),
                    second_page,
                ),
            ] {
                let (mut stream, _) = listener.accept().await.expect("accept agent list");
                let mut request = [0_u8; 4096];
                let bytes_read = stream.read(&mut request).await.expect("read agent list");
                let request = String::from_utf8_lossy(&request[..bytes_read]);
                assert!(
                    request.starts_with(&format!("GET {expected_path} HTTP/1.1")),
                    "unexpected agent list request: {request}"
                );
                let body = serde_json::to_string(&page).expect("agent list JSON");
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                stream
                    .write_all(response.as_bytes())
                    .await
                    .expect("write agent list");
            }
        });

        let api_url = format!("http://{address}");
        let mut client = PublicApiClient::with_credentials(
            &api_url,
            sealtask_client_auth::Credentials {
                api_url: api_url.clone(),
                access_token: "test-access".to_string(),
                refresh_token: "test-refresh".to_string(),
                access_expires_at: Utc::now() + chrono::Duration::hours(1),
                refresh_expires_at: Utc::now() + chrono::Duration::hours(2),
                user_id: Uuid::from_u128(500),
                email: "owner@example.com".to_string(),
                data_key_ciphertext: "unused".to_string(),
            },
        )
        .expect("owner agent API client");
        let agents = client.list_agents().await.expect("list all owner agents");
        assert_eq!(agents.len(), 101);
        assert_eq!(
            agents.first().map(|agent| agent.id),
            Some(Uuid::from_u128(1))
        );
        assert_eq!(
            agents.last().map(|agent| agent.id),
            Some(Uuid::from_u128(101))
        );
        server.await.expect("agent list server task");
    }

    #[tokio::test]
    async fn agent_assignment_listing_requests_one_bounded_cursor_page() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind assignment page server");
        let address = listener.local_addr().expect("assignment page address");
        let after = Uuid::from_u128(10);
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept assignment page");
            let mut request = [0_u8; 4096];
            let bytes_read = stream
                .read(&mut request)
                .await
                .expect("read assignment page");
            let request = String::from_utf8_lossy(&request[..bytes_read]);
            assert!(
                request.starts_with(&format!(
                    "GET /agent/me/assignments?limit=1&after={after} HTTP/1.1"
                )),
                "unexpected assignment page request: {request}"
            );
            let body = serde_json::to_string(&vec![agent_assignment_json(11)])
                .expect("assignment page JSON");
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream
                .write_all(response.as_bytes())
                .await
                .expect("write assignment page");
        });

        let client = AgentApiClient::authenticated(
            format!("http://{address}"),
            "agent-token".to_string(),
            ApiTransportOptions::default(),
        )
        .expect("agent client");
        assert!(client.list_assignments(None, 0).await.is_err());
        let assignments = client
            .list_assignments(Some(after), 1)
            .await
            .expect("list assignment page");
        assert_eq!(assignments.len(), 1);
        assert_eq!(assignments[0].id, Uuid::from_u128(11));
        server.await.expect("assignment page server task");
    }

    async fn assert_owner_agent_mutation_boundary(mutation: OwnerAgentMutation) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind owner mutation server");
        let address = listener.local_addr().expect("owner mutation address");
        let agent_id = Uuid::from_u128(1);
        let (request_seen_tx, request_seen_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept owner mutation");
            let mut request = [0_u8; 8192];
            let bytes_read = stream
                .read(&mut request)
                .await
                .expect("read owner mutation");
            let request = String::from_utf8_lossy(&request[..bytes_read]);
            let expected_path = match mutation {
                OwnerAgentMutation::Approve => "POST /agents HTTP/1.1",
                OwnerAgentMutation::Revoke => {
                    "POST /agents/00000000-0000-0000-0000-000000000001/revoke HTTP/1.1"
                }
                OwnerAgentMutation::Assign => "/agent-assignments/",
            };
            assert!(
                request.contains(expected_path),
                "unexpected mutation: {request}"
            );
            request_seen_tx.send(()).expect("publish request boundary");
            release_rx.await.expect("release owner mutation response");
            let body = match mutation {
                OwnerAgentMutation::Approve | OwnerAgentMutation::Revoke => agent_response_json(1),
                OwnerAgentMutation::Assign => agent_assignment_json(1),
            }
            .to_string();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream
                .write_all(response.as_bytes())
                .await
                .expect("write owner mutation response");
        });

        let cancellation = ApiCancellationToken::new();
        let api_url = format!("http://{address}");
        let mut client = owner_client(&api_url, cancellation.clone());
        let operation = tokio::spawn(async move {
            match mutation {
                OwnerAgentMutation::Approve => client
                    .approve_agent(&ApproveAgentRequest {
                        enrollment_token: "enrollment".to_string(),
                        fingerprint: "fingerprint".to_string(),
                        handle: "implementer".to_string(),
                        display_name: "Implementation Agent".to_string(),
                        work_list_id: Uuid::from_u128(2),
                        permission_preset: "assigned_task_worker".to_string(),
                        key_ciphertext: "key".to_string(),
                        instructions_ciphertext: "instructions".to_string(),
                        grant_signature: "signature".to_string(),
                        instructions_revision: 1,
                    })
                    .await
                    .map(|_| ()),
                OwnerAgentMutation::Revoke => client.revoke_agent(agent_id, None).await.map(|_| ()),
                OwnerAgentMutation::Assign => client
                    .assign_agent_to_task(Uuid::from_u128(2), Uuid::from_u128(3), agent_id)
                    .await
                    .map(|_| ()),
            }
        });
        request_seen_rx
            .await
            .expect("owner mutation reaches server");
        assert!(cancellation.mutation_request_in_flight());
        release_tx.send(()).expect("release owner mutation");
        operation
            .await
            .expect("owner mutation task")
            .expect("owner mutation response");
        assert!(!cancellation.mutation_request_in_flight());
        server.await.expect("owner mutation server task");
    }

    fn owner_client(api_url: &str, cancellation: ApiCancellationToken) -> PublicApiClient {
        PublicApiClient::with_credentials(
            api_url,
            sealtask_client_auth::Credentials {
                api_url: api_url.to_string(),
                access_token: "test-access".to_string(),
                refresh_token: "test-refresh".to_string(),
                access_expires_at: Utc::now() + chrono::Duration::hours(1),
                refresh_expires_at: Utc::now() + chrono::Duration::hours(2),
                user_id: Uuid::from_u128(500),
                email: "owner@example.com".to_string(),
                data_key_ciphertext: "unused".to_string(),
            },
        )
        .expect("owner agent API client")
        .with_cancellation_token(cancellation)
    }

    fn agent_response_json(id: u128) -> serde_json::Value {
        let now = Utc::now();
        serde_json::json!({
            "id": Uuid::from_u128(id),
            "ownerUserId": Uuid::from_u128(500),
            "handle": format!("agent-{id}"),
            "displayName": format!("Agent {id}"),
            "proposedHandle": null,
            "authPublicKey": "auth",
            "recipientPublicKey": "recipient",
            "fingerprint": "fingerprint",
            "status": "active",
            "revokedReason": null,
            "createdAt": now,
            "activatedAt": now,
            "revokedAt": null,
            "lastSeenAt": null,
            "grant": {
                "workListId": Uuid::from_u128(600),
                "permissionPreset": "assigned_task_worker",
                "instructionsRevision": 1,
                "createdAt": now,
                "updatedAt": now,
            },
        })
    }

    fn agent_assignment_json(id: u128) -> serde_json::Value {
        let now = Utc::now();
        serde_json::json!({
            "id": Uuid::from_u128(id),
            "taskId": Uuid::from_u128(id + 100),
            "workListId": Uuid::from_u128(600),
            "status": "pending",
            "statusRevision": 0,
            "referenceNumber": id as i64,
            "priority": null,
            "dueAt": null,
            "startAt": null,
            "completedAt": null,
            "isCompleted": false,
            "taskUpdatedAt": now,
            "createdAt": now,
            "updatedAt": now,
            "latestRunId": null,
            "latestRunStatus": null,
            "latestRunLeaseExpiresAt": null,
            "claimable": true,
        })
    }
}
