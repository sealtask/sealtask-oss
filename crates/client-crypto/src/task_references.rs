use std::io::Cursor;

use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use uuid::Uuid;
use zeroize::Zeroizing;

use sealtask_client_core::{PublicError, PublicResult};

use super::{
    SealedBlobPayload, SealedPayload, SymmetricKey, decrypt_sealed_bytes, encrypt_sealed_bytes,
    serialize_to_cbor,
};

pub const TASK_REFERENCE_SCHEME_CONTEXT: &[u8] = b"worklist.task_reference_scheme.v1";

pub const TASK_REFERENCE_SCHEME_PLAINTEXT_BYTES: usize = 512;
pub const TASK_REFERENCE_SCHEME_AEAD_CIPHERTEXT_BYTES: usize =
    TASK_REFERENCE_SCHEME_PLAINTEXT_BYTES + 16;
pub const TASK_REFERENCE_SCHEME_STRONG_BOX_BYTES: usize = 565;
pub const TASK_REFERENCE_SCHEME_SEALED_PAYLOAD_BYTES: usize = 589;
pub const TASK_REFERENCE_PREFIX_MIN_BYTES: usize = 2;
pub const TASK_REFERENCE_PREFIX_MAX_BYTES: usize = 10;
pub const TASK_REFERENCE_MINIMUM_DIGITS_MIN: u8 = 1;
pub const TASK_REFERENCE_MINIMUM_DIGITS_MAX: u8 = 8;
pub const TASK_REFERENCE_ORDINARY_REVISION_MAX: i64 = 32;
pub const TASK_REFERENCE_REPAIR_REVISION_MAX: i64 = 4;
pub const TASK_REFERENCE_REVISION_MAX: i64 =
    TASK_REFERENCE_ORDINARY_REVISION_MAX + TASK_REFERENCE_REPAIR_REVISION_MAX;
pub const TASK_REFERENCE_SAFE_INTEGER_MAX: i64 = 9_007_199_254_740_991;

const TASK_REFERENCE_SCHEME_KIND: &str = "task_reference_scheme";
const TASK_REFERENCE_SCHEME_VERSION: u8 = 1;
const TASK_REFERENCE_SEPARATOR: &str = "-";

#[derive(Clone, Eq, PartialEq)]
pub struct TaskReferenceSchemeV1 {
    pub work_list_id: Uuid,
    pub scheme_revision_id: Uuid,
    pub revision: i64,
    pub prefix: String,
    pub minimum_digits: u8,
}

impl std::fmt::Debug for TaskReferenceSchemeV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TaskReferenceSchemeV1")
            .field("work_list_id", &self.work_list_id)
            .field("scheme_revision_id", &self.scheme_revision_id)
            .field("revision", &self.revision)
            .field("prefix", &"<redacted>")
            .field("minimum_digits", &self.minimum_digits)
            .finish()
    }
}

impl TaskReferenceSchemeV1 {
    pub fn new(
        work_list_id: Uuid,
        scheme_revision_id: Uuid,
        revision: i64,
        prefix: impl Into<String>,
        minimum_digits: u8,
    ) -> PublicResult<Self> {
        let scheme = Self {
            work_list_id,
            scheme_revision_id,
            revision,
            prefix: prefix.into(),
            minimum_digits,
        };
        scheme.validate()?;
        Ok(scheme)
    }

    pub fn validate(&self) -> PublicResult<()> {
        validate_task_reference_prefix(&self.prefix)?;
        if !(1..=TASK_REFERENCE_REVISION_MAX).contains(&self.revision) {
            return Err(PublicError::validation(format!(
                "task reference scheme revision must be between 1 and {TASK_REFERENCE_REVISION_MAX}"
            )));
        }
        if !(TASK_REFERENCE_MINIMUM_DIGITS_MIN..=TASK_REFERENCE_MINIMUM_DIGITS_MAX)
            .contains(&self.minimum_digits)
        {
            return Err(PublicError::validation(format!(
                "task reference minimum digits must be between {TASK_REFERENCE_MINIMUM_DIGITS_MIN} and {TASK_REFERENCE_MINIMUM_DIGITS_MAX}"
            )));
        }
        Ok(())
    }

    pub fn format_reference(&self, reference_number: i64) -> PublicResult<String> {
        self.validate()?;
        if !(1..=TASK_REFERENCE_SAFE_INTEGER_MAX).contains(&reference_number) {
            return Err(PublicError::validation(format!(
                "task reference number must be between 1 and {TASK_REFERENCE_SAFE_INTEGER_MAX}"
            )));
        }
        Ok(format!(
            "{}{TASK_REFERENCE_SEPARATOR}{reference_number:0width$}",
            self.prefix,
            width = usize::from(self.minimum_digits)
        ))
    }

    pub fn parse_reference_number(&self, reference: &str) -> Option<i64> {
        if self.validate().is_err() {
            return None;
        }
        let (prefix, number) = reference.trim().rsplit_once(TASK_REFERENCE_SEPARATOR)?;
        let prefix = prefix.trim();
        let number = number.trim();
        if !prefix.eq_ignore_ascii_case(&self.prefix)
            || number.is_empty()
            || !number.bytes().all(|byte| byte.is_ascii_digit())
        {
            return None;
        }
        number
            .parse::<i64>()
            .ok()
            .filter(|number| (1..=TASK_REFERENCE_SAFE_INTEGER_MAX).contains(number))
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TaskReferenceSchemeWireV1 {
    kind: String,
    version: u8,
    work_list_id: String,
    scheme_revision_id: String,
    revision: i64,
    prefix: String,
    separator: String,
    minimum_digits: u8,
    #[serde(with = "serde_bytes")]
    padding: Vec<u8>,
}

pub fn encrypt_task_reference_scheme(
    scheme: &TaskReferenceSchemeV1,
    list_key: &SymmetricKey,
) -> PublicResult<SealedBlobPayload> {
    let plaintext = Zeroizing::new(encode_task_reference_scheme_with_rng(scheme, &mut OsRng)?);
    let sealed = encrypt_sealed_bytes(
        &plaintext,
        list_key,
        TASK_REFERENCE_SCHEME_CONTEXT,
        "failed to seal task reference scheme",
    )?;
    ensure_scheme_strong_box_size(&sealed.bytes)?;
    Ok(sealed)
}

pub fn decrypt_task_reference_scheme(
    list_key: &SymmetricKey,
    payload_ciphertext: &[u8],
    expected_work_list_id: Uuid,
    expected_scheme_revision_id: Uuid,
    expected_revision: i64,
) -> PublicResult<TaskReferenceSchemeV1> {
    ensure_scheme_strong_box_size(payload_ciphertext)?;
    let plaintext = Zeroizing::new(decrypt_sealed_bytes(
        list_key,
        payload_ciphertext,
        TASK_REFERENCE_SCHEME_CONTEXT,
        "failed to decrypt task reference scheme",
    )?);
    decode_task_reference_scheme(
        &plaintext,
        expected_work_list_id,
        expected_scheme_revision_id,
        expected_revision,
    )
}

fn encode_task_reference_scheme_with_rng(
    scheme: &TaskReferenceSchemeV1,
    rng: &mut impl RngCore,
) -> PublicResult<Vec<u8>> {
    scheme.validate()?;
    let mut wire = TaskReferenceSchemeWireV1 {
        kind: TASK_REFERENCE_SCHEME_KIND.to_string(),
        version: TASK_REFERENCE_SCHEME_VERSION,
        work_list_id: scheme.work_list_id.to_string(),
        scheme_revision_id: scheme.scheme_revision_id.to_string(),
        revision: scheme.revision,
        prefix: scheme.prefix.clone(),
        separator: TASK_REFERENCE_SEPARATOR.to_string(),
        minimum_digits: scheme.minimum_digits,
        padding: Vec::new(),
    };
    let padding_bytes = find_padding_bytes(&mut wire)?;
    wire.padding.resize(padding_bytes, 0);
    rng.try_fill_bytes(&mut wire.padding)
        .map_err(|err| PublicError::crypto(format!("failed to generate scheme padding: {err}")))?;
    let encoded = serialize_to_cbor(&wire)?;
    if encoded.len() != TASK_REFERENCE_SCHEME_PLAINTEXT_BYTES {
        return Err(PublicError::unexpected(
            "task reference scheme encoding did not reach its fixed size",
        ));
    }
    Ok(encoded)
}

fn find_padding_bytes(wire: &mut TaskReferenceSchemeWireV1) -> PublicResult<usize> {
    for padding_bytes in 0..=TASK_REFERENCE_SCHEME_PLAINTEXT_BYTES {
        wire.padding.resize(padding_bytes, 0);
        match serialize_to_cbor(wire)?.len() {
            TASK_REFERENCE_SCHEME_PLAINTEXT_BYTES => return Ok(padding_bytes),
            encoded if encoded > TASK_REFERENCE_SCHEME_PLAINTEXT_BYTES => break,
            _ => {}
        }
    }
    Err(PublicError::validation(
        "task reference scheme cannot fit the fixed-size v1 envelope",
    ))
}

fn decode_task_reference_scheme(
    plaintext: &[u8],
    expected_work_list_id: Uuid,
    expected_scheme_revision_id: Uuid,
    expected_revision: i64,
) -> PublicResult<TaskReferenceSchemeV1> {
    if plaintext.len() != TASK_REFERENCE_SCHEME_PLAINTEXT_BYTES {
        return Err(PublicError::validation(format!(
            "task reference scheme plaintext must be {TASK_REFERENCE_SCHEME_PLAINTEXT_BYTES} bytes"
        )));
    }
    let wire: TaskReferenceSchemeWireV1 = deserialize_map_from_cbor_exact(plaintext)?;
    validate_envelope_header(
        &wire.kind,
        TASK_REFERENCE_SCHEME_KIND,
        wire.version,
        TASK_REFERENCE_SCHEME_VERSION,
    )?;
    if wire.separator != TASK_REFERENCE_SEPARATOR {
        return Err(PublicError::validation(
            "task reference separator must be '-'",
        ));
    }

    let work_list_id = parse_uuid(&wire.work_list_id, "task reference work list id")?;
    let scheme_revision_id = parse_uuid(
        &wire.scheme_revision_id,
        "task reference scheme revision id",
    )?;
    ensure_uuid_matches(
        work_list_id,
        expected_work_list_id,
        "task reference work list",
    )?;
    ensure_uuid_matches(
        scheme_revision_id,
        expected_scheme_revision_id,
        "task reference scheme revision",
    )?;
    if wire.revision != expected_revision {
        return Err(PublicError::validation(
            "task reference scheme revision does not match its public metadata",
        ));
    }

    TaskReferenceSchemeV1::new(
        work_list_id,
        scheme_revision_id,
        wire.revision,
        wire.prefix,
        wire.minimum_digits,
    )
}

fn ensure_scheme_strong_box_size(payload_ciphertext: &[u8]) -> PublicResult<()> {
    if payload_ciphertext.len() != TASK_REFERENCE_SCHEME_SEALED_PAYLOAD_BYTES {
        return Err(PublicError::validation(format!(
            "serialized task reference scheme payload must be {TASK_REFERENCE_SCHEME_SEALED_PAYLOAD_BYTES} bytes"
        )));
    }
    let sealed = deserialize_sealed_payload_exact(payload_ciphertext)?;
    if sealed.version != SealedPayload::CURRENT_VERSION {
        return Err(PublicError::validation(format!(
            "unsupported sealed payload version {}",
            sealed.version
        )));
    }
    if sealed.ciphertext.len() != TASK_REFERENCE_SCHEME_STRONG_BOX_BYTES {
        return Err(PublicError::validation(format!(
            "task reference scheme StrongBox ciphertext must be {TASK_REFERENCE_SCHEME_STRONG_BOX_BYTES} bytes"
        )));
    }
    Ok(())
}

fn deserialize_from_cbor_exact<T: DeserializeOwned>(bytes: &[u8]) -> PublicResult<T> {
    let mut cursor = Cursor::new(bytes);
    let value = strong_box::ciborium::de::from_reader(&mut cursor)
        .map_err(|err| PublicError::crypto(format!("failed to deserialize payload: {err}")))?;
    if cursor.position() != bytes.len() as u64 {
        return Err(PublicError::validation(
            "encrypted payload contains trailing CBOR bytes",
        ));
    }
    Ok(value)
}

fn deserialize_map_from_cbor_exact<T: DeserializeOwned>(bytes: &[u8]) -> PublicResult<T> {
    if bytes.first().is_none_or(|byte| byte >> 5 != 5) {
        return Err(PublicError::validation(
            "encrypted payload must be a CBOR map",
        ));
    }
    deserialize_from_cbor_exact(bytes)
}

fn deserialize_sealed_payload_exact(bytes: &[u8]) -> PublicResult<SealedPayload> {
    let value: super::FlexibleValue = deserialize_from_cbor_exact(bytes)?;
    let super::FlexibleValue::Map(entries) = value else {
        return Err(PublicError::validation("sealed payload must be a CBOR map"));
    };
    if entries.len() != 2 {
        return Err(PublicError::validation(
            "sealed payload must contain exactly version and ciphertext",
        ));
    }

    let mut version = None;
    let mut ciphertext = None;
    for (key, value) in entries {
        let super::FlexibleValue::Text(key) = key else {
            return Err(PublicError::validation("sealed payload keys must be text"));
        };
        match (key.as_str(), value) {
            ("version", super::FlexibleValue::Integer(value)) if version.is_none() => {
                version = u8::try_from(i128::from(value)).ok();
                if version.is_none() {
                    return Err(PublicError::validation(
                        "sealed payload version must be an unsigned byte",
                    ));
                }
            }
            ("ciphertext", super::FlexibleValue::Bytes(value)) if ciphertext.is_none() => {
                ciphertext = Some(value);
            }
            ("version" | "ciphertext", _) => {
                return Err(PublicError::validation(
                    "sealed payload contains a duplicate or invalid field",
                ));
            }
            _ => {
                return Err(PublicError::validation(
                    "sealed payload contains an unknown field",
                ));
            }
        }
    }

    Ok(SealedPayload {
        version: version
            .ok_or_else(|| PublicError::validation("sealed payload version is required"))?,
        ciphertext: ciphertext
            .ok_or_else(|| PublicError::validation("sealed payload ciphertext is required"))?,
    })
}

fn validate_task_reference_prefix(prefix: &str) -> PublicResult<()> {
    let bytes = prefix.as_bytes();
    if !(TASK_REFERENCE_PREFIX_MIN_BYTES..=TASK_REFERENCE_PREFIX_MAX_BYTES).contains(&bytes.len()) {
        return Err(PublicError::validation(format!(
            "task reference prefix must contain between {TASK_REFERENCE_PREFIX_MIN_BYTES} and {TASK_REFERENCE_PREFIX_MAX_BYTES} ASCII characters"
        )));
    }
    if !bytes[0].is_ascii_uppercase()
        || !bytes
            .iter()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
    {
        return Err(PublicError::validation(
            "task reference prefix must start with an uppercase ASCII letter and contain only uppercase ASCII letters or digits",
        ));
    }
    Ok(())
}

fn validate_envelope_header(
    actual_kind: &str,
    expected_kind: &str,
    actual_version: u8,
    expected_version: u8,
) -> PublicResult<()> {
    if actual_kind != expected_kind {
        return Err(PublicError::validation(format!(
            "encrypted payload kind must be {expected_kind}"
        )));
    }
    if actual_version != expected_version {
        return Err(PublicError::validation(format!(
            "unsupported {expected_kind} version {actual_version}"
        )));
    }
    Ok(())
}

fn parse_uuid(value: &str, field: &str) -> PublicResult<Uuid> {
    let parsed = Uuid::parse_str(value)
        .map_err(|err| PublicError::validation(format!("{field} must be a UUID: {err}")))?;
    if parsed.to_string() != value
        || parsed.get_version_num() == 0
        || parsed.get_version_num() > 8
        || parsed.get_variant() != uuid::Variant::RFC4122
    {
        return Err(PublicError::validation(format!(
            "{field} must be a canonical RFC 4122 UUID"
        )));
    }
    Ok(parsed)
}

fn ensure_uuid_matches(actual: Uuid, expected: Uuid, entity: &str) -> PublicResult<()> {
    if actual != expected {
        return Err(PublicError::validation(format!(
            "{entity} identity does not match its public metadata"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{Engine as _, engine::general_purpose::STANDARD};

    fn work_list_id() -> Uuid {
        Uuid::parse_str("11111111-1111-7111-8111-111111111111").expect("work list UUID")
    }

    fn scheme_revision_id() -> Uuid {
        Uuid::parse_str("22222222-2222-7222-8222-222222222222").expect("revision UUID")
    }

    fn other_id() -> Uuid {
        Uuid::parse_str("44444444-4444-7444-8444-444444444444").expect("other UUID")
    }

    fn scheme(prefix: &str, revision: i64, minimum_digits: u8) -> TaskReferenceSchemeV1 {
        TaskReferenceSchemeV1::new(
            work_list_id(),
            scheme_revision_id(),
            revision,
            prefix,
            minimum_digits,
        )
        .expect("valid scheme")
    }

    fn malformed_outer_variants(payload: &[u8]) -> Vec<Vec<u8>> {
        let sealed = deserialize_sealed_payload_exact(payload).expect("valid sealed payload");
        let version = super::super::FlexibleValue::Integer(sealed.version.into());
        let ciphertext = super::super::FlexibleValue::Bytes(sealed.ciphertext);
        let mut trailing = payload.to_vec();
        trailing.push(0);
        vec![
            serialize_to_cbor(&super::super::FlexibleValue::Array(vec![
                version.clone(),
                ciphertext.clone(),
            ]))
            .expect("array outer"),
            serialize_to_cbor(&super::super::FlexibleValue::Map(vec![
                (
                    super::super::FlexibleValue::Text("version".to_string()),
                    version.clone(),
                ),
                (
                    super::super::FlexibleValue::Text("ciphertext".to_string()),
                    ciphertext.clone(),
                ),
                (
                    super::super::FlexibleValue::Text("future".to_string()),
                    super::super::FlexibleValue::Null,
                ),
            ]))
            .expect("unknown outer field"),
            serialize_to_cbor(&super::super::FlexibleValue::Map(vec![
                (
                    super::super::FlexibleValue::Text("version".to_string()),
                    version.clone(),
                ),
                (
                    super::super::FlexibleValue::Text("version".to_string()),
                    version,
                ),
                (
                    super::super::FlexibleValue::Text("ciphertext".to_string()),
                    ciphertext,
                ),
            ]))
            .expect("duplicate outer field"),
            trailing,
        ]
    }

    fn deterministic_scheme_plaintext(scheme: &TaskReferenceSchemeV1) -> Vec<u8> {
        let mut wire = TaskReferenceSchemeWireV1 {
            kind: TASK_REFERENCE_SCHEME_KIND.to_string(),
            version: TASK_REFERENCE_SCHEME_VERSION,
            work_list_id: scheme.work_list_id.to_string(),
            scheme_revision_id: scheme.scheme_revision_id.to_string(),
            revision: scheme.revision,
            prefix: scheme.prefix.clone(),
            separator: TASK_REFERENCE_SEPARATOR.to_string(),
            minimum_digits: scheme.minimum_digits,
            padding: Vec::new(),
        };
        let padding_bytes = find_padding_bytes(&mut wire).expect("padding length");
        wire.padding = vec![0xa5; padding_bytes];
        serialize_to_cbor(&wire).expect("deterministic scheme plaintext")
    }

    #[test]
    fn task_reference_scheme_round_trips_with_fixed_ciphertext_size() {
        let key = SymmetricKey::new([0x51; 32]);
        let shortest = scheme("AB", 1, 1);
        let longest = scheme("A123456789", TASK_REFERENCE_REVISION_MAX, 8);

        let shortest_sealed =
            encrypt_task_reference_scheme(&shortest, &key).expect("encrypt shortest scheme");
        let longest_sealed =
            encrypt_task_reference_scheme(&longest, &key).expect("encrypt longest scheme");

        assert_eq!(shortest_sealed.bytes.len(), longest_sealed.bytes.len());
        assert_eq!(
            shortest_sealed.bytes.len(),
            TASK_REFERENCE_SCHEME_SEALED_PAYLOAD_BYTES
        );
        for sealed in [&shortest_sealed, &longest_sealed] {
            let outer: SealedPayload =
                deserialize_from_cbor_exact(&sealed.bytes).expect("sealed payload");
            assert_eq!(
                outer.ciphertext.len(),
                TASK_REFERENCE_SCHEME_STRONG_BOX_BYTES
            );
        }
        assert_eq!(
            decrypt_task_reference_scheme(
                &key,
                &shortest_sealed.bytes,
                shortest.work_list_id,
                shortest.scheme_revision_id,
                shortest.revision,
            )
            .expect("decrypt shortest scheme"),
            shortest
        );
        assert_eq!(
            decrypt_task_reference_scheme(
                &key,
                &longest_sealed.bytes,
                longest.work_list_id,
                longest.scheme_revision_id,
                longest.revision,
            )
            .expect("decrypt longest scheme"),
            longest
        );
    }

    #[test]
    fn task_reference_scheme_plaintext_has_a_cross_client_vector() {
        let plaintext = deterministic_scheme_plaintext(&scheme("LAW", 1, 4));
        assert_eq!(plaintext.len(), TASK_REFERENCE_SCHEME_PLAINTEXT_BYTES);
        assert_eq!(
            STANDARD.encode(&plaintext),
            concat!(
                "qWRraW5kdXRhc2tfcmVmZXJlbmNlX3NjaGVtZWd2ZXJzaW9uAWx3b3JrX2xpc3RfaWR4JDExMTExMTExLTExMTEtNzExMS04MTEx",
                "LTExMTExMTExMTExMXJzY2hlbWVfcmV2aXNpb25faWR4JDIyMjIyMjIyLTIyMjItNzIyMi04MjIyLTIyMjIyMjIyMjIyMmhyZXZp",
                "c2lvbgFmcHJlZml4Y0xBV2lzZXBhcmF0b3JhLW5taW5pbXVtX2RpZ2l0cwRncGFkZGluZ1kBM6WlpaWlpaWlpaWlpaWlpaWlpaWl",
                "paWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWl",
                "paWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWl",
                "paWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWl",
                "paWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaU=",
            )
        );
        // This shorter assertion makes an accidental return to the previously
        // truncated fixture fail at a named boundary as well.
        assert_eq!(
            STANDARD.encode(&plaintext[..485]),
            "qWRraW5kdXRhc2tfcmVmZXJlbmNlX3NjaGVtZWd2ZXJzaW9uAWx3b3JrX2xpc3RfaWR4JDExMTExMTExLTExMTEtNzExMS04MTExLTExMTExMTExMTExMXJzY2hlbWVfcmV2aXNpb25faWR4JDIyMjIyMjIyLTIyMjItNzIyMi04MjIyLTIyMjIyMjIyMjIyMmhyZXZpc2lvbgFmcHJlZml4Y0xBV2lzZXBhcmF0b3JhLW5taW5pbXVtX2RpZ2l0cwRncGFkZGluZ1kBM6WlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaU="
        );
    }

    #[test]
    fn task_reference_scheme_rejects_invalid_grammar_and_identity() {
        for prefix in ["A", "ABCDEFGHIJK", "1A", "Law", "A-", "ÄB"] {
            assert!(
                TaskReferenceSchemeV1::new(work_list_id(), scheme_revision_id(), 1, prefix, 1,)
                    .is_err(),
                "{prefix:?} must be rejected"
            );
        }
        assert!(
            TaskReferenceSchemeV1::new(work_list_id(), scheme_revision_id(), 0, "LAW", 1).is_err()
        );
        assert!(
            TaskReferenceSchemeV1::new(
                work_list_id(),
                scheme_revision_id(),
                TASK_REFERENCE_REVISION_MAX + 1,
                "LAW",
                1,
            )
            .is_err()
        );
        assert!(
            parse_uuid(
                &"aaaaaaaa-aaaa-7aaa-8aaa-aaaaaaaaaaaa".to_uppercase(),
                "test UUID",
            )
            .is_err()
        );

        let key = SymmetricKey::new([0x52; 32]);
        let value = scheme("LAW", 1, 3);
        let sealed = encrypt_task_reference_scheme(&value, &key).expect("encrypt scheme");
        assert!(
            decrypt_task_reference_scheme(
                &key,
                &sealed.bytes,
                other_id(),
                value.scheme_revision_id,
                value.revision,
            )
            .is_err()
        );
        assert!(
            decrypt_task_reference_scheme(
                &key,
                &sealed.bytes,
                value.work_list_id,
                value.scheme_revision_id,
                value.revision + 1,
            )
            .is_err()
        );
    }

    #[test]
    fn task_reference_scheme_formats_and_parses_locally() {
        let value = scheme("LAW", 1, 4);
        assert_eq!(value.format_reference(31).expect("format"), "LAW-0031");
        assert_eq!(value.parse_reference_number("law-31"), Some(31));
        assert_eq!(value.parse_reference_number(" LAW-0031 "), Some(31));
        assert_eq!(value.parse_reference_number(" law - 00031 "), Some(31));
        assert_eq!(value.parse_reference_number("LAW\t-\n31"), Some(31));
        assert_eq!(value.parse_reference_number("TAX-31"), None);
        assert_eq!(value.parse_reference_number("LAW--31"), None);
        assert_eq!(value.parse_reference_number("LAW-0"), None);
    }

    #[test]
    fn task_reference_scheme_rejects_wrong_context_and_non_fixed_size() {
        let key = SymmetricKey::new([0x53; 32]);
        let value = scheme("LAW", 1, 1);
        let wrong_context = super::super::encrypt_sealed_bytes(
            &encode_task_reference_scheme_with_rng(&value, &mut OsRng).expect("encode"),
            &key,
            b"worklist.task_reference_scheme.wrong_test_context",
            "encrypt test payload",
        )
        .expect("encrypt");
        assert!(
            decrypt_task_reference_scheme(
                &key,
                &wrong_context.bytes,
                value.work_list_id,
                value.scheme_revision_id,
                value.revision,
            )
            .is_err()
        );

        let variable_size = super::super::encrypt_sealed_payload(
            &serde_json::json!({"prefix": "LAW"}),
            &key,
            TASK_REFERENCE_SCHEME_CONTEXT,
            "encrypt test payload",
        )
        .expect("encrypt");
        assert!(
            decrypt_task_reference_scheme(
                &key,
                &variable_size.bytes,
                value.work_list_id,
                value.scheme_revision_id,
                value.revision,
            )
            .is_err()
        );
    }

    #[test]
    fn task_reference_scheme_rejects_noncanonical_outer_sealed_payloads() {
        let key = SymmetricKey::new([0x55; 32]);
        let scheme = scheme("OPS", 1, 4);
        let sealed_scheme = encrypt_task_reference_scheme(&scheme, &key).expect("encrypted scheme");
        for malformed in malformed_outer_variants(&sealed_scheme.bytes) {
            assert!(
                decrypt_task_reference_scheme(
                    &key,
                    &malformed,
                    scheme.work_list_id,
                    scheme.scheme_revision_id,
                    scheme.revision,
                )
                .is_err()
            );
        }
    }
}
