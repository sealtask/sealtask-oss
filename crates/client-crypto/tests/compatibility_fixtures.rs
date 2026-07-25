#[path = "../examples/support/compatibility.rs"]
mod compatibility;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD_NO_PAD;
use hkdf::Hkdf;
use sha2::Sha256;
use strong_box::StrongBox;

use compatibility::{CompatibilityCorpus, corpus_path, generate_corpus, generate_corpus_json};
use sealtask_client_crypto::{
    SealedPayload, StrongBoxKeyRing, SymmetricKey, compute_payload_proof,
    decode_attachment_blob_key, decrypt_attachment_bytes, decrypt_comment_payload,
    decrypt_note_payload, decrypt_task_payload, decrypt_user_data_key,
    decrypt_user_data_key_with_opaque_export_key, derive_payload_binding_key,
};

const CHECKED_IN_CORPUS: &str = include_str!("../../../testdata/crypto-compat-v1.json");

#[test]
fn generated_compatibility_corpus_matches_the_checked_in_file() {
    assert_eq!(
        std::fs::read_to_string(corpus_path()).expect("read compatibility corpus"),
        CHECKED_IN_CORPUS,
    );
    let generated = generate_corpus_json().expect("generate compatibility corpus");
    assert_eq!(
        generated, CHECKED_IN_CORPUS,
        "compatibility fixtures drifted; regenerate with `cargo run -p sealtask-client-crypto \
         --example generate_compat_fixtures -- --write`",
    );

    let checked_in: CompatibilityCorpus =
        serde_json::from_str(CHECKED_IN_CORPUS).expect("parse checked-in compatibility corpus");
    assert_eq!(
        checked_in,
        generate_corpus().expect("generate typed compatibility corpus"),
    );
}

#[test]
fn rust_crypto_consumes_the_compatibility_vectors() {
    let corpus: CompatibilityCorpus =
        serde_json::from_str(CHECKED_IN_CORPUS).expect("parse compatibility corpus");
    let expected_data_key = decode(&corpus.data_keys.data_key_b64);

    let password_data_key = decrypt_user_data_key(
        &corpus.data_keys.password_v1.password,
        &corpus.data_keys.password_v1.ciphertext_b64,
    )
    .expect("decrypt password data-key fixture");
    assert_eq!(password_data_key.as_bytes(), expected_data_key.as_slice());

    let opaque_data_key = decrypt_user_data_key_with_opaque_export_key(
        &decode(&corpus.data_keys.opaque_v2.export_key_b64),
        &corpus.data_keys.opaque_v2.ciphertext_b64,
    )
    .expect("decrypt OPAQUE data-key fixture");
    assert_eq!(opaque_data_key.as_bytes(), expected_data_key.as_slice());

    assert_export_wrapper(&corpus.data_keys.opaque_v2);
    assert_export_wrapper(&corpus.data_keys.recovery_v1);
    assert_recovery_wrapper(&corpus, &expected_data_key);
    assert_strong_box_frame(&corpus);
    assert_payload_proof(&corpus);
    assert_payloads(&corpus);
    assert_attachment(&corpus);
}

fn assert_export_wrapper(vector: &compatibility::ExportDataKeyVector) {
    let export_key = decode(&vector.export_key_b64);
    let mut derived = [0_u8; 32];
    Hkdf::<Sha256>::new(None, &export_key)
        .expand(vector.wrapping_info_utf8.as_bytes(), &mut derived)
        .expect("derive fixture wrapping key");
    assert_eq!(derived.as_slice(), decode(&vector.wrapping_key_b64));
}

fn assert_recovery_wrapper(corpus: &CompatibilityCorpus, expected_data_key: &[u8]) {
    let vector = &corpus.data_keys.recovery_v1;
    let payload =
        SealedPayload::from_bytes(&decode(&vector.ciphertext_b64)).expect("parse recovery wrapper");
    assert_eq!(payload.version, 2);
    let wrapping_key =
        SymmetricKey::from_slice(&decode(&vector.wrapping_key_b64)).expect("recovery wrapping key");
    let plaintext = StrongBoxKeyRing::new(wrapping_key)
        .strong_box()
        .decrypt(&payload.ciphertext, vector.context_utf8.as_bytes())
        .expect("decrypt recovery wrapper");
    assert_eq!(plaintext, expected_data_key);
}

fn assert_strong_box_frame(corpus: &CompatibilityCorpus) {
    let vector = &corpus.strong_box;
    let key = SymmetricKey::from_slice(&decode(&vector.key_b64)).expect("StrongBox fixture key");
    let plaintext = StrongBoxKeyRing::new(key)
        .strong_box()
        .decrypt(
            decode(&vector.ciphertext_b64),
            vector.context_utf8.as_bytes(),
        )
        .expect("decrypt StrongBox fixture");
    assert_eq!(plaintext, decode(&vector.plaintext_b64));
}

fn assert_payload_proof(corpus: &CompatibilityCorpus) {
    let vector = &corpus.payload_proof;
    let list_key =
        SymmetricKey::from_slice(&decode(&vector.list_key_b64)).expect("payload-proof list key");
    let binding_key = derive_payload_binding_key(&list_key).expect("derive payload binding key");
    assert_eq!(
        binding_key.as_bytes(),
        decode(&vector.binding_key_b64).as_slice(),
    );
    assert_eq!(
        compute_payload_proof(&decode(&vector.ciphertext_b64), &binding_key)
            .expect("compute payload proof"),
        vector.proof_b64,
    );
}

fn assert_payloads(corpus: &CompatibilityCorpus) {
    let list_key = SymmetricKey::from_slice(&decode(&corpus.payload_proof.list_key_b64))
        .expect("payload fixture list key");

    let task = decrypt_task_payload(&list_key, &decode(&corpus.payloads.task.sealed_payload_b64))
        .expect("decrypt task fixture");
    assert_eq!(task.kind, corpus.payloads.task.envelope.kind);
    assert_eq!(task.version, corpus.payloads.task.envelope.version);
    assert_eq!(task.body.title, corpus.payloads.task.envelope.body.title);
    assert_eq!(
        task.body
            .rich_text
            .expect("task rich text")
            .blocks
            .first()
            .expect("task rich-text block")
            .text,
        corpus.payloads.task.envelope.body.rich_text.blocks[0].text,
    );

    let comment = decrypt_comment_payload(
        &list_key,
        &decode(&corpus.payloads.comment.sealed_payload_b64),
    )
    .expect("decrypt comment fixture");
    assert_eq!(comment.kind, corpus.payloads.comment.envelope.kind);
    assert_eq!(
        comment
            .body
            .content
            .blocks
            .first()
            .expect("comment rich-text block")
            .text,
        corpus.payloads.comment.envelope.body.content.blocks[0].text,
    );

    let note = decrypt_note_payload(&list_key, &decode(&corpus.payloads.note.sealed_payload_b64))
        .expect("decrypt note fixture");
    assert_eq!(note.kind, corpus.payloads.note.envelope.kind);
    assert_eq!(note.body.title, corpus.payloads.note.envelope.body.title);
    assert_eq!(
        note.body
            .content
            .blocks
            .first()
            .expect("note rich-text block")
            .text,
        corpus.payloads.note.envelope.body.content.blocks[0].text,
    );
}

fn assert_attachment(corpus: &CompatibilityCorpus) {
    let vector = &corpus.attachment;
    let plaintext = decrypt_attachment_bytes(
        &decode(&vector.blob_ciphertext_b64),
        &decode(&vector.file_key_b64),
        Some(&vector.blob_context_utf8),
    )
    .expect("decrypt attachment fixture");
    assert_eq!(plaintext, decode(&vector.plaintext_b64));

    let list_key =
        SymmetricKey::from_slice(&decode(&vector.list_key_b64)).expect("attachment list key");
    let blob_ref = decode_attachment_blob_key(&list_key, &decode(&vector.blob_key_b64))
        .expect("decode attachment reference fixture");
    assert_eq!(blob_ref.version, vector.blob_ref.version);
    assert_eq!(blob_ref.ciphertext_bytes, vector.blob_ref.ciphertext_bytes,);
    assert_eq!(blob_ref.file_key, decode(&vector.blob_ref.file_key_b64));
    assert_eq!(blob_ref.enc_context, vector.blob_ref.enc_context);
}

fn decode(value: &str) -> Vec<u8> {
    STANDARD_NO_PAD
        .decode(value)
        .expect("fixture contains valid unpadded base64")
}
