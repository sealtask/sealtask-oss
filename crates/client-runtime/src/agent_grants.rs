use base64::{Engine as _, engine::general_purpose::STANDARD_NO_PAD};
use uuid::Uuid;

use sealtask_client_auth::{canonicalize_agent_display_name, canonicalize_agent_handle};
use sealtask_client_core::{PublicError, PublicResult};
use sealtask_client_crypto::{
    AgentGrantAuthenticationInput, encrypt_agent_instructions, encrypt_agent_project_key,
    sign_agent_grant,
};

use crate::RuntimeClient;

#[derive(Clone, Debug)]
pub struct PreparedAgentApprovalGrant {
    pub key_ciphertext: String,
    pub instructions_ciphertext: String,
    pub grant_signature: String,
    pub instructions_revision: i64,
}

pub struct PrepareAgentApprovalGrant<'a> {
    pub enrollment_code: &'a str,
    pub agent_id: Uuid,
    pub auth_public_key: &'a str,
    pub recipient_public_key: &'a str,
    pub work_list_id: Uuid,
    pub handle: &'a str,
    pub display_name: &'a str,
    pub instructions: &'a [u8],
    pub instructions_revision: i64,
    pub password_stdin: bool,
}

impl RuntimeClient {
    pub async fn prepare_agent_approval_grant(
        &self,
        input: PrepareAgentApprovalGrant<'_>,
    ) -> PublicResult<PreparedAgentApprovalGrant> {
        let PrepareAgentApprovalGrant {
            enrollment_code,
            agent_id,
            auth_public_key,
            recipient_public_key,
            work_list_id,
            handle,
            display_name,
            instructions,
            instructions_revision,
            password_stdin,
        } = input;
        if instructions_revision <= 0 {
            return Err(PublicError::validation(
                "agent instructions revision must be positive",
            ));
        }
        if instructions.is_empty() {
            return Err(PublicError::validation(
                "agent instructions cannot be empty",
            ));
        }
        if canonicalize_agent_handle(handle)? != handle
            || canonicalize_agent_display_name(display_name)? != display_name
        {
            return Err(PublicError::validation(
                "agent identity metadata must use its canonical form",
            ));
        }
        let auth_public_key = decode_agent_public_key("authentication", auth_public_key)?;
        let recipient_public_key = decode_agent_public_key("recipient", recipient_public_key)?;
        let (_client, context) = self
            .load_work_list_context(
                work_list_id,
                password_stdin,
                "Password required to approve agent access.",
            )
            .await?;
        let project_key = self.require_work_list_key(&context)?;
        let key_ciphertext = encrypt_agent_project_key(
            &recipient_public_key,
            agent_id,
            work_list_id,
            instructions_revision,
            project_key,
        )?;
        let instructions_ciphertext = encrypt_agent_instructions(
            &recipient_public_key,
            agent_id,
            work_list_id,
            instructions_revision,
            instructions,
        )?;
        let grant_signature = sign_agent_grant(
            enrollment_code,
            AgentGrantAuthenticationInput {
                agent_id,
                work_list_id,
                handle,
                display_name,
                permission_preset: "assigned_task_worker",
                instructions_revision,
                auth_public_key: &auth_public_key,
                recipient_public_key: &recipient_public_key,
                key_ciphertext: &key_ciphertext.bytes,
                instructions_ciphertext: &instructions_ciphertext.bytes,
            },
        )?;

        Ok(PreparedAgentApprovalGrant {
            key_ciphertext: key_ciphertext.base64,
            instructions_ciphertext: instructions_ciphertext.base64,
            grant_signature,
            instructions_revision,
        })
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
