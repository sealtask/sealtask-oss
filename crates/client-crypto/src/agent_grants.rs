use base64::{
    Engine as _,
    engine::general_purpose::{STANDARD_NO_PAD, URL_SAFE_NO_PAD},
};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use hkdf::Hkdf;
use sha2::{Digest, Sha256};
use strong_box::{SharedStrongBox, SharedStrongBoxKey, StrongBox};
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};

use sealtask_client_core::{PublicError, PublicResult};

use crate::{
    KEY_SIZE, SealedBlobPayload, SymmetricKey, decrypt_encrypted_text_value, encrypt_text_value,
    symmetric_key_from_bytes,
};

const SHARED_STRONG_BOX_PRIVATE_KEY: u8 = 0;
const SHARED_STRONG_BOX_PUBLIC_KEY: u8 = 1;
const AGENT_ENROLLMENT_CODE_BYTES: usize = 32;
const AGENT_ENROLLMENT_TOKEN_CONTEXT: &[u8] = b"sealtask.agent.enrollment-token.v1\0";
const AGENT_GRANT_SIGNING_KEY_CONTEXT: &[u8] = b"sealtask.agent.grant-signing-key.v1";
const AGENT_GRANT_SIGNATURE_CONTEXT: &[u8] = b"sealtask.agent.grant-signature.v2\0";

pub const MAX_AGENT_INSTRUCTIONS_PLAINTEXT_BYTES: usize = 256 * 1024;
pub const MAX_AGENT_INSTRUCTIONS_CIPHERTEXT_BYTES: usize = 257 * 1024;

#[derive(Clone, Copy, Debug)]
pub struct AgentGrantAuthenticationInput<'a> {
    pub agent_id: Uuid,
    pub work_list_id: Uuid,
    pub handle: &'a str,
    pub display_name: &'a str,
    pub permission_preset: &'a str,
    pub instructions_revision: i64,
    pub auth_public_key: &'a [u8],
    pub recipient_public_key: &'a [u8],
    pub key_ciphertext: &'a [u8],
    pub instructions_ciphertext: &'a [u8],
}

/// Derives the opaque value sent to the API for enrollment lookup. The raw
/// enrollment code remains off-server and authenticates the owner-created
/// project grant.
pub fn derive_agent_enrollment_token(enrollment_code: &str) -> PublicResult<String> {
    let secret = decode_agent_enrollment_code(enrollment_code)?;
    let mut hasher = Sha256::new();
    hasher.update(AGENT_ENROLLMENT_TOKEN_CONTEXT);
    hasher.update(secret.as_slice());
    Ok(URL_SAFE_NO_PAD.encode(hasher.finalize()))
}

pub fn sign_agent_grant(
    enrollment_code: &str,
    input: AgentGrantAuthenticationInput<'_>,
) -> PublicResult<String> {
    let secret = decode_agent_enrollment_code(enrollment_code)?;
    let signing_key = derive_agent_grant_signing_key(&secret)?;
    let digest = agent_grant_authentication_digest(input)?;
    Ok(STANDARD_NO_PAD.encode(signing_key.sign(&digest).to_bytes()))
}

pub fn verify_agent_grant(
    enrollment_code: &str,
    signature: &str,
    input: AgentGrantAuthenticationInput<'_>,
) -> PublicResult<()> {
    let secret = decode_agent_enrollment_code(enrollment_code)?;
    let signing_key = derive_agent_grant_signing_key(&secret)?;
    let verifying_key = VerifyingKey::from(&signing_key);
    let signature = STANDARD_NO_PAD
        .decode(signature.trim())
        .map_err(|_| PublicError::crypto("invalid agent grant signature encoding"))?;
    let signature = Signature::from_slice(&signature)
        .map_err(|_| PublicError::crypto("invalid agent grant signature length"))?;
    let digest = agent_grant_authentication_digest(input)?;
    verifying_key
        .verify_strict(&digest, &signature)
        .map_err(|_| PublicError::crypto("agent grant was not authenticated by the project owner"))
}

pub fn encrypt_agent_project_key(
    recipient_public_key: &[u8],
    agent_id: Uuid,
    work_list_id: Uuid,
    instructions_revision: i64,
    project_key: &SymmetricKey,
) -> PublicResult<SealedBlobPayload> {
    let context = agent_grant_context("project-key", agent_id, work_list_id, instructions_revision);
    encrypt_for_agent(recipient_public_key, project_key.as_bytes(), &context)
}

pub fn decrypt_agent_project_key(
    recipient_private_key: &[u8],
    agent_id: Uuid,
    work_list_id: Uuid,
    instructions_revision: i64,
    ciphertext: &[u8],
) -> PublicResult<SymmetricKey> {
    let context = agent_grant_context("project-key", agent_id, work_list_id, instructions_revision);
    let plaintext = Zeroizing::new(decrypt_for_agent(
        recipient_private_key,
        ciphertext,
        &context,
    )?);
    symmetric_key_from_bytes(&plaintext)
}

pub fn encrypt_agent_instructions(
    recipient_public_key: &[u8],
    agent_id: Uuid,
    work_list_id: Uuid,
    instructions_revision: i64,
    instructions: &[u8],
) -> PublicResult<SealedBlobPayload> {
    if instructions.len() > MAX_AGENT_INSTRUCTIONS_PLAINTEXT_BYTES {
        return Err(PublicError::validation(format!(
            "agent instructions exceed the {MAX_AGENT_INSTRUCTIONS_PLAINTEXT_BYTES}-byte plaintext limit"
        )));
    }
    let context = agent_grant_context(
        "instructions",
        agent_id,
        work_list_id,
        instructions_revision,
    );
    let encrypted = encrypt_for_agent(recipient_public_key, instructions, &context)?;
    if encrypted.bytes.len() > MAX_AGENT_INSTRUCTIONS_CIPHERTEXT_BYTES {
        return Err(PublicError::unexpected(
            "encrypted agent instructions exceeded the protocol ciphertext limit",
        ));
    }
    Ok(encrypted)
}

pub fn decrypt_agent_instructions(
    recipient_private_key: &[u8],
    agent_id: Uuid,
    work_list_id: Uuid,
    instructions_revision: i64,
    ciphertext: &[u8],
) -> PublicResult<Zeroizing<Vec<u8>>> {
    let context = agent_grant_context(
        "instructions",
        agent_id,
        work_list_id,
        instructions_revision,
    );
    decrypt_for_agent(recipient_private_key, ciphertext, &context).map(Zeroizing::new)
}

pub fn encrypt_agent_run_result(
    result: &str,
    run_id: Uuid,
    project_key: &SymmetricKey,
) -> PublicResult<SealedBlobPayload> {
    encrypt_text_value(result, project_key, &agent_run_result_context(run_id))
}

pub fn decrypt_agent_run_result(
    ciphertext: &[u8],
    run_id: Uuid,
    project_key: &SymmetricKey,
) -> PublicResult<String> {
    decrypt_encrypted_text_value(ciphertext, project_key, &agent_run_result_context(run_id))
}

fn encrypt_for_agent(
    recipient_public_key: &[u8],
    plaintext: &[u8],
    context: &[u8],
) -> PublicResult<SealedBlobPayload> {
    let key = shared_key(recipient_public_key, SHARED_STRONG_BOX_PUBLIC_KEY)?;
    let ciphertext = SharedStrongBox::new(key)
        .encrypt(plaintext, context)
        .map_err(|error| PublicError::crypto(format!("failed to encrypt agent grant: {error}")))?;
    Ok(SealedBlobPayload {
        base64: STANDARD_NO_PAD.encode(&ciphertext),
        bytes: ciphertext,
    })
}

fn decrypt_for_agent(
    recipient_private_key: &[u8],
    ciphertext: &[u8],
    context: &[u8],
) -> PublicResult<Vec<u8>> {
    let key = shared_key(recipient_private_key, SHARED_STRONG_BOX_PRIVATE_KEY)?;
    SharedStrongBox::new(key)
        .decrypt(ciphertext, context)
        .map_err(|error| PublicError::crypto(format!("failed to decrypt agent grant: {error}")))
}

fn shared_key(raw_key: &[u8], key_kind: u8) -> PublicResult<SharedStrongBoxKey> {
    if raw_key.len() != KEY_SIZE {
        return Err(PublicError::validation(
            "agent recipient key must be 32 bytes",
        ));
    }
    let mut encoded = Vec::with_capacity(KEY_SIZE + 1);
    encoded.push(key_kind);
    encoded.extend_from_slice(raw_key);
    SharedStrongBoxKey::try_from(encoded.as_slice())
        .map_err(|error| PublicError::crypto(format!("invalid agent recipient key: {error}")))
}

fn decode_agent_enrollment_code(enrollment_code: &str) -> PublicResult<Zeroizing<Vec<u8>>> {
    let decoded = Zeroizing::new(
        URL_SAFE_NO_PAD
            .decode(enrollment_code.trim())
            .map_err(|_| PublicError::validation("invalid agent enrollment code"))?,
    );
    if decoded.len() != AGENT_ENROLLMENT_CODE_BYTES {
        return Err(PublicError::validation("invalid agent enrollment code"));
    }
    Ok(decoded)
}

fn derive_agent_grant_signing_key(secret: &[u8]) -> PublicResult<SigningKey> {
    let hkdf = Hkdf::<Sha256>::new(None, secret);
    let mut seed = [0_u8; AGENT_ENROLLMENT_CODE_BYTES];
    hkdf.expand(AGENT_GRANT_SIGNING_KEY_CONTEXT, &mut seed)
        .map_err(|error| {
            PublicError::crypto(format!(
                "agent grant signing key derivation failed: {error}"
            ))
        })?;
    let signing_key = SigningKey::from_bytes(&seed);
    seed.zeroize();
    Ok(signing_key)
}

fn agent_grant_authentication_digest(
    input: AgentGrantAuthenticationInput<'_>,
) -> PublicResult<[u8; 32]> {
    if input.instructions_revision <= 0
        || input.handle.is_empty()
        || input.display_name.is_empty()
        || input.permission_preset.is_empty()
        || input.auth_public_key.len() != KEY_SIZE
        || input.recipient_public_key.len() != KEY_SIZE
        || input.key_ciphertext.is_empty()
        || input.instructions_ciphertext.is_empty()
    {
        return Err(PublicError::validation(
            "invalid authenticated agent grant fields",
        ));
    }

    let mut hasher = Sha256::new();
    hasher.update(AGENT_GRANT_SIGNATURE_CONTEXT);
    update_grant_field(&mut hasher, b"agent_id", input.agent_id.as_bytes());
    update_grant_field(&mut hasher, b"work_list_id", input.work_list_id.as_bytes());
    update_grant_field(&mut hasher, b"handle", input.handle.as_bytes());
    update_grant_field(&mut hasher, b"display_name", input.display_name.as_bytes());
    update_grant_field(
        &mut hasher,
        b"permission_preset",
        input.permission_preset.as_bytes(),
    );
    update_grant_field(
        &mut hasher,
        b"instructions_revision",
        &input.instructions_revision.to_be_bytes(),
    );
    update_grant_field(&mut hasher, b"auth_public_key", input.auth_public_key);
    update_grant_field(
        &mut hasher,
        b"recipient_public_key",
        input.recipient_public_key,
    );
    update_grant_field(&mut hasher, b"key_ciphertext", input.key_ciphertext);
    update_grant_field(
        &mut hasher,
        b"instructions_ciphertext",
        input.instructions_ciphertext,
    );
    Ok(hasher.finalize().into())
}

fn update_grant_field(hasher: &mut Sha256, label: &[u8], value: &[u8]) {
    hasher.update((label.len() as u64).to_be_bytes());
    hasher.update(label);
    hasher.update((value.len() as u64).to_be_bytes());
    hasher.update(value);
}

fn agent_grant_context(
    kind: &str,
    agent_id: Uuid,
    work_list_id: Uuid,
    instructions_revision: i64,
) -> Vec<u8> {
    format!(
        "sealtask.agent-grant.v1:{kind}:{agent_id}:{work_list_id}:{instructions_revision}:assigned_task_worker"
    )
    .into_bytes()
}

fn agent_run_result_context(run_id: Uuid) -> Vec<u8> {
    format!("sealtask.agent-run-result.v1:{run_id}").into_bytes()
}

#[cfg(test)]
mod tests {
    use x25519_dalek::{PublicKey, StaticSecret};

    use super::*;

    #[test]
    fn project_key_and_instructions_round_trip_with_bound_context() {
        let private = StaticSecret::from([0x41; KEY_SIZE]);
        let public = PublicKey::from(&private);
        let agent_id = Uuid::now_v7();
        let work_list_id = Uuid::now_v7();
        let project_key = SymmetricKey::new([0x52; KEY_SIZE]);

        let encrypted_key =
            encrypt_agent_project_key(public.as_bytes(), agent_id, work_list_id, 1, &project_key)
                .expect("encrypt project key");
        let decrypted_key = decrypt_agent_project_key(
            &private.to_bytes(),
            agent_id,
            work_list_id,
            1,
            &encrypted_key.bytes,
        )
        .expect("decrypt project key");
        assert_eq!(decrypted_key.as_bytes(), project_key.as_bytes());

        let encrypted_instructions = encrypt_agent_instructions(
            public.as_bytes(),
            agent_id,
            work_list_id,
            1,
            b"Review tests before editing.",
        )
        .expect("encrypt instructions");
        let instructions = decrypt_agent_instructions(
            &private.to_bytes(),
            agent_id,
            work_list_id,
            1,
            &encrypted_instructions.bytes,
        )
        .expect("decrypt instructions");
        assert_eq!(instructions.as_slice(), b"Review tests before editing.");
    }

    #[test]
    fn grant_context_rejects_cross_agent_substitution() {
        let private = StaticSecret::from([0x43; KEY_SIZE]);
        let public = PublicKey::from(&private);
        let encrypted = encrypt_agent_instructions(
            public.as_bytes(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            1,
            b"instructions",
        )
        .expect("encrypt instructions");
        assert!(
            decrypt_agent_instructions(
                &private.to_bytes(),
                Uuid::now_v7(),
                Uuid::now_v7(),
                1,
                &encrypted.bytes,
            )
            .is_err()
        );
    }

    #[test]
    fn maximum_instructions_fit_the_protocol_ciphertext_budget() {
        let private = StaticSecret::from([0x61; KEY_SIZE]);
        let public = PublicKey::from(&private);
        let instructions = vec![b'x'; MAX_AGENT_INSTRUCTIONS_PLAINTEXT_BYTES];
        let encrypted = encrypt_agent_instructions(
            public.as_bytes(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            1,
            &instructions,
        )
        .expect("encrypt instructions at the boundary");
        assert!(encrypted.bytes.len() <= MAX_AGENT_INSTRUCTIONS_CIPHERTEXT_BYTES);

        let oversized = vec![b'x'; MAX_AGENT_INSTRUCTIONS_PLAINTEXT_BYTES + 1];
        assert!(
            encrypt_agent_instructions(
                public.as_bytes(),
                Uuid::now_v7(),
                Uuid::now_v7(),
                1,
                &oversized,
            )
            .is_err()
        );
    }

    #[test]
    fn signed_grant_is_bound_to_identity_project_and_ciphertexts() {
        let enrollment_code = URL_SAFE_NO_PAD.encode([0x71; AGENT_ENROLLMENT_CODE_BYTES]);
        let agent_id = Uuid::now_v7();
        let work_list_id = Uuid::now_v7();
        let auth_public_key = [0x72; KEY_SIZE];
        let recipient_public_key = [0x73; KEY_SIZE];
        let key_ciphertext = [0x74; 48];
        let instructions_ciphertext = [0x75; 96];
        let input = AgentGrantAuthenticationInput {
            agent_id,
            work_list_id,
            handle: "implementer",
            display_name: "Implementation Agent",
            permission_preset: "assigned_task_worker",
            instructions_revision: 1,
            auth_public_key: &auth_public_key,
            recipient_public_key: &recipient_public_key,
            key_ciphertext: &key_ciphertext,
            instructions_ciphertext: &instructions_ciphertext,
        };
        let signature = sign_agent_grant(&enrollment_code, input).expect("sign grant");

        verify_agent_grant(&enrollment_code, &signature, input).expect("verify grant");
        let substituted = AgentGrantAuthenticationInput {
            instructions_revision: 2,
            ..input
        };
        assert!(verify_agent_grant(&enrollment_code, &signature, substituted).is_err());
        let substituted = AgentGrantAuthenticationInput {
            handle: "reviewer",
            ..input
        };
        assert!(verify_agent_grant(&enrollment_code, &signature, substituted).is_err());
        let substituted = AgentGrantAuthenticationInput {
            display_name: "Review Agent",
            ..input
        };
        assert!(verify_agent_grant(&enrollment_code, &signature, substituted).is_err());
        let mut forged_instructions = instructions_ciphertext;
        forged_instructions[0] ^= 1;
        let substituted = AgentGrantAuthenticationInput {
            instructions_ciphertext: &forged_instructions,
            ..input
        };
        assert!(verify_agent_grant(&enrollment_code, &signature, substituted).is_err());

        let lookup_token = derive_agent_enrollment_token(&enrollment_code).unwrap();
        assert_ne!(lookup_token, enrollment_code);
        let server_forgery = sign_agent_grant(&lookup_token, input).unwrap();
        assert!(verify_agent_grant(&enrollment_code, &server_forgery, input).is_err());
    }
}
