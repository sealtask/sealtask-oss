use std::fmt;

use hkdf::Hkdf;
use serde::de::IgnoredAny;
use sha2::{Digest, Sha256};
use strong_box::StrongBox;
use uuid::Uuid;
use zeroize::Zeroizing;

use sealtask_client_core::{PublicError, PublicResult};

use crate::{
    DataKeyCiphertextVersion, SealedPayload, StrongBoxKeyRing, SymmetricKey, decode_base64,
    deserialize_from_cbor,
};

/// Maximum decrypted size of a persistent read-cache document.
pub const MAX_READ_CACHE_PLAINTEXT_BYTES: usize = 64 * 1024 * 1024;

/// Maximum size of the single raw StrongBox frame stored for a read cache.
///
/// The allowance above [`MAX_READ_CACHE_PLAINTEXT_BYTES`] covers the
/// StrongBox key identifier, nonce, authentication tag, and CBOR framing.
pub const MAX_READ_CACHE_CIPHERTEXT_BYTES: usize = MAX_READ_CACHE_PLAINTEXT_BYTES + 256;

const MAX_API_URL_BYTES: usize = 2_048;
const MAX_PROFILE_NAME_BYTES: usize = 64;
const MAX_DATA_KEY_CIPHERTEXT_B64_BYTES: usize = 64 * 1024;
const READ_CACHE_BINDING_DOMAIN: &[u8] = b"sealtask.read-cache.binding.v1";
const READ_CACHE_KEY_SALT: &[u8] = b"sealtask.read-cache.hkdf.salt.v1";
const READ_CACHE_KEY_INFO: &[u8] = b"sealtask.read-cache.encryption-key.v1";
const READ_CACHE_AAD_DOMAIN: &[u8] = b"sealtask.read-cache.aad.v1";
const STRONG_BOX_CIPHERTEXT_MAGIC: &[u8] = &[0xb1, 0xb8, 0xf5];

/// Opaque identity binding for an encrypted read cache.
///
/// The binding commits to the normalized API URL, account UUID, active
/// profile, and SHA-256 digest of the decoded data-key ciphertext. It is
/// deliberately opaque and its debug representation is redacted.
///
/// Cache encryption authenticates the binding but does not provide rollback
/// protection. Deleting a cache file is also only best-effort secure deletion;
/// filesystems or backups may retain ciphertext blocks.
#[derive(Clone, Eq, PartialEq)]
pub struct ReadCacheBinding {
    digest: Zeroizing<[u8; 32]>,
}

impl ReadCacheBinding {
    /// Builds an identity binding for a read-cache ciphertext.
    ///
    /// Equivalent API URLs with surrounding whitespace or trailing slashes
    /// produce the same binding. The data-key ciphertext is decoded before it
    /// is hashed, so equivalent supported base64 encodings bind identically.
    pub fn new(
        normalized_api_url: &str,
        user_id: Uuid,
        active_profile: &str,
        data_key_ciphertext_b64: &str,
    ) -> PublicResult<Self> {
        let api_url = normalized_api_url.trim().trim_end_matches('/');
        validate_api_url(api_url)?;
        let profile = active_profile.trim();
        validate_profile(profile)?;

        let encoded_data_key = data_key_ciphertext_b64.trim();
        if encoded_data_key.is_empty() || encoded_data_key.len() > MAX_DATA_KEY_CIPHERTEXT_B64_BYTES
        {
            return Err(PublicError::validation(
                "data key ciphertext must contain at most 65536 base64 bytes",
            ));
        }
        let decoded_data_key = Zeroizing::new(decode_base64(encoded_data_key)?);
        let payload = SealedPayload::from_bytes(&decoded_data_key)?;
        DataKeyCiphertextVersion::try_from(payload.version)?;
        if payload.ciphertext.is_empty() {
            return Err(PublicError::validation("data key payload is empty"));
        }
        let data_key_ciphertext_digest = Sha256::digest(&decoded_data_key);

        let mut hasher = Sha256::new();
        hasher.update(READ_CACHE_BINDING_DOMAIN);
        hash_binding_field(&mut hasher, b"api_url", api_url.as_bytes())?;
        hash_binding_field(&mut hasher, b"user_id", user_id.as_bytes())?;
        hash_binding_field(&mut hasher, b"profile", profile.as_bytes())?;
        hash_binding_field(
            &mut hasher,
            b"data_key_ciphertext_sha256",
            &data_key_ciphertext_digest,
        )?;

        Ok(Self {
            digest: Zeroizing::new(hasher.finalize().into()),
        })
    }
}

impl fmt::Debug for ReadCacheBinding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ReadCacheBinding")
            .field("digest", &"<redacted>")
            .finish()
    }
}

/// Encrypts one strict CBOR read-cache document into one raw StrongBox frame.
///
/// The caller must persist the returned bytes directly, without a plaintext
/// metadata sidecar. This format authenticates cache contents and identity
/// binding, but cannot detect replacement with an older valid ciphertext.
pub fn seal_read_cache(
    data_key: &SymmetricKey,
    binding: &ReadCacheBinding,
    plaintext_cbor: &[u8],
) -> PublicResult<Vec<u8>> {
    validate_plaintext_cbor(plaintext_cbor)?;
    let cache_key = derive_read_cache_key(data_key, binding)?;
    let context = ReadCacheContext::new(binding);
    let ciphertext = StrongBoxKeyRing::new(cache_key)
        .strong_box()
        .encrypt(plaintext_cbor, &context)
        .map_err(|err| PublicError::crypto(format!("failed to encrypt read cache: {err}")))?;
    if ciphertext.len() > MAX_READ_CACHE_CIPHERTEXT_BYTES {
        return Err(PublicError::validation(
            "encrypted read cache exceeds the 64 MiB storage limit",
        ));
    }
    validate_raw_strong_box_ciphertext(&ciphertext)?;
    Ok(ciphertext)
}

/// Decrypts one raw StrongBox frame and returns one strict CBOR cache document.
///
/// Plaintext is zeroized when the returned value is dropped. Authentication
/// fails if the API URL, account, profile, or data-key ciphertext binding
/// differs from the values used at encryption time.
pub fn open_read_cache(
    data_key: &SymmetricKey,
    binding: &ReadCacheBinding,
    ciphertext: &[u8],
) -> PublicResult<Zeroizing<Vec<u8>>> {
    if ciphertext.is_empty() || ciphertext.len() > MAX_READ_CACHE_CIPHERTEXT_BYTES {
        return Err(PublicError::validation(
            "encrypted read cache must contain at most 64 MiB plus framing",
        ));
    }
    validate_raw_strong_box_ciphertext(ciphertext)?;
    let cache_key = derive_read_cache_key(data_key, binding)?;
    let context = ReadCacheContext::new(binding);
    let plaintext = Zeroizing::new(
        StrongBoxKeyRing::new(cache_key)
            .strong_box()
            .decrypt(ciphertext, &context)
            .map_err(|err| PublicError::crypto(format!("failed to decrypt read cache: {err}")))?,
    );
    validate_plaintext_cbor(&plaintext)?;
    Ok(plaintext)
}

fn derive_read_cache_key(
    data_key: &SymmetricKey,
    binding: &ReadCacheBinding,
) -> PublicResult<SymmetricKey> {
    let mut output = Zeroizing::new([0_u8; 32]);
    let mut info = Zeroizing::new(Vec::with_capacity(READ_CACHE_KEY_INFO.len() + 32));
    info.extend_from_slice(READ_CACHE_KEY_INFO);
    info.extend_from_slice(&binding.digest[..]);
    Hkdf::<Sha256>::new(Some(READ_CACHE_KEY_SALT), data_key.as_bytes())
        .expand(&info, &mut output[..])
        .map_err(|err| PublicError::crypto(format!("read-cache HKDF failed: {err}")))?;
    Ok(SymmetricKey::new(*output))
}

fn validate_plaintext_cbor(plaintext: &[u8]) -> PublicResult<()> {
    if plaintext.is_empty() || plaintext.len() > MAX_READ_CACHE_PLAINTEXT_BYTES {
        return Err(PublicError::validation(
            "read-cache plaintext must be one CBOR value of at most 64 MiB",
        ));
    }
    let _: IgnoredAny = deserialize_from_cbor(plaintext).map_err(|_| {
        PublicError::validation("read-cache plaintext must contain exactly one valid CBOR value")
    })?;
    Ok(())
}

fn validate_raw_strong_box_ciphertext(ciphertext: &[u8]) -> PublicResult<()> {
    let Some(encoded_frame) = ciphertext.strip_prefix(STRONG_BOX_CIPHERTEXT_MAGIC) else {
        return Err(PublicError::validation(
            "encrypted read cache is not a StrongBox ciphertext",
        ));
    };
    let _: IgnoredAny = deserialize_from_cbor(encoded_frame).map_err(|_| {
        PublicError::validation(
            "encrypted read cache must contain exactly one valid StrongBox frame",
        )
    })?;
    Ok(())
}

fn validate_api_url(api_url: &str) -> PublicResult<()> {
    if api_url.is_empty()
        || api_url.len() > MAX_API_URL_BYTES
        || !(api_url.starts_with("https://") || api_url.starts_with("http://"))
    {
        return Err(PublicError::validation(
            "normalized API URL must be a non-empty HTTP(S) URL of at most 2048 bytes",
        ));
    }
    Ok(())
}

fn validate_profile(profile: &str) -> PublicResult<()> {
    if profile.is_empty()
        || profile.len() > MAX_PROFILE_NAME_BYTES
        || profile == "."
        || profile == ".."
        || !profile
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(PublicError::validation(
            "profile must contain 1 to 64 ASCII letters, digits, '.', '_', or '-' and cannot be '.' or '..'",
        ));
    }
    Ok(())
}

fn hash_binding_field(hasher: &mut Sha256, label: &[u8], value: &[u8]) -> PublicResult<()> {
    let label_length = u16::try_from(label.len())
        .map_err(|_| PublicError::validation("read-cache binding label is too long"))?;
    let value_length = u32::try_from(value.len())
        .map_err(|_| PublicError::validation("read-cache binding field is too long"))?;
    hasher.update(label_length.to_be_bytes());
    hasher.update(label);
    hasher.update(value_length.to_be_bytes());
    hasher.update(value);
    Ok(())
}

struct ReadCacheContext(Zeroizing<Vec<u8>>);

impl ReadCacheContext {
    fn new(binding: &ReadCacheBinding) -> Self {
        let mut context = Zeroizing::new(Vec::with_capacity(READ_CACHE_AAD_DOMAIN.len() + 32));
        context.extend_from_slice(READ_CACHE_AAD_DOMAIN);
        context.extend_from_slice(&binding.digest[..]);
        Self(context)
    }
}

impl AsRef<[u8]> for ReadCacheContext {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Debug for ReadCacheContext {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ReadCacheContext(<redacted>)")
    }
}

#[cfg(test)]
mod tests {
    use base64::{Engine as _, engine::general_purpose::STANDARD_NO_PAD};
    use serde::Serialize;

    use super::*;
    use crate::{SealedPayload, serialize_to_cbor};

    #[derive(Serialize)]
    struct CacheDocument<'a> {
        version: u8,
        value: &'a str,
    }

    fn data_key_payload(marker: u8) -> String {
        let payload = SealedPayload::new(vec![marker; 48])
            .to_bytes()
            .expect("encode data-key envelope");
        STANDARD_NO_PAD.encode(payload)
    }

    fn binding(
        api_url: &str,
        user_id: Uuid,
        profile: &str,
        data_key_ciphertext: &str,
    ) -> ReadCacheBinding {
        ReadCacheBinding::new(api_url, user_id, profile, data_key_ciphertext)
            .expect("read-cache binding")
    }

    fn plaintext() -> Vec<u8> {
        serialize_to_cbor(&CacheDocument {
            version: 1,
            value: "private",
        })
        .expect("cache CBOR")
    }

    #[test]
    fn test_should_round_trip_one_raw_strong_box_ciphertext() {
        let key = SymmetricKey::new([0x31; 32]);
        let account = Uuid::from_u128(1);
        let data_key_ciphertext = data_key_payload(0x44);
        let binding = binding(
            "https://api.example.test/",
            account,
            "default",
            &data_key_ciphertext,
        );
        let expected = plaintext();

        let ciphertext = seal_read_cache(&key, &binding, &expected).expect("seal cache");
        assert!(ciphertext.starts_with(&[0xb1, 0xb8, 0xf5]));
        let actual = open_read_cache(&key, &binding, &ciphertext).expect("open cache");

        assert_eq!(&actual[..], expected);
    }

    #[test]
    fn test_should_bind_cache_to_account_profile_api_and_data_key_ciphertext() {
        let key = SymmetricKey::new([0x32; 32]);
        let account = Uuid::from_u128(2);
        let data_key_ciphertext = data_key_payload(0x45);
        let original = binding(
            "https://api.example.test",
            account,
            "default",
            &data_key_ciphertext,
        );
        let ciphertext = seal_read_cache(&key, &original, &plaintext()).expect("seal read cache");

        let different_bindings = [
            binding(
                "https://other.example.test",
                account,
                "default",
                &data_key_ciphertext,
            ),
            binding(
                "https://api.example.test",
                Uuid::from_u128(3),
                "default",
                &data_key_ciphertext,
            ),
            binding(
                "https://api.example.test",
                account,
                "other",
                &data_key_ciphertext,
            ),
            binding(
                "https://api.example.test",
                account,
                "default",
                &data_key_payload(0x46),
            ),
        ];

        for different in different_bindings {
            assert!(open_read_cache(&key, &different, &ciphertext).is_err());
        }
    }

    #[test]
    fn test_should_reject_tampering_and_trailing_ciphertext_bytes() {
        let key = SymmetricKey::new([0x33; 32]);
        let binding = binding(
            "https://api.example.test",
            Uuid::from_u128(4),
            "default",
            &data_key_payload(0x47),
        );
        let ciphertext = seal_read_cache(&key, &binding, &plaintext()).expect("seal read cache");

        let mut tampered = ciphertext.clone();
        let last = tampered.last_mut().expect("ciphertext byte");
        *last ^= 1;
        assert!(open_read_cache(&key, &binding, &tampered).is_err());

        let mut trailing = ciphertext;
        trailing.push(0);
        assert!(open_read_cache(&key, &binding, &trailing).is_err());
    }

    #[test]
    fn test_should_reject_invalid_or_trailing_plaintext_cbor() {
        let key = SymmetricKey::new([0x34; 32]);
        let binding = binding(
            "https://api.example.test",
            Uuid::from_u128(5),
            "default",
            &data_key_payload(0x48),
        );
        assert!(seal_read_cache(&key, &binding, b"not CBOR").is_err());

        let mut trailing = plaintext();
        trailing.push(0);
        assert!(seal_read_cache(&key, &binding, &trailing).is_err());
    }

    #[test]
    fn test_should_normalize_binding_inputs_and_redact_debug() {
        let data_key_ciphertext = data_key_payload(0x49);
        let account = Uuid::from_u128(6);
        let first = binding(
            " https://api.example.test/// ",
            account,
            " default ",
            &data_key_ciphertext,
        );
        let second = binding(
            "https://api.example.test",
            account,
            "default",
            &data_key_ciphertext,
        );

        assert_eq!(first, second);
        let rendered = format!("{first:?}");
        assert_eq!(rendered, "ReadCacheBinding { digest: \"<redacted>\" }");
        assert!(!rendered.contains("api.example"));
        assert!(!rendered.contains("default"));
    }

    #[test]
    fn test_should_reject_trailing_data_key_envelope_bytes() {
        let mut decoded = SealedPayload::new(vec![0x50; 48])
            .to_bytes()
            .expect("encode data-key envelope");
        decoded.push(0);
        let encoded = STANDARD_NO_PAD.encode(decoded);

        assert!(
            ReadCacheBinding::new(
                "https://api.example.test",
                Uuid::from_u128(7),
                "default",
                &encoded,
            )
            .is_err()
        );
    }

    #[test]
    fn test_should_enforce_binding_and_cache_size_limits_before_crypto() {
        let account = Uuid::from_u128(8);
        let data_key_ciphertext = data_key_payload(0x51);
        assert!(
            ReadCacheBinding::new(
                &format!("https://example.test/{}", "a".repeat(MAX_API_URL_BYTES)),
                account,
                "default",
                &data_key_ciphertext,
            )
            .is_err()
        );
        assert!(
            ReadCacheBinding::new(
                "https://api.example.test",
                account,
                &"a".repeat(MAX_PROFILE_NAME_BYTES + 1),
                &data_key_ciphertext,
            )
            .is_err()
        );
        assert!(
            ReadCacheBinding::new(
                "https://api.example.test",
                account,
                "default",
                &"A".repeat(MAX_DATA_KEY_CIPHERTEXT_B64_BYTES + 1),
            )
            .is_err()
        );

        let key = SymmetricKey::new([0x35; 32]);
        let binding = binding(
            "https://api.example.test",
            account,
            "default",
            &data_key_ciphertext,
        );
        let oversized_plaintext = vec![0; MAX_READ_CACHE_PLAINTEXT_BYTES + 1];
        assert!(seal_read_cache(&key, &binding, &oversized_plaintext).is_err());
        drop(oversized_plaintext);

        let oversized_ciphertext = vec![0; MAX_READ_CACHE_CIPHERTEXT_BYTES + 1];
        assert!(open_read_cache(&key, &binding, &oversized_ciphertext).is_err());
    }
}
