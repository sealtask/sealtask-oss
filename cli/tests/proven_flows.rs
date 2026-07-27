use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt as _;
use std::path::Path as FsPath;
use std::process::{Child, Stdio};
use std::sync::{Arc, Mutex};

use assert_cmd::Command;
use axum::{
    Json, Router,
    body::Bytes,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, patch, post, put},
};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD_NO_PAD;
use chrono::{DateTime, Duration, Utc};
use hkdf::Hkdf;
use opaque_ke::{
    ClientRegistration, ClientRegistrationFinishParameters, CredentialRequest, Identifiers,
    RegistrationResponse, ServerLogin, ServerLoginParameters, ServerRegistration, ServerSetup,
};
use rand_core::OsRng;
use sealtask_client_api::{AuditPatchFieldRequest, AuditPatchRequest, CONTROL_PLANE_USER_AGENT};
use sealtask_client_auth::{ClientCipherSuite, Credentials};
use sealtask_client_crypto::{
    ATTACHMENT_BLOB_CONTEXT, ATTACHMENT_BLOB_CONTEXT_LABEL, ATTACHMENT_BLOB_REF_VERSION,
    AttachmentBlobRef, CommentPayloadBody, FlexibleValue, NOTE_TITLE_CONTEXT,
    OPAQUE_EXPORT_KEY_BYTES, SealedPayload, StrongBoxKeyRing, SymmetricKey, TASK_TITLE_CONTEXT,
    TaskPayloadBody, USER_DATA_KEY_CONTEXT, USER_DATA_KEY_OPAQUE_CONTEXT,
    USER_DATA_KEY_OPAQUE_WRAP_INFO, WORK_LIST_MEMBERSHIP_CONTEXT, WORK_LIST_PAYLOAD_CONTEXT,
    build_comment_payload_envelope, build_task_payload_envelope, compute_payload_proof,
    decode_attachment_blob_key, decrypt_attachment_bytes, decrypt_comment_payload,
    decrypt_encrypted_text_value, decrypt_note_key, decrypt_note_payload, decrypt_task_payload,
    derive_payload_binding_key,
    encode_attachment_blob_key as encode_production_attachment_blob_key, encrypt_comment_payload,
    encrypt_task_payload, flexible_value_to_json, json_value_to_flexible, plaintext_rich_text,
    seal_text_value, serialize_to_cbor,
};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::Sha256;
use strong_box::StrongBox;
use tempfile::TempDir;
use tokio::net::TcpListener;
use tokio::sync::Notify;
use uuid::Uuid;
use zeroize::Zeroize;

const CLI_READINESS_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);
const MAX_CHILD_DIAGNOSTIC_BYTES: usize = 4 * 1024;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn d6_project_audit_excludes_encrypted_payload_canaries() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--format",
            "jsonl",
            "projects",
            "audit",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
        ],
        None,
    );

    assert!(output.status.success(), "audit failed: {}", output.stderr);
    assert_eq!(output.stdout.lines().count(), 1);
    assert!(output.stderr.is_empty());
    assert!(!output.stdout.contains("D6_PAYLOAD_CIPHERTEXT_CANARY"));
    assert!(!output.stdout.contains("D6_PAYLOAD_HMAC_CANARY"));
    let page = parse_stdout_json(&output.stdout);
    assert_eq!(page["projectId"], fixture.work_list_id.to_string());
    assert_eq!(page["events"][0]["payloadPresent"], true);
    assert!(page["events"][0].get("payload").is_none());
    assert!(page["events"][0].get("payloadCiphertext").is_none());
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn d6_task_watch_subscribes_then_refetches_and_sigint_exits_130() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    state.lock().expect("state lock").d6_watch_enabled = true;
    let server = spawn_server(state.clone()).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let mut child = spawn_cli_process_with_stdin(
        home.path(),
        &server.base_url,
        &[
            "--format",
            "jsonl",
            "tasks",
            "watch",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--password-stdin",
        ],
        &fixture.password,
    );
    wait_for_cli_condition(
        &mut child,
        "watch refetches once after the queued event burst",
        || state.lock().expect("state lock").d6_watch_task_reads >= 2,
    )
    .await;
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    signal_process(&mut child, libc::SIGINT);
    let output = wait_for_cli_process(child).await;

    assert_eq!(
        output.status.code(),
        Some(130),
        "stdout: {}\nstderr: {}",
        output.stdout,
        output.stderr
    );
    assert!(!output.stdout.contains('\u{1b}'));
    assert!(!output.stdout.contains("D6_SECRET_STREAM_TOKEN"));
    assert!(!output.stdout.contains("D6_ACTOR_INSTANCE_CANARY"));
    let records = output
        .stdout
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("compact JSONL record"))
        .collect::<Vec<_>>();
    assert_eq!(records.len(), 2, "stdout: {}", output.stdout);
    assert_eq!(records[0]["type"], "snapshot");
    assert_eq!(records[0]["tasks"][0]["commentCount"], 1);
    assert_eq!(records[1]["type"], "refresh");
    assert_eq!(records[1]["trigger"], "resync");
    assert_eq!(records[1]["missedEvents"], 2);
    assert_eq!(records[1]["updatedTaskIds"][0], fixture.task_id.to_string());
    assert_eq!(
        records[1]["addedTaskIds"],
        json!([]),
        "filter transitions must use neutral diff names"
    );
    assert_eq!(records[1]["removedTaskIds"], json!([]));
    assert!(records[1].get("created").is_none());
    assert!(records[1].get("deletedTaskIds").is_none());
    assert_eq!(records[1]["tasks"][0]["commentCount"], 2);

    let state = state.lock().expect("state lock");
    assert_eq!(
        state.d6_watch_task_reads, 2,
        "the initial snapshot plus one authoritative refetch should cover the queued burst"
    );
    let call_order = &state.d6_watch_call_order;
    assert!(
        call_order.starts_with(&["token", "connect", "list"]),
        "SSE must connect before the initial task snapshot: {call_order:?}"
    );
    for line in output.stderr.lines() {
        let envelope: Value = serde_json::from_str(line).expect("compact JSON stderr envelope");
        assert!(envelope.is_object());
    }
    let final_error: Value = serde_json::from_str(
        output
            .stderr
            .lines()
            .last()
            .expect("typed interruption on stderr"),
    )
    .expect("interruption JSON");
    assert_eq!(final_error["error"]["code"], "interrupted");
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn d6_activity_is_oldest_first_and_sigterm_exits_130_without_stdout_status() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state.clone()).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let mut child = spawn_cli_process(
        home.path(),
        &server.base_url,
        &[
            "--format",
            "jsonl",
            "activity",
            "follow",
            "--since",
            "1h",
            "--interval",
            "250ms",
        ],
    );
    wait_for_cli_condition(&mut child, "activity initial page and poll", || {
        state.lock().expect("state lock").d6_activity_reads >= 2
    })
    .await;
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    signal_process(&mut child, libc::SIGTERM);
    let output = wait_for_cli_process(child).await;

    assert_eq!(
        output.status.code(),
        Some(130),
        "stdout: {}\nstderr: {}",
        output.stdout,
        output.stderr
    );
    assert!(!output.stdout.contains('\u{1b}'));
    let records = output
        .stdout
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("activity JSONL"))
        .collect::<Vec<_>>();
    assert_eq!(records.len(), 3, "stdout: {}", output.stdout);
    assert_eq!(
        records
            .iter()
            .map(|record| record["event"]["action"].as_str().expect("action"))
            .collect::<Vec<_>>(),
        ["updated_1", "updated_2", "updated_3"]
    );
    assert!(records.iter().all(|record| record["type"] == "activity"));
    let error = parse_stderr_json(&output.stderr);
    assert_eq!(error["error"]["code"], "interrupted");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_proven_flows_round_trip_through_mock_api() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state.clone()).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let list_detail_output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "lists",
            "get",
            &fixture.work_list_id.to_string(),
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        list_detail_output.status.success(),
        "lists get failed: {}",
        list_detail_output.stderr
    );
    let list_detail: Value = parse_stdout_json(&list_detail_output.stdout);
    assert_eq!(list_detail["title"], "Fixture Work List");
    assert!(list_detail.get("titleCiphertext").is_none());

    let create_task_output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "tasks",
            "create",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--input-file",
            write_json_file(
                home.path(),
                "task-create.json",
                &json!({
                    "title": "Created from test",
                    "body": "Created body"
                }),
            )
            .to_str()
            .expect("utf8 path"),
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        create_task_output.status.success(),
        "task create failed: {}",
        create_task_output.stderr
    );
    let created_task_json: Value = parse_stdout_json(&create_task_output.stdout);
    assert_eq!(created_task_json["title"], "Created from test");
    assert!(created_task_json.get("titleCiphertext").is_none());

    let update_task_output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--format",
            "json-pretty",
            "tasks",
            "update",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--task-id",
            &fixture.task_id.to_string(),
            "--title",
            "Updated title",
            "--body",
            "Updated body",
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        update_task_output.status.success(),
        "task update failed: {}",
        update_task_output.stderr
    );
    let updated_task_json: Value = parse_stdout_json(&update_task_output.stdout);
    assert_eq!(updated_task_json["title"], "Updated title");
    assert_eq!(updated_task_json["bodyMarkdown"], "Updated body");

    let move_section_id = Uuid::now_v7();
    let insert_before_task_id = Uuid::now_v7();
    let move_task_output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "tasks",
            "move",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--task-id",
            &fixture.task_id.to_string(),
            "--section-id",
            &move_section_id.to_string(),
            "--insert-before-task-id",
            &insert_before_task_id.to_string(),
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        move_task_output.status.success(),
        "task move failed: {}",
        move_task_output.stderr
    );
    let moved_task_json: Value = parse_stdout_json(&move_task_output.stdout);
    assert_eq!(moved_task_json["sectionId"], move_section_id.to_string());

    let archive_task_output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "tasks",
            "archive",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--task-id",
            &fixture.task_id.to_string(),
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        archive_task_output.status.success(),
        "task archive failed: {}",
        archive_task_output.stderr
    );
    let archived_task_json: Value = parse_stdout_json(&archive_task_output.stdout);
    assert!(archived_task_json["archivedAt"].is_string());

    let unarchive_task_output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "tasks",
            "unarchive",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--task-id",
            &fixture.task_id.to_string(),
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        unarchive_task_output.status.success(),
        "task unarchive failed: {}",
        unarchive_task_output.stderr
    );
    let unarchived_task_json: Value = parse_stdout_json(&unarchive_task_output.stdout);
    assert!(unarchived_task_json["archivedAt"].is_null());

    let comment_body_path = home.path().join("comment-body.md");
    std::fs::write(&comment_body_path, "New comment\n").expect("write comment body");
    let create_comment_output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "comments",
            "create",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--task-id",
            &fixture.task_id.to_string(),
            "--body-file",
            comment_body_path.to_str().expect("UTF-8 comment body path"),
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        create_comment_output.status.success(),
        "comment create failed: {}",
        create_comment_output.stderr
    );
    let created_comment_json: Value = parse_stdout_json(&create_comment_output.stdout);
    assert_eq!(created_comment_json["bodyMarkdown"], "New comment");

    let comment_id_prefix = fixture
        .comment_id
        .simple()
        .to_string()
        .chars()
        .take(8)
        .collect::<String>();
    let update_comment_output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "comments",
            "update",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--task-id",
            &fixture.task_id.to_string(),
            "--comment-id",
            &comment_id_prefix,
            "--body",
            "Updated comment",
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        update_comment_output.status.success(),
        "comment update failed: {}",
        update_comment_output.stderr
    );
    let updated_comment_json: Value = parse_stdout_json(&update_comment_output.stdout);
    assert_eq!(updated_comment_json["bodyMarkdown"], "Updated comment");

    let list_comments_output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "comments",
            "list",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--task-id",
            &fixture.task_id.to_string(),
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        list_comments_output.status.success(),
        "comment list failed: {}",
        list_comments_output.stderr
    );
    let list_comments_json: Value = parse_stdout_json(&list_comments_output.stdout);
    assert_eq!(list_comments_json[0]["bodyMarkdown"], "Existing comment");

    let (delete_comment_input, expected_delete_comment_audit_patch) = delete_input(
        "bodyCiphertextDigest",
        "comment-delete-ciphertext",
        "comment-delete-proof",
    );
    let delete_comment_output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "comments",
            "delete",
            "--yes",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--task-id",
            &fixture.task_id.to_string(),
            "--comment-id",
            &fixture.comment_id.to_string(),
            "--input-stdin",
        ],
        Some(&delete_comment_input.to_string()),
    );
    assert!(
        delete_comment_output.status.success(),
        "comment delete failed: {}",
        delete_comment_output.stderr
    );
    let deleted_comment_json: Value = parse_stdout_json(&delete_comment_output.stdout);
    assert_eq!(deleted_comment_json["deleted"], true);
    assert_eq!(
        deleted_comment_json["commentId"],
        fixture.comment_id.to_string()
    );

    let empty_comments_output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "comments",
            "list",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--task-id",
            &fixture.task_id.to_string(),
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        empty_comments_output.status.success(),
        "empty comment list failed: {}",
        empty_comments_output.stderr
    );
    let empty_comments_json: Value = parse_stdout_json(&empty_comments_output.stdout);
    assert_eq!(empty_comments_json, json!([]));

    let (delete_task_input, expected_delete_task_audit_patch) = delete_input(
        "payloadCiphertextDigest",
        "task-delete-ciphertext",
        "task-delete-proof",
    );
    let delete_task_output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "tasks",
            "delete",
            "--yes",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--task-id",
            &fixture.task_id.to_string(),
            "--input-stdin",
        ],
        Some(&delete_task_input.to_string()),
    );
    assert!(
        delete_task_output.status.success(),
        "task delete failed: {}",
        delete_task_output.stderr
    );
    let deleted_task_json: Value = parse_stdout_json(&delete_task_output.stdout);
    assert_eq!(deleted_task_json["deleted"], true);
    assert_eq!(deleted_task_json["taskId"], fixture.task_id.to_string());

    let state = state.lock().expect("state lock");
    let created_task = state
        .created_task_body
        .as_ref()
        .expect("created task body recorded");
    assert_eq!(created_task.title, "Created from test");

    let updated_task = state
        .updated_task_body
        .as_ref()
        .expect("updated task body recorded");
    assert_eq!(updated_task.title, "Updated title");
    assert_eq!(
        updated_task
            .checklist
            .as_ref()
            .expect("checklist preserved")
            .len(),
        1
    );
    assert_eq!(
        updated_task
            .attachments
            .as_ref()
            .expect("attachments preserved")
            .len(),
        4
    );
    assert_eq!(
        updated_task.mentions.as_ref().expect("mentions preserved")[0],
        fixture.mentioned_user_id.to_string()
    );
    let moved_task = state
        .moved_task_body
        .as_ref()
        .expect("moved task request recorded");
    assert_eq!(moved_task.section_id, Some(move_section_id));
    assert_eq!(moved_task.section_boundary, None);
    assert_eq!(
        moved_task.insert_before_task_id,
        Some(insert_before_task_id)
    );
    assert_eq!(state.archive_task_count, 1);
    assert_eq!(state.unarchive_task_count, 1);

    let created_comment = state
        .created_comment_body
        .as_ref()
        .expect("created comment body recorded");
    assert_eq!(created_comment.content.blocks[0].text, "New comment");

    let updated_comment = state
        .updated_comment_body
        .as_ref()
        .expect("updated comment body recorded");
    assert_eq!(updated_comment.content.blocks[0].text, "Updated comment");
    assert_eq!(
        updated_comment
            .mentions
            .as_ref()
            .expect("comment mentions preserved")[0],
        fixture.mentioned_user_id.to_string()
    );
    assert_eq!(
        updated_comment
            .attachments
            .as_ref()
            .expect("comment attachments preserved")
            .len(),
        1
    );
    assert_eq!(
        flexible_value_to_json(
            updated_comment
                .client_meta
                .as_ref()
                .expect("comment client meta preserved")
                .clone(),
        )["source"],
        "fixture"
    );
    assert_eq!(state.list_comments_count, 4);
    assert_eq!(state.deleted_comment_id, Some(fixture.comment_id));
    assert_eq!(state.deleted_task_id, Some(fixture.task_id));
    assert_eq!(
        state.deleted_comment_audit_patch.as_ref(),
        Some(&expected_delete_comment_audit_patch)
    );
    assert_eq!(
        state.deleted_task_audit_patch.as_ref(),
        Some(&expected_delete_task_audit_patch)
    );
}

#[test]
fn cli_schema_canonicalizes_lists_alias_to_projects() {
    let home = TempDir::new().expect("temp home");
    let projects = run_cli_exact(home.path(), &["--json", "schema", "projects"], None);
    let lists = run_cli_exact(home.path(), &["--json", "schema", "lists"], None);

    assert!(
        projects.status.success(),
        "projects schema failed: {}",
        projects.stderr
    );
    assert!(
        lists.status.success(),
        "lists schema failed: {}",
        lists.stderr
    );
    let projects_schema = parse_stdout_json(&projects.stdout);
    let lists_schema = parse_stdout_json(&lists.stdout);
    assert_eq!(projects_schema, lists_schema);
    assert_eq!(projects_schema["name"], "projects");
    assert!(
        projects_schema["usage"]
            .as_str()
            .expect("schema usage")
            .starts_with("Usage: sealtask projects")
    );
}

#[test]
fn cli_task_list_composable_controls_fail_before_authentication() {
    let home = TempDir::new().expect("temp home");
    for (args, expected) in [
        (
            &["--json", "tasks", "list", "--field", "id"][..],
            "cannot be combined with --json",
        ),
        (
            &["--pager", "always", "tasks", "list", "--field", "id"][..],
            "paging is unavailable for this raw-output command",
        ),
        (
            &["tasks", "list", "--web-url", "https://app.example"][..],
            "only used with '--field url'",
        ),
        (
            &[
                "tasks",
                "list",
                "--field",
                "url",
                "--web-url",
                "file:///tmp/app",
            ][..],
            "must use HTTP or HTTPS",
        ),
        (
            &["tasks", "list", "--columns", "id,title,id"][..],
            "duplicate task-list column 'id'",
        ),
    ] {
        let output = run_cli_exact(home.path(), args, None);
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert!(output.stdout.is_empty(), "{args:?}: {}", output.stdout);
        assert!(
            output.stderr.contains(expected),
            "{args:?}: {}",
            output.stderr
        );
        assert!(
            !output.stderr.contains("authentication"),
            "{args:?}: {}",
            output.stderr
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_task_list_columns_and_fields_are_composable() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let configured_web_origin = run_cli_with_operator_env(
        home.path(),
        &server.base_url,
        &["tasks", "list", "--password-stdin"],
        &[("SEALTASK_WEB_URL", "https://app.example")],
        Some(&fixture.password),
    );
    assert!(
        configured_web_origin.status.success(),
        "SEALTASK_WEB_URL poisoned ordinary list output: {}",
        configured_web_origin.stderr
    );
    assert!(configured_web_origin.stdout.contains("Existing task"));

    let table = run_cli(
        home.path(),
        &server.base_url,
        &[
            "tasks",
            "list",
            "--all",
            "--columns",
            "project,title,id",
            "--sort",
            "title",
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        table.status.success(),
        "task table failed: {}",
        table.stderr
    );
    let header = table.stdout.lines().next().expect("table header");
    let project = header.find("Project").expect("project header");
    let title = header.find("Title").expect("title header");
    let id = header.find("ID").expect("ID header");
    assert!(project < title && title < id, "{header}");
    assert!(table.stdout.contains("Fixture Work List"));
    assert!(table.stdout.contains("Existing task"));

    for (field, expected) in [
        ("id", format!("id:{}\n", fixture.task_id.simple())),
        ("title", "Existing task\n".to_string()),
        (
            "url",
            format!(
                "https://app.example/workspace/work-lists/{}?task={}\n",
                fixture.work_list_id, fixture.task_id
            ),
        ),
    ] {
        let mut args = vec![
            "tasks",
            "list",
            "--all",
            "--field",
            field,
            "--password-stdin",
        ];
        if field == "url" {
            args.extend(["--web-url", "https://app.example"]);
        }
        let output = run_cli(
            home.path(),
            &server.base_url,
            &args,
            Some(&fixture.password),
        );
        assert!(
            output.status.success(),
            "--field {field} failed: {}",
            output.stderr
        );
        assert_eq!(output.stdout, expected);
        assert!(output.stderr.is_empty());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_projects_alias_context_profiles_and_sections_are_operator_friendly() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);
    copy_credentials_to_profile(home.path(), "isolated");

    let canonical = run_cli(
        home.path(),
        &server.base_url,
        &["--json", "projects", "list", "--password-stdin"],
        Some(&fixture.password),
    );
    let alias = run_cli(
        home.path(),
        &server.base_url,
        &["--json", "lists", "list", "--password-stdin"],
        Some(&fixture.password),
    );
    let bare_alias = run_cli(
        home.path(),
        &server.base_url,
        &["lists", "--json", "--password-stdin"],
        Some(&fixture.password),
    );
    let parent_scoped_flag = run_cli(
        home.path(),
        &server.base_url,
        &["--json", "projects", "--password-stdin", "list"],
        Some(&fixture.password),
    );
    assert!(
        canonical.status.success(),
        "projects list failed: {}",
        canonical.stderr
    );
    assert!(
        alias.status.success(),
        "lists list failed: {}",
        alias.stderr
    );
    assert!(
        bare_alias.status.success(),
        "bare lists failed: {}",
        bare_alias.stderr
    );
    assert!(
        parent_scoped_flag.status.success(),
        "parent-scoped projects flag failed: {}",
        parent_scoped_flag.stderr
    );
    for output in [&canonical, &alias, &bare_alias, &parent_scoped_flag] {
        let projects = parse_stdout_json(&output.stdout);
        assert_eq!(projects.as_array().expect("projects array").len(), 1);
        assert_eq!(projects[0]["id"], fixture.work_list_id.to_string());
        assert_eq!(projects[0]["title"], "Fixture Work List");
        assert!(projects[0].get("titleCiphertext").is_none());
    }

    let canonical_details = run_cli(
        home.path(),
        &server.base_url,
        &["projects", "list", "--details", "--password-stdin"],
        Some(&fixture.password),
    );
    let legacy_list_verbose = run_cli(
        home.path(),
        &server.base_url,
        &["projects", "list", "--verbose", "--password-stdin"],
        Some(&fixture.password),
    );
    let legacy_parent_verbose = run_cli(
        home.path(),
        &server.base_url,
        &["projects", "--verbose", "--password-stdin", "list"],
        Some(&fixture.password),
    );
    for output in [
        &canonical_details,
        &legacy_list_verbose,
        &legacy_parent_verbose,
    ] {
        assert!(
            output.status.success(),
            "project details failed: {}",
            output.stderr
        );
        assert!(output.stderr.is_empty());
        assert!(output.stdout.contains("Project:"));
        assert!(output.stdout.contains("Workspace:"));
    }
    let legacy_unscoped_tasks = run_cli(
        home.path(),
        &server.base_url,
        &["--json", "tasks", "list", "--password-stdin"],
        Some(&fixture.password),
    );
    assert!(
        legacy_unscoped_tasks.status.success(),
        "legacy unscoped task listing failed: {}",
        legacy_unscoped_tasks.stderr
    );
    assert_eq!(
        parse_stdout_json(&legacy_unscoped_tasks.stdout)
            .as_array()
            .expect("legacy unscoped tasks")
            .len(),
        1
    );

    let parent_scoped_get = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "projects",
            "--password-stdin",
            "get",
            "Fixture Work List",
        ],
        Some(&fixture.password),
    );
    assert!(
        parent_scoped_get.status.success(),
        "parent-scoped password flag was not applied to get: {}",
        parent_scoped_get.stderr
    );
    assert_eq!(
        parse_stdout_json(&parent_scoped_get.stdout)["id"],
        fixture.work_list_id.to_string()
    );

    let select = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "projects",
            "use",
            "Fixture Work List",
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        select.status.success(),
        "projects use failed: {}",
        select.stderr
    );
    let selected = parse_stdout_json(&select.stdout);
    assert_eq!(selected["projectId"], fixture.work_list_id.to_string());
    assert_eq!(selected["changed"], true);

    let current = run_cli(
        home.path(),
        &server.base_url,
        &["--json", "projects", "current"],
        None,
    );
    assert!(
        current.status.success(),
        "projects current failed: {}",
        current.stderr
    );
    let current = parse_stdout_json(&current.stdout);
    assert_eq!(current["schemaVersion"], 1);
    assert_eq!(current["projectId"], fixture.work_list_id.to_string());
    assert_eq!(current["profile"], "default");

    let tasks_in_current_project = run_cli(
        home.path(),
        &server.base_url,
        &["--json", "tasks", "list", "--password-stdin"],
        Some(&fixture.password),
    );
    assert!(
        tasks_in_current_project.status.success(),
        "current-project task list failed: {}",
        tasks_in_current_project.stderr
    );
    let tasks_in_current_project = parse_stdout_json(&tasks_in_current_project.stdout);
    assert_eq!(
        tasks_in_current_project
            .as_array()
            .expect("current-project tasks")
            .len(),
        1
    );
    assert_eq!(
        tasks_in_current_project[0]["workListId"],
        fixture.work_list_id.to_string()
    );
    assert_eq!(tasks_in_current_project[0]["title"], "Existing task");

    let isolated = run_cli(
        home.path(),
        &server.base_url,
        &["--json", "--profile", "isolated", "projects", "current"],
        None,
    );
    assert!(
        isolated.status.success(),
        "isolated projects current failed: {}",
        isolated.stderr
    );
    let isolated = parse_stdout_json(&isolated.stdout);
    assert_eq!(isolated["profile"], "isolated");
    assert!(isolated["projectId"].is_null());

    let sections = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "projects",
            "sections",
            "list",
            "--project",
            "Fixture Work List",
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        sections.status.success(),
        "section listing failed: {}",
        sections.stderr
    );
    let sections = parse_stdout_json(&sections.stdout);
    assert_eq!(sections.as_array().expect("sections array").len(), 2);
    assert_eq!(sections[0]["id"], fixture.first_section_id.to_string());
    assert_eq!(sections[0]["name"], "Planning");
    assert_eq!(sections[0]["position"], 0);
    assert_eq!(sections[1]["id"], fixture.done_section_id.to_string());
    assert_eq!(sections[1]["name"], "Done");
    assert_eq!(sections[1]["position"], 1);

    let clear = run_cli(
        home.path(),
        &server.base_url,
        &["--json", "projects", "clear"],
        None,
    );
    assert!(
        clear.status.success(),
        "projects clear failed: {}",
        clear.stderr
    );
    let clear = parse_stdout_json(&clear.stdout);
    assert_eq!(clear["changed"], true);
    assert!(clear["projectId"].is_null());

    let cleared = run_cli(
        home.path(),
        &server.base_url,
        &["--json", "projects", "current"],
        None,
    );
    assert!(
        cleared.status.success(),
        "cleared projects current failed: {}",
        cleared.stderr
    );
    assert!(parse_stdout_json(&cleared.stdout)["projectId"].is_null());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_project_task_lists_include_archived_only_with_a_project_scope() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    state.lock().expect("state lock").task_archived_at = Some(Utc::now());
    let server = spawn_server(state.clone()).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let missing_scope = run_cli(
        home.path(),
        &server.base_url,
        &["--json", "tasks", "list", "--include-archived"],
        None,
    );
    assert_eq!(missing_scope.status.code(), Some(1));
    assert_json_error_contains(
        &missing_scope.stderr,
        "--include-archived requires --project, --work-list-id, or a current project",
    );

    let default_list = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "tasks",
            "list",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        default_list.status.success(),
        "default project task list failed: {}",
        default_list.stderr
    );
    assert_eq!(parse_stdout_json(&default_list.stdout), json!([]));

    let decrypted = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "tasks",
            "list",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--include-archived",
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        decrypted.status.success(),
        "archived decrypted task list failed: {}",
        decrypted.stderr
    );
    let decrypted = parse_stdout_json(&decrypted.stdout);
    assert_eq!(decrypted.as_array().expect("decrypted tasks").len(), 1);
    assert!(decrypted[0]["archivedAt"].is_string());

    let raw = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "tasks",
            "list",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--include-archived",
            "--raw",
        ],
        None,
    );
    assert!(
        raw.status.success(),
        "archived raw task list failed: {}",
        raw.stderr
    );
    let raw = parse_stdout_json(&raw.stdout);
    assert_eq!(raw.as_array().expect("raw tasks").len(), 1);
    assert!(raw[0]["archivedAt"].is_string());

    let select = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "projects",
            "use",
            "Fixture Work List",
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        select.status.success(),
        "selecting current project failed: {}",
        select.stderr
    );
    let current = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "tasks",
            "list",
            "--include-archived",
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        current.status.success(),
        "current-project archived task list failed: {}",
        current.stderr
    );
    assert_eq!(
        parse_stdout_json(&current.stdout)
            .as_array()
            .expect("current-project tasks")
            .len(),
        1
    );

    let missing_task = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "tasks",
            "get",
            "Missing task",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert_eq!(missing_task.status.code(), Some(1));
    assert_json_error_contains(
        &missing_task.stderr,
        "--include-completed --include-archived",
    );

    let all_conflict = run_cli(
        home.path(),
        &server.base_url,
        &["--json", "tasks", "list", "--all", "--include-archived"],
        None,
    );
    assert_eq!(all_conflict.status.code(), Some(2));
    assert_json_error_contains(&all_conflict.stderr, "--include-archived");
    assert_json_error_contains(&all_conflict.stderr, "--all");

    assert_eq!(
        state.lock().expect("state lock").list_task_include_archived,
        [false, true, true, true, true]
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn task_dry_runs_emit_secret_safe_plans_without_post_or_patch() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state.clone()).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let create_canary = "dry-run-create-plaintext-canary";
    let create = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "--quiet",
            "--non-interactive",
            "tasks",
            "create",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--title",
            create_canary,
            "--dry-run",
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        create.status.success(),
        "task create dry run failed: {}",
        create.stderr
    );
    assert!(!create.stdout.contains(create_canary));
    let create_plan = parse_stdout_json(&create.stdout);
    assert_eq!(create_plan["schemaVersion"], 1);
    assert_eq!(create_plan["type"], "taskMutationPlan");
    assert_eq!(create_plan["action"], "task.create");
    assert_eq!(create_plan["projectId"], fixture.work_list_id.to_string());
    assert_eq!(create_plan["wouldChange"], true);
    assert_eq!(create_plan["willMutate"], false);
    assert_eq!(create_plan["idempotencyProtected"], false);
    assert_eq!(create_plan["changedFields"], json!(["title"]));
    assert!(create_plan["changeCommitment"].is_string());
    assert!(
        state
            .lock()
            .expect("state lock")
            .created_task_request
            .is_none(),
        "dry run must not POST a task"
    );

    let quiet_table = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--quiet",
            "tasks",
            "create",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--title",
            create_canary,
            "--dry-run",
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        quiet_table.status.success(),
        "quiet table dry run failed: {}",
        quiet_table.stderr
    );
    assert!(quiet_table.stdout.contains("Task mutation dry run"));
    assert!(quiet_table.stdout.contains("Mutation sent: no"));
    assert!(!quiet_table.stdout.contains(create_canary));

    let update_canary = "dry-run-update-plaintext-canary";
    let update = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--format",
            "json-pretty",
            "tasks",
            "update",
            "--task-id",
            &fixture.task_id.to_string(),
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--title",
            update_canary,
            "--dry-run",
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        update.status.success(),
        "task update dry run failed: {}",
        update.stderr
    );
    assert!(!update.stdout.contains(update_canary));
    let update_plan = parse_stdout_json(&update.stdout);
    assert_eq!(update_plan["action"], "task.update");
    assert_eq!(update_plan["taskId"], fixture.task_id.to_string());
    assert_eq!(update_plan["changedFields"], json!(["title"]));
    assert_eq!(update_plan["wouldChange"], true);
    assert_eq!(update_plan["willMutate"], false);
    assert!(update_plan["expectedUpdatedAt"].is_string());
    assert!(
        state
            .lock()
            .expect("state lock")
            .updated_task_request
            .is_none(),
        "dry run must not PATCH a task"
    );

    let no_op = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "tasks",
            "update",
            "--task-id",
            &fixture.task_id.to_string(),
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--title",
            "Existing task",
            "--dry-run",
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        no_op.status.success(),
        "task no-op dry run failed: {}",
        no_op.stderr
    );
    let no_op_plan = parse_stdout_json(&no_op.stdout);
    assert_eq!(no_op_plan["wouldChange"], false);
    assert_eq!(no_op_plan["changedFieldCount"], 0);
    assert_eq!(no_op_plan["changedFields"], json!([]));
    assert!(
        state
            .lock()
            .expect("state lock")
            .updated_task_request
            .is_none(),
        "no-op dry run must not PATCH a task"
    );
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn d7_first_sigint_waits_for_a_definitive_task_create_response() {
    let fixture = TestFixture::new();
    let committed = Arc::new(Notify::new());
    let release_response = Arc::new(Notify::new());
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    {
        let mut state = state.lock().expect("state lock");
        state.d7_task_create_committed = Some(committed.clone());
        state.d7_task_create_release_response = Some(release_response.clone());
    }
    let server = spawn_server(state.clone()).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let mut child = spawn_cli_process_with_stdin(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "tasks",
            "create",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--title",
            "Graceful interrupt",
            "--password-stdin",
        ],
        &fixture.password,
    );
    tokio::time::timeout(std::time::Duration::from_secs(5), committed.notified())
        .await
        .expect("task create should commit before its response is released");
    signal_process(&mut child, libc::SIGINT);
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert!(
        child.try_wait().expect("poll CLI process").is_none(),
        "the first interrupt must keep waiting for a definitive response"
    );

    release_response.notify_one();
    let output = wait_for_cli_process(child).await;
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        output.stdout,
        output.stderr
    );
    assert_eq!(
        parse_stdout_json(&output.stdout)["title"],
        "Graceful interrupt"
    );
    let warnings = output
        .stderr
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("JSON warning envelope"))
        .collect::<Vec<_>>();
    assert_eq!(warnings.len(), 1, "stderr: {}", output.stderr);
    assert_eq!(
        warnings[0]["warnings"][0]["code"],
        "mutation_interruption_deferred"
    );
    assert!(
        state
            .lock()
            .expect("state lock")
            .created_task_request
            .is_some()
    );
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn d7_second_sigint_exits_130_with_an_ambiguous_mutation_outcome() {
    let fixture = TestFixture::new();
    let committed = Arc::new(Notify::new());
    let release_response = Arc::new(Notify::new());
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    {
        let mut state = state.lock().expect("state lock");
        state.d7_task_create_committed = Some(committed.clone());
        state.d7_task_create_release_response = Some(release_response.clone());
    }
    let server = spawn_server(state).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let mut child = spawn_cli_process_with_stdin(
        home.path(),
        &server.base_url,
        &[
            "--format",
            "json-pretty",
            "tasks",
            "create",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--title",
            "Ambiguous interrupt",
            "--password-stdin",
        ],
        &fixture.password,
    );
    tokio::time::timeout(std::time::Duration::from_secs(5), committed.notified())
        .await
        .expect("task create should commit before its response is released");
    let release_if_signals_are_missed = release_response.clone();
    let fallback_release = tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        release_if_signals_are_missed.notify_one();
    });
    signal_process(&mut child, libc::SIGINT);
    // Distinct pending Unix signals cannot coalesce. The monitor installs both
    // handlers synchronously and keeps both streams registered for the entire
    // operation, so this proves there is no listener re-registration gap.
    signal_process(&mut child, libc::SIGTERM);
    let output = wait_for_cli_process(child).await;
    release_response.notify_one();
    fallback_release.abort();

    assert_eq!(
        output.status.code(),
        Some(130),
        "stdout: {}\nstderr: {}",
        output.stdout,
        output.stderr
    );
    assert!(output.stdout.is_empty(), "stdout: {}", output.stdout);
    let envelope =
        serde_json::from_str::<Value>(&output.stderr).expect("one JSON-pretty stderr document");
    assert_eq!(
        envelope["warnings"][0]["code"],
        "mutation_interruption_deferred"
    );
    assert_eq!(envelope["error"]["code"], "interrupted");
    assert_eq!(envelope["error"]["outcome"], "ambiguous");
    assert_eq!(
        envelope["warnings"][1]["code"],
        "mutation_interruption_forced"
    );
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn d7_batch_forced_interrupt_reports_each_in_flight_post_and_patch_as_ambiguous() {
    let fixture = TestFixture::new();
    let create_committed = Arc::new(Notify::new());
    let create_release = Arc::new(Notify::new());
    let update_committed = Arc::new(Notify::new());
    let update_release = Arc::new(Notify::new());
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    {
        let mut state = state.lock().expect("state lock");
        state.d7_task_create_committed = Some(create_committed.clone());
        state.d7_task_create_release_response = Some(create_release.clone());
        state.d7_task_update_committed = Some(update_committed.clone());
        state.d7_task_update_release_response = Some(update_release.clone());
    }
    let server = spawn_server(state).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);
    let unlock = run_cli(
        home.path(),
        &server.base_url,
        &["auth", "unlock", "--ttl-seconds", "300", "--password-stdin"],
        Some(&fixture.password),
    );
    assert!(unlock.status.success(), "unlock failed: {}", unlock.stderr);

    let input = home.path().join("forced-batch.jsonl");
    std::fs::write(
        &input,
        format!(
            concat!(
                "{{\"schemaVersion\":1,\"operationId\":\"forced-create\",\"type\":\"task.create\",",
                "\"project\":\"id:{}\",\"input\":{{\"title\":\"forced-create-canary\"}}}}\n",
                "{{\"schemaVersion\":1,\"operationId\":\"forced-update\",\"type\":\"task.update\",",
                "\"project\":\"id:{}\",\"task\":\"id:{}\",",
                "\"input\":{{\"title\":\"forced-update-canary\"}}}}\n"
            ),
            fixture.work_list_id, fixture.work_list_id, fixture.task_id
        ),
    )
    .expect("write batch input");

    let mut child = spawn_cli_process(
        home.path(),
        &server.base_url,
        &[
            "--format",
            "jsonl",
            "batch",
            "run",
            "--input",
            input.to_str().expect("batch input path"),
            "--jobs",
            "2",
        ],
    );
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        tokio::join!(create_committed.notified(), update_committed.notified());
    })
    .await
    .expect("both batch mutations should commit before responses are released");

    let fallback_create_release = create_release.clone();
    let fallback_update_release = update_release.clone();
    let fallback_release = tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        fallback_create_release.notify_one();
        fallback_update_release.notify_one();
    });
    signal_process(&mut child, libc::SIGINT);
    signal_process(&mut child, libc::SIGTERM);
    let output = wait_for_cli_process(child).await;
    create_release.notify_one();
    update_release.notify_one();
    fallback_release.abort();
    let lock = run_cli(home.path(), &server.base_url, &["auth", "lock"], None);
    assert!(lock.status.success(), "lock failed: {}", lock.stderr);

    assert_eq!(
        output.status.code(),
        Some(130),
        "stdout: {}\nstderr: {}",
        output.stdout,
        output.stderr
    );
    assert!(!output.stdout.contains("forced-create-canary"));
    assert!(!output.stdout.contains("forced-update-canary"));
    assert!(!output.stderr.contains("forced-create-canary"));
    assert!(!output.stderr.contains("forced-update-canary"));

    let records = output
        .stdout
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("compact JSONL batch record"))
        .collect::<Vec<_>>();
    assert_eq!(records.len(), 3, "stdout: {}", output.stdout);
    for (record, operation_id) in records[..2].iter().zip(["forced-create", "forced-update"]) {
        assert_eq!(record["type"], "batch.operation");
        assert_eq!(record["operationId"], operation_id);
        assert_eq!(record["status"], "interrupted");
        assert_eq!(record["error"]["code"], "outcome_ambiguous");
    }
    assert_eq!(records[0]["operation"], "task.create");
    assert_eq!(records[1]["operation"], "task.update");
    assert_eq!(records[1]["taskId"], fixture.task_id.to_string());

    let summary = &records[2];
    assert_eq!(summary["type"], "batch.summary");
    assert_eq!(summary["total"], 2);
    assert_eq!(summary["succeeded"], 0);
    assert_eq!(summary["failed"], 0);
    assert_eq!(summary["notRun"], 0);
    assert_eq!(summary["interruptedCount"], 2);
    assert_eq!(summary["interrupted"], true);

    let error: Value = serde_json::from_str(&output.stderr).expect("typed JSONL stderr error");
    assert_eq!(error["error"]["code"], "interrupted");
    assert_eq!(error["error"]["outcome"], "ambiguous");
    assert!(
        !error["error"]["message"]
            .as_str()
            .expect("error message")
            .contains("checkpoint")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_task_selectors_natural_inputs_and_single_password_read_work_together() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state.clone()).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let by_name = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "tasks",
            "get",
            "Existing task",
            "--project",
            "Fixture Work List",
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        by_name.status.success(),
        "task name selector failed: {}",
        by_name.stderr
    );
    let by_name = parse_stdout_json(&by_name.stdout);
    assert_eq!(by_name["id"], fixture.task_id.to_string());
    assert_eq!(by_name["title"], "Existing task");

    let task_id_prefix = fixture
        .task_id
        .simple()
        .to_string()
        .chars()
        .take(8)
        .collect::<String>();
    let by_prefix = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "tasks",
            "get",
            &task_id_prefix,
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        by_prefix.status.success(),
        "task ID-prefix selector failed: {}",
        by_prefix.stderr
    );
    assert_eq!(
        parse_stdout_json(&by_prefix.stdout)["id"],
        fixture.task_id.to_string()
    );

    let before_create = Utc::now().date_naive();
    let create = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "tasks",
            "create",
            "--project",
            "Fixture Work List",
            "--title",
            "Natural input task",
            "--priority",
            "p1",
            "--due",
            "tomorrow",
            "--section",
            "Planning",
            "--idempotency-key",
            "operator:natural-inputs",
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    let after_create = Utc::now().date_naive();
    assert!(
        create.status.success(),
        "natural task create failed: {}",
        create.stderr
    );
    let created = parse_stdout_json(&create.stdout);
    assert_eq!(created["title"], "Natural input task");
    assert_eq!(created["priority"], 8);
    assert_eq!(created["sectionId"], fixture.first_section_id.to_string());
    let due_at = DateTime::parse_from_rfc3339(
        created["dueAt"]
            .as_str()
            .expect("created task due timestamp"),
    )
    .expect("valid due timestamp")
    .with_timezone(&Utc);
    assert_eq!(due_at.time(), chrono::NaiveTime::MIN);
    assert!(
        due_at.date_naive() == before_create + Duration::days(1)
            || due_at.date_naive() == after_create + Duration::days(1)
    );
    {
        let state = state.lock().expect("state lock");
        let request = state
            .created_task_request
            .as_ref()
            .expect("create request captured");
        assert_eq!(request["priority"], 8);
        assert_eq!(request["sectionId"], fixture.first_section_id.to_string());
        assert_eq!(request["idempotencyKey"], "operator:natural-inputs");
        assert_eq!(
            state
                .created_task_body
                .as_ref()
                .expect("created task body")
                .title,
            "Natural input task"
        );
    }

    let update = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "tasks",
            "update",
            "Existing task",
            "--project",
            "Fixture Work List",
            "--title",
            "Updated through selectors",
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        update.status.success(),
        "selector mutation did not reuse its single password input: {}",
        update.stderr
    );
    assert_eq!(
        parse_stdout_json(&update.stdout)["title"],
        "Updated through selectors"
    );
    assert_eq!(
        state
            .lock()
            .expect("state lock")
            .updated_task_body
            .as_ref()
            .expect("updated task body")
            .title,
        "Updated through selectors"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_destructive_task_selector_requires_explicit_confirmation() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state.clone()).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let unconfirmed = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "tasks",
            "delete",
            "Existing task",
            "--project",
            "Fixture Work List",
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert_eq!(unconfirmed.status.code(), Some(1));
    assert!(unconfirmed.stdout.is_empty());
    assert_json_error_contains(&unconfirmed.stderr, "requires --yes");
    assert_json_error_contains(&unconfirmed.stderr, "Existing task");
    assert!(state.lock().expect("state lock").deleted_task_id.is_none());

    let confirmed = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "tasks",
            "delete",
            "Existing task",
            "--project",
            "Fixture Work List",
            "--password-stdin",
            "--yes",
        ],
        Some(&fixture.password),
    );
    assert!(
        confirmed.status.success(),
        "confirmed selector delete failed: {}",
        confirmed.stderr
    );
    let confirmed = parse_stdout_json(&confirmed.stdout);
    assert_eq!(confirmed["deleted"], true);
    assert_eq!(confirmed["taskId"], fixture.task_id.to_string());
    assert_eq!(
        state.lock().expect("state lock").deleted_task_id,
        Some(fixture.task_id)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_unlock_accepts_compound_human_ttl() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let unlock = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "auth",
            "unlock",
            "--ttl",
            "1h30m",
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        unlock.status.success(),
        "compound TTL unlock failed: {}",
        unlock.stderr
    );
    let unlock = parse_stdout_json(&unlock.stdout);
    assert_eq!(unlock["unlocked"], true);
    assert_eq!(unlock["mode"], "daemon");
    assert_eq!(unlock["ttlSeconds"], 5_400);

    let lock = run_cli(home.path(), &server.base_url, &["auth", "lock"], None);
    assert!(lock.status.success(), "lock failed: {}", lock.stderr);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_should_round_trip_shared_and_private_notes_through_encrypted_api_contract() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state.clone()).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let create = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "notes",
            "create",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--title",
            "Private launch notes",
            "--body",
            "First paragraph\n\nSecond paragraph",
            "--private",
            "--idempotency-key",
            "agent:notes:private-launch",
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        create.status.success(),
        "note create failed: {}",
        create.stderr
    );
    let created = parse_stdout_json(&create.stdout);
    let note_id = Uuid::parse_str(created["id"].as_str().expect("note id")).expect("note UUID");
    assert_eq!(created["title"], "Private launch notes");
    assert_eq!(
        created["bodyMarkdown"],
        "First paragraph\n\nSecond paragraph"
    );
    assert_eq!(created["isPrivate"], true);
    assert!(created.get("titleCiphertext").is_none());
    {
        let state = state.lock().expect("state lock");
        let (commitment, stored_note_id) = state
            .note_create_operations
            .get("agent:notes:private-launch")
            .expect("explicit note idempotency key reached the API");
        assert_eq!(*stored_note_id, note_id);
        assert_eq!(commitment.len(), 43);
    }

    let list = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "notes",
            "list",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(list.status.success(), "note list failed: {}", list.stderr);
    let listed = parse_stdout_json(&list.stdout);
    assert_eq!(listed.as_array().expect("notes array").len(), 1);
    assert_eq!(listed[0]["id"], note_id.to_string());

    let update = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "notes",
            "update",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--note-id",
            &note_id.to_string(),
            "--title",
            "Updated private notes",
            "--body",
            "Updated body",
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        update.status.success(),
        "note update failed: {}",
        update.stderr
    );
    let updated = parse_stdout_json(&update.stdout);
    assert_eq!(updated["title"], "Updated private notes");
    assert_eq!(updated["bodyMarkdown"], "Updated body");

    let get = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "notes",
            "get",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--note-id",
            &note_id.to_string(),
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(get.status.success(), "note get failed: {}", get.stderr);
    assert_eq!(
        parse_stdout_json(&get.stdout)["title"],
        "Updated private notes"
    );

    let delete = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "notes",
            "delete",
            "--yes",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--note-id",
            &note_id.to_string(),
        ],
        None,
    );
    assert!(
        delete.status.success(),
        "note delete failed: {}",
        delete.stderr
    );
    assert_eq!(parse_stdout_json(&delete.stdout)["deleted"], true);
    assert!(state.lock().expect("state lock").notes.is_empty());

    let shared = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "notes",
            "create",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--title",
            "Shared context",
            "--idempotency-key",
            "agent:notes:shared-context",
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        shared.status.success(),
        "shared note create failed: {}",
        shared.stderr
    );
    let shared = parse_stdout_json(&shared.stdout);
    assert_eq!(shared["title"], "Shared context");
    assert_eq!(shared["isPrivate"], false);
    assert!(shared["bodyMarkdown"].is_null());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_should_upload_encrypt_attach_and_delete_task_attachment() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state.clone()).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);
    let plaintext = b"# Uploaded\n\nEncrypted attachment body\n";
    let upload_path = home.path().join("agent-notes.md");
    std::fs::write(&upload_path, plaintext).expect("write upload fixture");

    let upload = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "tasks",
            "attachments",
            "upload",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--task-id",
            &fixture.task_id.to_string(),
            "--file",
            "agent-notes.md",
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        upload.status.success(),
        "attachment upload failed: {}",
        upload.stderr
    );
    let uploaded = parse_stdout_json(&upload.stdout);
    let attachment_id = Uuid::parse_str(uploaded["id"].as_str().expect("uploaded attachment id"))
        .expect("attachment UUID");
    assert_eq!(uploaded["fileName"], "agent-notes.md");
    assert_eq!(uploaded["contentType"], "text/markdown");
    assert!(uploaded.get("blobKey").is_none());

    {
        let state = state.lock().expect("state lock");
        let value = state
            .current_task_body
            .attachments
            .as_ref()
            .expect("task attachments")
            .iter()
            .find(|value| {
                flexible_value_to_json((*value).clone())["id"] == attachment_id.to_string()
            })
            .expect("uploaded attachment payload");
        let json = flexible_value_to_json(value.clone());
        let blob_key = json["blob_key"]
            .as_array()
            .expect("blob key bytes")
            .iter()
            .map(|value| value.as_u64().expect("blob byte") as u8)
            .collect::<Vec<_>>();
        let blob_ref = decode_attachment_blob_key(&fixture.list_key, &blob_key)
            .expect("decode uploaded blob key");
        let ciphertext = state
            .attachment_uploads
            .get(&attachment_id)
            .expect("uploaded ciphertext");
        assert_ne!(ciphertext.as_slice(), plaintext);
        assert_eq!(
            decrypt_attachment_bytes(ciphertext, &blob_ref.file_key, Some(&blob_ref.enc_context),)
                .expect("decrypt uploaded ciphertext"),
            plaintext
        );
    }

    let delete = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "tasks",
            "attachments",
            "delete",
            "--yes",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--task-id",
            &fixture.task_id.to_string(),
            "--attachment-id",
            &attachment_id.to_string(),
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        delete.status.success(),
        "attachment delete failed: {}",
        delete.stderr
    );
    assert_eq!(parse_stdout_json(&delete.stdout)["deleted"], true);
    let state = state.lock().expect("state lock");
    assert!(!task_attachment_ids(&state.current_task_body).contains(&attachment_id));
    assert!(state.deleted_attachment_ids.contains(&attachment_id));
    assert!(!state.attachment_uploads.contains_key(&attachment_id));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_should_preserve_cli_task_lifecycle_fields_checklist_completion_and_revision_contract()
{
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state.clone()).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);
    let checklist_id = Uuid::now_v7();
    let due_at = "2026-08-10T09:30:00Z";
    let start_at = "2026-08-09T08:00:00Z";
    let idempotency_key = format!("agent:{}", Uuid::now_v7());

    let create_input = write_json_file(
        home.path(),
        "task-lifecycle-create.json",
        &json!({
            "title": "Lifecycle task",
            "body": "Initial body",
            "checklist": [{
                "id": checklist_id,
                "title": "First step",
                "is_done": false
            }],
            "priority": 5,
            "dueAt": due_at,
            "startAt": start_at,
            "sectionId": fixture.first_section_id,
            "idempotencyKey": idempotency_key
        }),
    );
    let create_output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "tasks",
            "create",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--input-file",
            create_input.to_str().expect("utf8 path"),
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        create_output.status.success(),
        "task lifecycle create failed: {}",
        create_output.stderr
    );
    assert!(create_output.stderr.is_empty());
    let created = parse_stdout_json(&create_output.stdout);
    assert_eq!(created["priority"], 5);
    assert_eq!(created["dueAt"], due_at);
    assert_eq!(created["startAt"], start_at);
    assert_eq!(created["sectionId"], fixture.first_section_id.to_string());

    let first_commitment = {
        let state = state.lock().expect("state lock");
        let request = state
            .created_task_request
            .as_ref()
            .expect("create request captured");
        assert_eq!(request["idempotencyKey"], idempotency_key);
        assert!(request["idempotencyCommitment"].is_string());
        assert_eq!(
            state
                .created_task_body
                .as_ref()
                .and_then(|body| body.checklist.as_ref())
                .map(Vec::len),
            Some(1)
        );
        request["idempotencyCommitment"].clone()
    };

    let retry_output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "tasks",
            "create",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--input-file",
            create_input.to_str().expect("utf8 path"),
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        retry_output.status.success(),
        "retry failed: {}",
        retry_output.stderr
    );
    let state_after_retry = state.lock().expect("state lock");
    assert_eq!(
        state_after_retry
            .created_task_request
            .as_ref()
            .expect("retry request captured")["idempotencyCommitment"],
        first_commitment,
        "logical retries must keep a stable commitment despite fresh ciphertext"
    );
    drop(state_after_retry);

    let update_input = write_json_file(
        home.path(),
        "task-lifecycle-update.json",
        &json!({
            "body": null,
            "checklist": [{
                "id": checklist_id,
                "title": "First step complete",
                "is_done": true,
                "completed_at": 1786262400
            }],
            "priority": null,
            "dueAt": null,
            "startAt": null
        }),
    );
    let update_output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "tasks",
            "update",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--task-id",
            &fixture.task_id.to_string(),
            "--input-file",
            update_input.to_str().expect("utf8 path"),
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        update_output.status.success(),
        "task lifecycle update failed: {}",
        update_output.stderr
    );
    let updated = parse_stdout_json(&update_output.stdout);
    assert!(updated["priority"].is_null());
    assert!(updated["dueAt"].is_null());
    assert!(updated["startAt"].is_null());
    {
        let state = state.lock().expect("state lock");
        let request = state
            .updated_task_request
            .as_ref()
            .expect("update request captured");
        assert!(request["expectedUpdatedAt"].is_string());
        assert!(request["priority"].is_null());
        assert!(request["dueAt"].is_null());
        assert!(request["startAt"].is_null());
        let body = state.updated_task_body.as_ref().expect("payload updated");
        assert!(body.rich_text.is_none());
        let item = &body.checklist.as_ref().expect("checklist present")[0];
        assert!(item.is_done);
        assert!(item.completed_at.is_some());
    }

    let completed_output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "tasks",
            "complete",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--task-id",
            &fixture.task_id.to_string(),
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        completed_output.status.success(),
        "task complete failed: {}",
        completed_output.stderr
    );
    let completed = parse_stdout_json(&completed_output.stdout);
    assert_eq!(completed["isCompleted"], true);
    assert_eq!(completed["sectionId"], fixture.done_section_id.to_string());
    {
        let state = state.lock().expect("state lock");
        let request = state
            .moved_task_body
            .as_ref()
            .expect("completion move request captured");
        assert_eq!(request.section_id, None);
        assert_eq!(request.section_boundary.as_deref(), Some("last"));
    }

    let reopened_output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "tasks",
            "reopen",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--task-id",
            &fixture.task_id.to_string(),
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        reopened_output.status.success(),
        "task reopen failed: {}",
        reopened_output.stderr
    );
    let reopened = parse_stdout_json(&reopened_output.stdout);
    assert_eq!(reopened["isCompleted"], false);
    assert_eq!(reopened["sectionId"], fixture.first_section_id.to_string());
    let state = state.lock().expect("state lock");
    let request = state
        .moved_task_body
        .as_ref()
        .expect("reopen move request captured");
    assert_eq!(request.section_id, None);
    assert_eq!(request.section_boundary.as_deref(), Some("first"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_should_treat_reopening_an_open_task_as_a_noop_with_one_section() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    state.lock().expect("state lock").single_section = true;
    let server = spawn_server(state.clone()).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "tasks",
            "reopen",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--task-id",
            &fixture.task_id.to_string(),
            "--password-stdin",
        ],
        Some(&fixture.password),
    );

    assert!(
        output.status.success(),
        "task reopen failed: {}",
        output.stderr
    );
    assert_eq!(parse_stdout_json(&output.stdout)["isCompleted"], false);
    let state = state.lock().expect("state lock");
    let request = state
        .moved_task_body
        .as_ref()
        .expect("atomic reopen request captured");
    assert_eq!(request.section_boundary.as_deref(), Some("first"));
    assert_eq!(state.task_section_id, None);
    assert!(!state.task_is_completed);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_should_page_all_completed_tasks_for_raw_and_decrypted_lists() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    {
        let mut state = state.lock().expect("state lock");
        state.my_tasks_count = 125;
        state.task_is_completed = true;
        state.task_completed_at = Some(Utc::now());
    }
    let server = spawn_server(state.clone()).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let raw_output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "tasks",
            "list",
            "--all",
            "--include-completed",
            "--raw",
        ],
        None,
    );
    assert!(
        raw_output.status.success(),
        "raw paginated task list failed: {}",
        raw_output.stderr
    );
    let raw_tasks = parse_stdout_json(&raw_output.stdout);
    assert_eq!(raw_tasks.as_array().expect("raw task array").len(), 125);

    let decrypted_output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "tasks",
            "list",
            "--all",
            "--include-completed",
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        decrypted_output.status.success(),
        "decrypted paginated task list failed: {}",
        decrypted_output.stderr
    );
    let decrypted_tasks = parse_stdout_json(&decrypted_output.stdout);
    assert_eq!(
        decrypted_tasks
            .as_array()
            .expect("decrypted task array")
            .len(),
        125
    );

    assert_eq!(
        state.lock().expect("state lock").my_tasks_queries,
        vec![(0, true), (100, true), (0, true), (100, true)]
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_should_emit_strict_json_documents_for_empty_task_collections_and_conflicts() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state.clone()).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);
    state.lock().expect("state lock").tasks_empty = true;
    let work_list_id = fixture.work_list_id.to_string();

    for args in [
        vec![
            "--json",
            "tasks",
            "list",
            "--work-list-id",
            work_list_id.as_str(),
            "--password-stdin",
        ],
        vec!["--json", "tasks", "list", "--all", "--password-stdin"],
        vec![
            "--json",
            "tasks",
            "list",
            "--work-list-id",
            work_list_id.as_str(),
            "--raw",
        ],
    ] {
        let output = run_cli(
            home.path(),
            &server.base_url,
            &args,
            args.contains(&"--password-stdin")
                .then_some(fixture.password.as_str()),
        );
        assert!(
            output.status.success(),
            "empty list failed: {}",
            output.stderr
        );
        assert_eq!(parse_stdout_json(&output.stdout), json!([]));
        assert!(output.stderr.is_empty());
    }

    {
        let mut state = state.lock().expect("state lock");
        state.tasks_empty = false;
        state.reject_next_task_update_as_conflict = true;
        state.updated_task_body = None;
    }
    let output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "tasks",
            "update",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--task-id",
            &fixture.task_id.to_string(),
            "--title",
            "Must not overwrite",
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let error = parse_stderr_json(&output.stderr);
    assert_eq!(error["error"]["code"], "conflict");
    assert_eq!(
        error["error"]["message"],
        "request conflicted with current server state"
    );
    let state = state.lock().expect("state lock");
    assert!(state.updated_task_body.is_none());
    assert!(state.updated_task_request.as_ref().expect("request")["expectedUpdatedAt"].is_string());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_work_list_archive_lifecycle_preserves_active_only_defaults() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state.clone()).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let active_lists_output = run_cli(
        home.path(),
        &server.base_url,
        &["--json", "lists", "--password-stdin"],
        Some(&fixture.password),
    );
    assert!(
        active_lists_output.status.success(),
        "active lists failed: {}",
        active_lists_output.stderr
    );
    let active_lists: Value = parse_stdout_json(&active_lists_output.stdout);
    assert!(active_lists[0]["archivedAt"].is_null());

    let archive_output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "lists",
            "archive",
            &fixture.work_list_id.to_string(),
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        archive_output.status.success(),
        "list archive failed: {}",
        archive_output.stderr
    );
    let archived: Value = parse_stdout_json(&archive_output.stdout);
    assert!(archived[0]["archivedAt"].is_string());

    let select_archived = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "projects",
            "use",
            &fixture.work_list_id.to_string(),
        ],
        None,
    );
    assert_eq!(select_archived.status.code(), Some(1));
    assert!(select_archived.stdout.is_empty());
    assert_json_error_contains(
        &select_archived.stderr,
        "archived project cannot be selected",
    );

    let active_lists_output = run_cli(
        home.path(),
        &server.base_url,
        &["--json", "lists", "--password-stdin"],
        Some(&fixture.password),
    );
    assert!(
        active_lists_output.status.success(),
        "active-only archived list query failed: {}",
        active_lists_output.stderr
    );
    assert_eq!(parse_stdout_json(&active_lists_output.stdout), json!([]));

    let archived_lists_output = run_cli(
        home.path(),
        &server.base_url,
        &["--json", "lists", "--include-archived", "--password-stdin"],
        Some(&fixture.password),
    );
    assert!(
        archived_lists_output.status.success(),
        "archived lists failed: {}",
        archived_lists_output.stderr
    );
    let archived_lists: Value = parse_stdout_json(&archived_lists_output.stdout);
    assert!(archived_lists[0]["archivedAt"].is_string());

    let raw_archived_lists_output = run_cli(
        home.path(),
        &server.base_url,
        &["--json", "lists", "--include-archived", "--raw"],
        None,
    );
    assert!(
        raw_archived_lists_output.status.success(),
        "raw archived lists failed: {}",
        raw_archived_lists_output.stderr
    );
    let raw_archived_lists: Value = parse_stdout_json(&raw_archived_lists_output.stdout);
    assert!(raw_archived_lists[0]["archivedAt"].is_string());
    assert!(raw_archived_lists[0]["titleCiphertext"].is_string());

    let unarchive_output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "lists",
            "unarchive",
            &fixture.work_list_id.to_string(),
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        unarchive_output.status.success(),
        "list unarchive failed: {}",
        unarchive_output.stderr
    );
    let unarchived: Value = parse_stdout_json(&unarchive_output.stdout);
    assert!(unarchived[0]["archivedAt"].is_null());

    let state = state.lock().expect("state lock");
    assert_eq!(state.archive_work_list_count, 1);
    assert_eq!(state.unarchive_work_list_count, 1);
    assert_eq!(
        state.list_work_list_include_archived,
        vec![false, false, true, true]
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_task_reads_parse_current_api_shapes() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let lists_output = run_cli(
        home.path(),
        &server.base_url,
        &["--json", "lists", "--password-stdin"],
        Some(&fixture.password),
    );
    assert!(
        lists_output.status.success(),
        "lists failed: {}",
        lists_output.stderr
    );
    let lists_json: Value = parse_stdout_json(&lists_output.stdout);
    assert_eq!(lists_json[0]["title"], "Fixture Work List");
    assert!(lists_json[0].get("payloadCiphertext").is_none());

    let list_detail_output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "lists",
            "get",
            &fixture.work_list_id.to_string(),
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        list_detail_output.status.success(),
        "lists get failed: {}",
        list_detail_output.stderr
    );
    let list_detail_json: Value = parse_stdout_json(&list_detail_output.stdout);
    assert_eq!(list_detail_json["title"], "Fixture Work List");
    assert_eq!(list_detail_json["members"][0]["role"], "owner");

    let my_tasks_output = run_cli(
        home.path(),
        &server.base_url,
        &["--json", "tasks", "list", "--all", "--password-stdin"],
        Some(&fixture.password),
    );
    assert!(
        my_tasks_output.status.success(),
        "my tasks failed: {}",
        my_tasks_output.stderr
    );
    let my_tasks_json: Value = parse_stdout_json(&my_tasks_output.stdout);
    assert_eq!(my_tasks_json[0]["id"], fixture.task_id.to_string());
    assert_eq!(my_tasks_json[0]["title"], "Existing task");
    assert_eq!(my_tasks_json[0]["bodyMarkdown"], "Existing task body");
    assert_eq!(my_tasks_json[0]["workListTitle"], "Fixture Work List");
    assert!(my_tasks_json[0].get("titleCiphertext").is_none());
    assert!(my_tasks_json[0].get("payloadCiphertext").is_none());

    let list_tasks_output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "tasks",
            "list",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        list_tasks_output.status.success(),
        "list tasks failed: {}",
        list_tasks_output.stderr
    );
    let list_tasks_json: Value = parse_stdout_json(&list_tasks_output.stdout);
    assert_eq!(list_tasks_json[0]["title"], "Existing task");
    assert_eq!(list_tasks_json[0]["bodyMarkdown"], "Existing task body");

    let task_detail_output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "tasks",
            "get",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--task-id",
            &fixture.task_id.to_string(),
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        task_detail_output.status.success(),
        "task get failed: {}",
        task_detail_output.stderr
    );
    let task_detail_json: Value = parse_stdout_json(&task_detail_output.stdout);
    assert_eq!(task_detail_json["title"], "Existing task");
    assert_eq!(
        task_detail_json["comments"][0]["bodyMarkdown"],
        "Existing comment"
    );
    assert_eq!(task_detail_json["clientMeta"]["source"], "fixture");
    assert_eq!(task_detail_json["clientMeta"]["blob"], json!([1, 2, 3, 4]));
    assert_eq!(
        task_detail_json["attachments"][0]["id"],
        fixture.text_attachment.id.to_string()
    );
    assert_eq!(task_detail_json["attachments"][0]["fileName"], "notes.md");
    assert_eq!(
        task_detail_json["attachments"][0]["contentType"],
        "text/markdown"
    );
    assert!(task_detail_json["attachments"][0].get("blobKey").is_none());
    assert_eq!(
        task_detail_json["comments"][0]["clientMeta"]["blob"],
        json!([9, 8, 7])
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_persists_rotated_refresh_tokens_after_automatic_refresh() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state.clone()).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials_with_expiry(
        home.path(),
        &fixture,
        &server.base_url,
        Utc::now() - Duration::minutes(5),
        Utc::now() + Duration::days(1),
    );

    let first_output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "tasks",
            "get",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--task-id",
            &fixture.task_id.to_string(),
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        first_output.status.success(),
        "first task get failed: {}",
        first_output.stderr
    );

    let credentials_path = home.path().join(".sealtask").join("credentials.json");
    let mut saved_credentials: Credentials = serde_json::from_slice(
        &std::fs::read(&credentials_path).expect("read credentials after first task get"),
    )
    .expect("parse credentials after first task get");
    saved_credentials.access_expires_at = Utc::now() - Duration::minutes(5);
    std::fs::write(
        &credentials_path,
        serde_json::to_vec_pretty(&saved_credentials).expect("serialize expired credentials"),
    )
    .expect("rewrite credentials");

    let second_output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "tasks",
            "get",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--task-id",
            &fixture.task_id.to_string(),
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        second_output.status.success(),
        "second task get failed: {}",
        second_output.stderr
    );

    let saved_credentials: Credentials =
        serde_json::from_slice(&std::fs::read(&credentials_path).expect("read credentials"))
            .expect("parse credentials");

    let state = state.lock().expect("state lock");
    assert_eq!(state.refresh_request_count, 2);
    assert_eq!(saved_credentials.refresh_token, state.current_refresh_token);
    assert_eq!(saved_credentials.access_token, state.current_access_token);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_inspect_returns_decrypted_work_list_detail() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let inspect_output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "inspect",
            &fixture.work_list_id.to_string(),
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        inspect_output.status.success(),
        "inspect failed: {}",
        inspect_output.stderr
    );
    let inspect_json: Value = parse_stdout_json(&inspect_output.stdout);
    assert_eq!(inspect_json["title"], "Fixture Work List");
    assert_eq!(
        inspect_json["payload"]["body"]["title"],
        "Fixture Work List"
    );
    assert!(inspect_json.get("payloadCiphertext").is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_reads_surface_partial_decryption_errors() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    {
        let mut guard = state.lock().expect("state lock");
        guard.invalid_work_list_payload = true;
        guard.invalid_task_payload = true;
    }
    let server = spawn_server(state).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let lists_output = run_cli(
        home.path(),
        &server.base_url,
        &["--json", "lists", "--password-stdin"],
        Some(&fixture.password),
    );
    assert!(
        lists_output.status.success(),
        "lists failed: {}",
        lists_output.stderr
    );
    let lists_json: Value = parse_stdout_json(&lists_output.stdout);
    assert_eq!(lists_json[0]["title"], "Fixture Work List");
    assert_eq!(lists_json[0]["readError"]["code"], "work_list_payload");

    let tasks_output = run_cli(
        home.path(),
        &server.base_url,
        &["--json", "tasks", "list", "--all", "--password-stdin"],
        Some(&fixture.password),
    );
    assert!(
        tasks_output.status.success(),
        "tasks failed: {}",
        tasks_output.stderr
    );
    let tasks_json: Value = parse_stdout_json(&tasks_output.stdout);
    assert_eq!(tasks_json[0]["title"], "Existing task");
    assert_eq!(tasks_json[0]["readError"]["code"], "task_payload");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_tasks_do_not_inherit_work_list_payload_errors_when_task_decrypts() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    {
        let mut guard = state.lock().expect("state lock");
        guard.invalid_work_list_payload = true;
    }
    let server = spawn_server(state).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let tasks_output = run_cli(
        home.path(),
        &server.base_url,
        &["--json", "tasks", "list", "--all", "--password-stdin"],
        Some(&fixture.password),
    );
    assert!(
        tasks_output.status.success(),
        "tasks failed: {}",
        tasks_output.stderr
    );
    let tasks_json: Value = parse_stdout_json(&tasks_output.stdout);
    assert_eq!(tasks_json[0]["title"], "Existing task");
    assert!(tasks_json[0].get("readError").is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_reads_surface_attachment_projection_errors() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    {
        let mut guard = state.lock().expect("state lock");
        guard.invalid_task_attachment_metadata = true;
        guard.invalid_comment_attachment_metadata = true;
    }
    let server = spawn_server(state).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "tasks",
            "get",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--task-id",
            &fixture.task_id.to_string(),
            "--password-stdin",
        ],
        Some(&fixture.password),
    );

    assert!(
        output.status.success(),
        "task get failed: {}",
        output.stderr
    );
    let task_json: Value = parse_stdout_json(&output.stdout);
    assert_eq!(task_json["title"], "Existing task");
    assert_eq!(task_json["bodyMarkdown"], "Existing task body");
    assert_eq!(task_json["readError"]["code"], "task_attachments");
    assert!(task_json["attachments"].is_null());
    assert_eq!(task_json["comments"][0]["bodyMarkdown"], "Existing comment");
    assert_eq!(
        task_json["comments"][0]["readError"]["code"],
        "comment_attachments"
    );
    assert!(task_json["comments"][0]["attachments"].is_null());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_tasks_help_uses_explicit_verbs() {
    let home = TempDir::new().expect("temp home");
    let output = run_cli(
        home.path(),
        "https://sealtask.com",
        &["tasks", "--help"],
        None,
    );

    assert!(
        output.status.success(),
        "tasks help failed: {}",
        output.stderr
    );
    assert!(output.stdout.contains("list"));
    assert!(output.stdout.contains("get"));
    assert!(output.stdout.contains("create"));
    assert!(output.stdout.contains("update"));

    let attachment_help = run_cli(
        home.path(),
        "https://sealtask.com",
        &["tasks", "attachments", "--help"],
        None,
    );
    assert!(attachment_help.status.success());
    for command in ["upload", "delete", "read", "download"] {
        assert!(attachment_help.stdout.contains(command));
    }

    let notes_help = run_cli(
        home.path(),
        "https://sealtask.com",
        &["notes", "--help"],
        None,
    );
    assert!(notes_help.status.success());
    for command in ["list", "get", "create", "update", "delete"] {
        assert!(notes_help.stdout.contains(command));
    }

    let create_help = run_cli(
        home.path(),
        "https://sealtask.com",
        &["tasks", "create", "--help"],
        None,
    );
    assert!(create_help.status.success());
    for description in [
        "Create an encrypted task",
        "Task priority: low/p4/1, medium/p3/3, high/p2/5, or urgent/p1/8",
        "Read the complete camelCase task input object",
        "Never prompt",
    ] {
        assert!(
            create_help.stdout.contains(description),
            "missing help description {description:?}: {}",
            create_help.stdout
        );
    }
}

#[test]
fn cli_schema_and_output_formats_are_machine_discoverable() {
    let home = TempDir::new().expect("temp home");
    let compact = run_cli(
        home.path(),
        "https://sealtask.com",
        &["--json", "schema", "tasks", "create"],
        None,
    );
    assert!(
        compact.status.success(),
        "schema failed: {}",
        compact.stderr
    );
    assert_eq!(compact.stdout.lines().count(), 1);
    let schema = parse_stdout_json(&compact.stdout);
    assert_eq!(schema["schemaVersion"], 1);
    assert_eq!(schema["name"], "create");
    assert!(
        schema["arguments"]
            .as_array()
            .expect("arguments")
            .iter()
            .any(|argument| argument["long"] == "idempotency-key")
    );

    let task_list = run_cli(
        home.path(),
        "https://sealtask.com",
        &["--json", "schema", "tasks", "list"],
        None,
    );
    assert!(
        task_list.status.success(),
        "task-list schema failed: {}",
        task_list.stderr
    );
    let task_list_schema = parse_stdout_json(&task_list.stdout);
    let field = task_list_schema["arguments"]
        .as_array()
        .expect("task-list arguments")
        .iter()
        .find(|argument| argument["long"] == "field")
        .expect("field schema");
    assert_eq!(field["possibleValues"], json!(["id", "title", "url"]));

    let pretty = run_cli(
        home.path(),
        "https://sealtask.com",
        &["--format", "json-pretty", "info"],
        None,
    );
    assert!(
        pretty.status.success(),
        "pretty info failed: {}",
        pretty.stderr
    );
    assert!(pretty.stdout.lines().count() > 1);
    let info = parse_stdout_json(&pretty.stdout);
    assert_eq!(info["jsonContractVersion"], 2);
    assert_eq!(
        info["taskListing"]["rawFields"],
        json!(["id", "title", "url"])
    );
    assert_eq!(info["canonicalFlags"]["projectListDetails"], "--details");

    let human = run_cli(home.path(), "https://sealtask.com", &["info"], None);
    assert!(
        human.status.success(),
        "human info failed: {}",
        human.stderr
    );
    assert!(human.stdout.starts_with("SealTask CLI contract version 2"));
}

#[test]
fn cli_terminal_policies_are_explicit_machine_safe_and_quiet_when_requested() {
    let home = TempDir::new().expect("temp home");

    let automatic = run_cli_exact(home.path(), &["info"], None);
    assert!(
        automatic.status.success(),
        "info failed: {}",
        automatic.stderr
    );
    assert!(!automatic.stdout.contains('\u{1b}'));
    assert!(automatic.stderr.is_empty());

    let colored = run_cli_exact(home.path(), &["--color", "always", "info"], None);
    assert!(
        colored.status.success(),
        "colored info failed: {}",
        colored.stderr
    );
    assert!(colored.stdout.contains("\u{1b}[1m"));

    let json = run_cli_exact(home.path(), &["--color", "always", "--json", "info"], None);
    assert!(json.status.success(), "JSON info failed: {}", json.stderr);
    assert!(!json.stdout.contains('\u{1b}'));
    assert!(json.stderr.is_empty());
    assert_eq!(
        parse_stdout_json(&json.stdout)["terminalPolicies"]["quietFlag"],
        "--quiet"
    );
    assert_eq!(
        parse_stdout_json(&json.stdout)["picker"]["selectorFormat"],
        "id:<32-lowercase-hex>"
    );

    let quiet = run_cli_exact(home.path(), &["--quiet", "auth", "logout"], None);
    assert!(
        quiet.status.success(),
        "quiet logout failed: {}",
        quiet.stderr
    );
    assert!(quiet.stdout.is_empty());
    assert!(quiet.stderr.is_empty());

    let no_pager = run_cli_exact(home.path(), &["--no-pager", "info"], None);
    assert!(
        no_pager.status.success(),
        "--no-pager failed: {}",
        no_pager.stderr
    );
    assert!(
        no_pager
            .stdout
            .starts_with("SealTask CLI contract version 2")
    );

    let mut no_pager_override = Command::cargo_bin("sealtask").expect("binary");
    no_pager_override
        .env("HOME", home.path())
        .env("SEALTASK_PAGER_MODE", "always")
        .current_dir(home.path())
        .args(["--no-pager", "info"]);
    let no_pager_override = no_pager_override.output().expect("run --no-pager override");
    assert!(
        no_pager_override.status.success(),
        "--no-pager should override SEALTASK_PAGER_MODE: {}",
        String::from_utf8_lossy(&no_pager_override.stderr)
    );

    let mut json_environment = Command::cargo_bin("sealtask").expect("binary");
    json_environment
        .env("HOME", home.path())
        .env("SEALTASK_PAGER_MODE", "always")
        .env("SEALTASK_PROGRESS", "always")
        .current_dir(home.path())
        .args(["--json", "info"]);
    let json_environment = json_environment
        .output()
        .expect("run JSON environment policy");
    assert!(
        json_environment.status.success(),
        "JSON should override terminal-mode environment: {}",
        String::from_utf8_lossy(&json_environment.stderr)
    );
    assert!(!json_environment.stdout.contains(&b'\x1b'));

    for (args, expected) in [
        (
            &["--pager", "always", "info"][..],
            "requires stdout to be a terminal",
        ),
        (
            &["--progress", "always", "info"][..],
            "requires stderr to be a terminal",
        ),
    ] {
        let output = run_cli_exact(home.path(), args, None);
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert!(output.stdout.is_empty(), "{args:?}: {}", output.stdout);
        assert!(
            output.stderr.contains(expected),
            "{args:?}: {}",
            output.stderr
        );
        assert!(!output.stderr.contains('\u{1b}'));
    }

    for args in [
        &["--json", "--pager", "always", "info"][..],
        &["--json", "--progress", "always", "info"][..],
    ] {
        let output = run_cli_exact(home.path(), args, None);
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert!(output.stdout.is_empty(), "{args:?}: {}", output.stdout);
        assert_eq!(output.stderr.lines().count(), 1, "{args:?}");
        assert_eq!(
            parse_stderr_json(&output.stderr)["error"]["code"],
            "validation"
        );
        assert!(!output.stderr.contains('\u{1b}'));
    }
}

#[test]
fn cli_fuzzy_picker_is_raw_private_and_automation_safe() {
    let home = TempDir::new().expect("temp home");

    let help = run_cli_exact(home.path(), &["pick", "task", "--help"], None);
    assert!(help.status.success(), "picker help failed: {}", help.stderr);
    for expected in [
        "Pick a task in the selected/current project",
        "--include-completed",
        "--include-archived",
        "sealtask tasks get \"$(sealtask pick task)\"",
    ] {
        assert!(
            help.stdout.contains(expected),
            "missing {expected:?} in:\n{}",
            help.stdout
        );
    }
    assert!(!home.path().join(".sealtask").exists());

    let non_interactive =
        run_cli_exact(home.path(), &["--non-interactive", "pick", "project"], None);
    assert_eq!(non_interactive.status.code(), Some(1));
    assert!(non_interactive.stdout.is_empty());
    assert!(
        non_interactive
            .stderr
            .contains("requires an interactive controlling terminal")
    );
    assert!(!home.path().join(".sealtask").exists());

    for args in [
        &["--json", "pick", "project"][..],
        &["--format", "json", "pick", "project"][..],
        &["--format", "json-pretty", "pick", "task"][..],
    ] {
        let output = run_cli_exact(home.path(), args, None);
        assert_eq!(output.status.code(), Some(1), "{args:?}: {}", output.stderr);
        assert!(output.stdout.is_empty(), "{args:?}: {}", output.stdout);
        let error = parse_stderr_json(&output.stderr);
        assert_eq!(error["error"]["code"], "validation");
        assert!(
            error["error"]["message"]
                .as_str()
                .expect("picker error message")
                .contains("one raw reusable selector")
        );
    }
    assert!(!home.path().join(".sealtask").exists());

    let schema = run_cli_exact(home.path(), &["--json", "schema", "pick", "task"], None);
    assert!(
        schema.status.success(),
        "picker schema failed: {}",
        schema.stderr
    );
    let schema = parse_stdout_json(&schema.stdout);
    assert_eq!(schema["name"], "task");
    assert!(
        schema["arguments"]
            .as_array()
            .expect("picker arguments")
            .iter()
            .any(|argument| argument["long"] == "project")
    );
    assert!(!schema.to_string().contains("Fixture Work List"));

    let argv_query = run_cli_exact(home.path(), &["pick", "project", "secret query"], None);
    assert_eq!(argv_query.status.code(), Some(2));
    assert!(argv_query.stdout.is_empty());
    assert!(argv_query.stderr.contains("unexpected argument"));

    let forced_pager = run_cli_exact(home.path(), &["--pager", "always", "pick", "project"], None);
    assert_eq!(forced_pager.status.code(), Some(1));
    assert!(forced_pager.stdout.is_empty());
    assert!(
        forced_pager
            .stderr
            .contains("paging is unavailable for this raw-output command")
    );

    let completion = run_cli_exact(home.path(), &["completion", "bash"], None);
    assert!(completion.status.success());
    assert!(completion.stdout.contains("pick"));
    assert!(!completion.stdout.to_ascii_lowercase().contains("fzf"));
}

#[test]
fn cli_editor_and_body_file_inputs_are_discoverable_and_automation_safe() {
    let home = TempDir::new().expect("temp home");

    for (args, expected) in [
        (
            &["tasks", "create", "--help"][..],
            ["--edit", "--body-file", "sealtask tasks create --edit"],
        ),
        (
            &["tasks", "edit", "--help"][..],
            ["configured editor", "TASK", "sealtask tasks edit"],
        ),
        (
            &["notes", "edit", "--help"][..],
            ["configured editor", "NOTE", "sealtask notes edit"],
        ),
        (
            &["comments", "create", "--help"][..],
            ["--body-file", "use '-' for stdin", "--input-file"],
        ),
    ] {
        let help = run_cli_exact(home.path(), args, None);
        assert!(help.status.success(), "{args:?}: {}", help.stderr);
        for marker in expected {
            assert!(
                help.stdout.contains(marker),
                "{args:?} missing {marker:?}:\n{}",
                help.stdout
            );
        }
    }
    assert!(!home.path().join(".sealtask").exists());

    let malformed_editor = run_cli_with_operator_env(
        home.path(),
        "https://sealtask.invalid",
        &["tasks", "create", "--edit"],
        &[
            ("SEALTASK_EDITOR", "vim '"),
            ("SEALTASK_CONNECT_TIMEOUT", "not-a-duration"),
        ],
        None,
    );
    assert_eq!(malformed_editor.status.code(), Some(1));
    assert!(malformed_editor.stdout.is_empty());
    assert!(
        malformed_editor
            .stderr
            .contains("SEALTASK_EDITOR contains unmatched quoting")
    );
    assert!(!home.path().join(".sealtask").exists());

    for args in [
        &["--non-interactive", "tasks", "create", "--edit"][..],
        &["--non-interactive", "tasks", "edit", "Release checklist"][..],
        &["--non-interactive", "notes", "edit", "Incident runbook"][..],
    ] {
        let output = run_cli_exact(home.path(), args, None);
        assert_eq!(output.status.code(), Some(1), "{args:?}: {}", output.stderr);
        assert!(output.stdout.is_empty(), "{args:?}: {}", output.stdout);
        assert!(
            output.stderr.contains("interactive controlling terminal"),
            "{args:?}: {}",
            output.stderr
        );
    }
    assert!(!home.path().join(".sealtask").exists());

    for args in [
        &[
            "comments",
            "create",
            "Release checklist",
            "--body-file",
            "-",
            "--password-stdin",
        ][..],
        &[
            "tasks",
            "update",
            "Release checklist",
            "--body-file",
            "-",
            "--password-stdin",
        ][..],
    ] {
        let output = run_cli_exact(home.path(), args, Some("password"));
        assert_eq!(output.status.code(), Some(1), "{args:?}: {}", output.stderr);
        assert!(output.stdout.is_empty(), "{args:?}: {}", output.stdout);
        assert!(output.stderr.contains("both consume stdin"), "{args:?}");
    }

    let info = run_cli_exact(home.path(), &["--json", "info"], None);
    assert!(info.status.success(), "info failed: {}", info.stderr);
    let info = parse_stdout_json(&info.stdout);
    assert_eq!(
        info["editorInput"]["temporaryPermissions"]["posixFileMode"],
        "0600"
    );
    assert_eq!(
        info["editorInput"]["temporaryPermissions"]["posixModesEnforced"],
        cfg!(unix)
    );
    assert_eq!(info["editorInput"]["directProcess"], true);
    assert_eq!(info["bodyFileInput"]["stdinPath"], "-");
}

#[test]
fn cli_profiles_isolate_credentials_and_support_custom_config_roots() {
    let home = TempDir::new().expect("temp home");
    let fixture = TestFixture::new();
    seed_credentials(home.path(), &fixture, "https://sealtask.com");
    let isolated = run_cli(
        home.path(),
        "https://sealtask.com",
        &["--json", "--profile", "isolated-agent", "auth", "status"],
        None,
    );
    assert!(
        isolated.status.success(),
        "isolated status failed: {}",
        isolated.stderr
    );
    assert_eq!(parse_stdout_json(&isolated.stdout)["loggedIn"], false);

    let config_root = home.path().join("agent-state");
    let output = run_cli(
        home.path(),
        "https://sealtask.com",
        &[
            "--json",
            "--config-dir",
            config_root.to_str().expect("UTF-8 config path"),
            "--profile",
            "build-agent",
            "auth",
            "status",
        ],
        None,
    );
    assert!(
        output.status.success(),
        "profile status failed: {}",
        output.stderr
    );
    let status = parse_stdout_json(&output.stdout);
    assert_eq!(status["loggedIn"], false);
    assert_eq!(status["profile"], "build-agent");
    assert_eq!(
        status["configDirectory"],
        config_root
            .join("profiles/build-agent")
            .display()
            .to_string()
    );
    assert_eq!(
        status["credentialsPath"],
        config_root
            .join("profiles/build-agent/credentials.json")
            .display()
            .to_string()
    );
}

#[test]
fn cli_operator_config_reports_resolved_timeout_values_and_sources() {
    let home = TempDir::new().expect("temp home");
    let output = run_cli_with_operator_env(
        home.path(),
        "https://operator.example.test",
        &[
            "--json",
            "--connect-timeout",
            "250ms",
            "--read-timeout",
            "2s",
            "--request-timeout",
            "1m30s",
            "config",
            "show",
            "--resolved",
        ],
        &[],
        None,
    );

    assert!(
        output.status.success(),
        "resolved config failed: {}",
        output.stderr
    );
    assert!(output.stderr.is_empty());
    let config = parse_stdout_json(&output.stdout);
    assert_eq!(config["schemaVersion"], 1);
    assert_eq!(config["resolved"], true);
    assert_eq!(config["apiUrl"]["value"], "https://operator.example.test");
    assert_eq!(config["apiUrl"]["source"], "cli");
    assert_eq!(config["profile"]["value"], "default");
    assert_eq!(config["profile"]["source"], "default");
    assert_eq!(config["configDirectory"]["source"], "default");
    assert_eq!(config["connectTimeout"]["milliseconds"], 250);
    assert_eq!(config["connectTimeout"]["display"], "250ms");
    assert_eq!(config["connectTimeout"]["source"], "cli");
    assert_eq!(config["readTimeout"]["milliseconds"], 2_000);
    assert_eq!(config["readTimeout"]["display"], "2s");
    assert_eq!(config["readTimeout"]["source"], "cli");
    assert_eq!(config["requestTimeout"]["milliseconds"], 90_000);
    assert_eq!(config["requestTimeout"]["display"], "90s");
    assert_eq!(config["requestTimeout"]["source"], "cli");

    let environment = run_cli_with_operator_env(
        home.path(),
        "https://operator.example.test",
        &["--json", "config", "show", "--resolved"],
        &[("SEALTASK_READ_TIMEOUT", "3s")],
        None,
    );
    assert!(
        environment.status.success(),
        "environment config failed: {}",
        environment.stderr
    );
    let environment = parse_stdout_json(&environment.stdout);
    assert_eq!(environment["readTimeout"]["milliseconds"], 3_000);
    assert_eq!(environment["readTimeout"]["source"], "environment");

    let shadowed_invalid_environment = run_cli_with_operator_env(
        home.path(),
        "https://operator.example.test",
        &[
            "--json",
            "--read-timeout",
            "4s",
            "config",
            "show",
            "--resolved",
        ],
        &[("SEALTASK_READ_TIMEOUT", "not-a-duration")],
        None,
    );
    assert!(
        shadowed_invalid_environment.status.success(),
        "CLI override did not shadow invalid lower-precedence environment: {}",
        shadowed_invalid_environment.stderr
    );
    let shadowed_invalid_environment = parse_stdout_json(&shadowed_invalid_environment.stdout);
    assert_eq!(
        shadowed_invalid_environment["readTimeout"]["milliseconds"],
        4_000
    );
    assert_eq!(shadowed_invalid_environment["readTimeout"]["source"], "cli");
    assert!(
        !home.path().join(".sealtask").exists(),
        "read-only config inspection must not create local state"
    );
}

#[test]
fn cli_profile_use_persists_defaults_and_explains_environment_overrides() {
    let home = TempDir::new().expect("temp home");
    let select_team = run_cli_with_operator_env(
        home.path(),
        "https://operator.example.test",
        &["--json", "profile", "use", "team"],
        &[],
        None,
    );
    assert!(
        select_team.status.success(),
        "profile use failed: {}",
        select_team.stderr
    );
    let selected = parse_stdout_json(&select_team.stdout);
    assert_eq!(selected["schemaVersion"], 1);
    assert_eq!(selected["defaultProfile"], "team");
    assert_eq!(selected["changed"], true);
    assert_eq!(selected["effectiveProfileForThisCommand"], "default");
    assert_eq!(selected["effectiveSourceForThisCommand"], "default");
    assert_eq!(selected["overrideActive"], false);

    let persisted_list = run_cli_with_operator_env(
        home.path(),
        "https://operator.example.test",
        &["--json", "profile", "list"],
        &[],
        None,
    );
    assert!(
        persisted_list.status.success(),
        "profile list failed: {}",
        persisted_list.stderr
    );
    let persisted_list = parse_stdout_json(&persisted_list.stdout);
    assert_eq!(persisted_list["activeProfile"], "team");
    assert_eq!(persisted_list["activeSource"], "persisted");
    assert_eq!(persisted_list["defaultProfile"], "team");
    assert!(
        persisted_list["profiles"]
            .as_array()
            .expect("profile entries")
            .iter()
            .any(|profile| profile["name"] == "team" && profile["active"] == true)
    );

    let overridden = run_cli_with_operator_env(
        home.path(),
        "https://operator.example.test",
        &["--json", "profile", "use", "next-default"],
        &[("SEALTASK_PROFILE", "deployment")],
        None,
    );
    assert!(
        overridden.status.success(),
        "profile use with override failed: {}",
        overridden.stderr
    );
    let overridden = parse_stdout_json(&overridden.stdout);
    assert_eq!(overridden["defaultProfile"], "next-default");
    assert_eq!(overridden["effectiveProfileForThisCommand"], "deployment");
    assert_eq!(overridden["effectiveSourceForThisCommand"], "environment");
    assert_eq!(overridden["overrideActive"], true);

    let overridden_list = run_cli_with_operator_env(
        home.path(),
        "https://operator.example.test",
        &["--json", "profile", "list"],
        &[("SEALTASK_PROFILE", "deployment")],
        None,
    );
    assert!(
        overridden_list.status.success(),
        "overridden profile list failed: {}",
        overridden_list.stderr
    );
    let overridden_list = parse_stdout_json(&overridden_list.stdout);
    assert_eq!(overridden_list["activeProfile"], "deployment");
    assert_eq!(overridden_list["activeSource"], "environment");
    assert_eq!(overridden_list["defaultProfile"], "next-default");

    let future_default = run_cli_with_operator_env(
        home.path(),
        "https://operator.example.test",
        &["--json", "profile", "list"],
        &[],
        None,
    );
    assert!(
        future_default.status.success(),
        "persisted replacement profile list failed: {}",
        future_default.stderr
    );
    let future_default = parse_stdout_json(&future_default.stdout);
    assert_eq!(future_default["activeProfile"], "next-default");
    assert_eq!(future_default["activeSource"], "persisted");
    assert_eq!(future_default["defaultProfile"], "next-default");
}

#[test]
fn cli_profile_lock_contention_is_bounded_and_retryable() {
    let home = TempDir::new().expect("temp home");
    let config_dir = home.path().join(".sealtask");
    std::fs::create_dir_all(&config_dir).expect("create config directory");
    let lock_path = config_dir.join("operator-settings.lock");
    let lock_file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&lock_path)
        .expect("open operator settings lock");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        std::fs::set_permissions(&lock_path, std::fs::Permissions::from_mode(0o600))
            .expect("secure lock fixture");
    }
    fs2::FileExt::lock_exclusive(&lock_file).expect("hold operator settings lock");

    let started_at = std::time::Instant::now();
    let output = run_cli_with_operator_env(
        home.path(),
        "https://operator.example.test",
        &["--json", "profile", "use", "blocked"],
        &[],
        None,
    );
    let elapsed = started_at.elapsed();
    fs2::FileExt::unlock(&lock_file).expect("release operator settings lock");

    assert_eq!(output.status.code(), Some(1));
    assert!(
        elapsed < std::time::Duration::from_secs(4),
        "operator settings lock wait was not bounded: {elapsed:?}"
    );
    let error = parse_stderr_json(&output.stderr);
    assert_eq!(error["error"]["code"], "request_timeout");
    assert_eq!(error["error"]["retryable"], true);
    assert!(
        error["error"]["hint"]
            .as_str()
            .expect("retry hint")
            .contains("Retry")
    );
    assert!(
        error["error"]["message"]
            .as_str()
            .expect("lock error message")
            .contains("other SealTask command")
    );
}

#[test]
fn cli_doctor_offline_json_versions_and_skips_api_checks() {
    let home = TempDir::new().expect("temp home");
    let output = run_cli_with_operator_env(
        home.path(),
        "https://unreachable.invalid",
        &["--json", "doctor", "--offline"],
        &[],
        None,
    );

    assert!(
        output.status.success(),
        "offline doctor failed: {}",
        output.stderr
    );
    assert!(output.stderr.is_empty());
    let report = parse_stdout_json(&output.stdout);
    assert_eq!(report["schemaVersion"], 1);
    assert_eq!(report["options"]["offline"], true);
    assert_eq!(report["options"]["strict"], false);
    assert_eq!(
        report["environment"]["apiTarget"],
        "https://unreachable.invalid"
    );
    for check_id in ["api.reachability", "api.health", "api.identity"] {
        let check = report["checks"]
            .as_array()
            .expect("doctor checks")
            .iter()
            .find(|check| check["id"] == check_id)
            .unwrap_or_else(|| panic!("missing doctor check {check_id}"));
        assert_eq!(check["status"], "skipped");
    }
    assert!(
        report["summary"]["skipped"]
            .as_u64()
            .expect("skipped count")
            >= 3
    );

    let strict = run_cli_with_operator_env(
        home.path(),
        "https://unreachable.invalid",
        &["--json", "doctor", "--offline", "--strict"],
        &[],
        None,
    );
    assert_eq!(strict.status.code(), Some(1));
    let strict_report = parse_stdout_json(&strict.stdout);
    assert_eq!(strict_report["schemaVersion"], 1);
    assert_eq!(strict_report["options"]["strict"], true);
    assert_eq!(strict_report["outcome"], "failed");
    let strict_error = parse_stderr_json(&strict.stderr);
    assert_eq!(strict_error["error"]["code"], "validation");
    assert!(
        strict_error["error"]["message"]
            .as_str()
            .expect("strict doctor error")
            .contains("strict diagnostics")
    );

    let settings_path = home.path().join(".sealtask/operator-settings.json");
    std::fs::create_dir_all(settings_path.parent().expect("settings parent"))
        .expect("create settings directory");
    std::fs::write(&settings_path, b"{not valid JSON").expect("write corrupt settings");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        std::fs::set_permissions(&settings_path, std::fs::Permissions::from_mode(0o600))
            .expect("secure corrupt settings");
    }

    let corrupt_settings = run_cli_with_operator_env(
        home.path(),
        "https://unreachable.invalid",
        &["--json", "doctor", "--offline"],
        &[],
        None,
    );
    assert_eq!(corrupt_settings.status.code(), Some(1));
    let corrupt_report = parse_stdout_json(&corrupt_settings.stdout);
    let settings_check = corrupt_report["checks"]
        .as_array()
        .expect("doctor checks")
        .iter()
        .find(|check| check["id"] == "configuration.operator_settings")
        .expect("operator settings check");
    assert_eq!(settings_check["status"], "fail");
    assert_eq!(settings_check["errorCode"], "operator_settings_invalid");
    assert!(
        settings_check["remediation"]
            .as_str()
            .expect("settings remediation")
            .contains("move operator-settings.json aside")
    );
    assert_eq!(
        std::fs::read(&settings_path).expect("corrupt settings preserved"),
        b"{not valid JSON"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_doctor_online_health_is_correlated_and_identifiable_while_logged_out() {
    let server = spawn_doctor_health_server(None).await;
    let home = TempDir::new().expect("temp home");
    let output =
        run_cli_with_operator_env(home.path(), &server.base_url, &["-v", "doctor"], &[], None);

    assert!(
        output.status.success(),
        "online doctor failed: {}",
        output.stderr
    );
    assert!(output.stdout.contains("api.reachability"));
    assert!(output.stdout.contains("api.health"));
    assert!(output.stdout.contains("api.identity"));
    assert!(output.stdout.contains("PASS"));
    assert!(output.stdout.contains("SKIP"));
    assert!(!output.stdout.contains("[sealtask]"));

    let telemetry_invocation_id = telemetry_invocation_id(&output.stderr);
    let observations = server.observations.lock().expect("observations lock");
    assert_eq!(
        observations.len(),
        1,
        "doctor should make exactly one logged-out health request"
    );
    let request = &observations[0];
    assert_eq!(
        request.user_agent.as_deref(),
        Some(CONTROL_PLANE_USER_AGENT)
    );
    let request_id = request
        .request_id
        .as_deref()
        .expect("health x-request-id")
        .parse::<Uuid>()
        .expect("health x-request-id UUID");
    assert_eq!(request_id, telemetry_invocation_id);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_doctor_reports_transport_timeout_for_stalled_health_probe() {
    let server = spawn_doctor_health_server(Some(std::time::Duration::from_secs(5))).await;
    let home = TempDir::new().expect("temp home");
    let started_at = std::time::Instant::now();
    let output = run_cli_with_operator_env(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "--connect-timeout",
            "20ms",
            "--read-timeout",
            "40ms",
            "--request-timeout",
            "75ms",
            "doctor",
        ],
        &[],
        None,
    );
    let elapsed = started_at.elapsed();

    assert_eq!(output.status.code(), Some(1));
    assert!(
        elapsed < std::time::Duration::from_secs(2),
        "75ms request deadline took {elapsed:?}"
    );
    let report = parse_stdout_json(&output.stdout);
    let reachability = report["checks"]
        .as_array()
        .expect("doctor checks")
        .iter()
        .find(|check| check["id"] == "api.reachability")
        .expect("API reachability check");
    assert_eq!(reachability["status"], "fail");
    assert_eq!(reachability["errorCode"], "transport_timeout");
    let health = report["checks"]
        .as_array()
        .expect("doctor checks")
        .iter()
        .find(|check| check["id"] == "api.health")
        .expect("API health check");
    assert_eq!(health["status"], "skipped");
    assert_eq!(report["outcome"], "failed");
    assert_eq!(
        server.observations.lock().expect("observations lock").len(),
        1
    );
    let error = parse_stderr_json(&output.stderr);
    assert_eq!(error["error"]["code"], "validation");
}

#[test]
fn cli_operator_telemetry_is_correlatable_stderr_only_and_json_incompatible() {
    let home = TempDir::new().expect("temp home");
    for flag in ["-v", "-vv", "--debug"] {
        let output = run_cli_with_operator_env(
            home.path(),
            "https://operator.example.test",
            &[flag, "info"],
            &[],
            None,
        );
        assert!(
            output.status.success(),
            "{flag} info failed: {}",
            output.stderr
        );
        assert!(output.stdout.starts_with("SealTask CLI contract version"));
        assert!(!output.stdout.contains("[sealtask]"));
        assert!(output.stderr.contains("[sealtask] event=start"));
        assert!(output.stderr.contains("[sealtask] event=finish"));
        if flag == "-v" {
            assert!(!output.stderr.contains("event=config"));
        } else {
            assert!(output.stderr.contains("event=config"));
        }

        telemetry_invocation_id(&output.stderr);
    }

    let profile_canary = "release-c-sensitive-profile";
    let config_path_canary = home.path().join("release-c-sensitive-config-path");
    let redacted_configuration = run_cli_with_operator_env(
        home.path(),
        "https://operator.example.test/private-api-path-canary",
        &[
            "-vv",
            "--profile",
            profile_canary,
            "--config-dir",
            config_path_canary.to_str().expect("UTF-8 config path"),
            "info",
        ],
        &[],
        None,
    );
    assert!(
        redacted_configuration.status.success(),
        "redacted config telemetry failed: {}",
        redacted_configuration.stderr
    );
    assert!(!redacted_configuration.stderr.contains(profile_canary));
    assert!(
        !redacted_configuration
            .stderr
            .contains("release-c-sensitive-config-path")
    );
    assert!(
        !redacted_configuration
            .stderr
            .contains("private-api-path-canary")
    );
    assert!(
        redacted_configuration
            .stderr
            .contains("profile_kind=named profile_source=cli")
    );
    assert!(
        redacted_configuration
            .stderr
            .contains("config_dir_source=cli")
    );

    let mutation_canary = "release-c-super-secret-task-canary";
    let mutation = run_cli_with_operator_env(
        home.path(),
        "https://operator.example.test",
        &[
            "-vv",
            "tasks",
            "create",
            "--work-list-id",
            &Uuid::now_v7().to_string(),
            "--title",
            mutation_canary,
            "--body",
            mutation_canary,
            "--password-stdin",
        ],
        &[],
        Some("unused-password"),
    );
    assert!(!mutation.status.success());
    assert!(!mutation.stdout.contains(mutation_canary));
    assert!(!mutation.stderr.contains(mutation_canary));
    assert!(mutation.stderr.contains("[sealtask] event=finish"));

    for flag in ["-v", "-vv", "--debug"] {
        let output = run_cli_with_operator_env(
            home.path(),
            "https://operator.example.test",
            &["--json", flag, "info"],
            &[],
            None,
        );
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.contains("[sealtask]"));
        assert_json_error_contains(&output.stderr, "cannot be combined with JSON");
    }

    let pretty_json = run_cli_with_operator_env(
        home.path(),
        "https://operator.example.test",
        &["--format", "json-pretty", "--debug", "info"],
        &[],
        None,
    );
    assert_eq!(pretty_json.status.code(), Some(1));
    assert!(pretty_json.stdout.is_empty());
    assert!(!pretty_json.stderr.contains("[sealtask]"));
    assert_json_error_contains(&pretty_json.stderr, "cannot be combined with JSON");
}

#[test]
fn cli_operator_timeout_validation_is_actionable_and_precedes_network_use() {
    let home = TempDir::new().expect("temp home");
    let invalid_duration = run_cli_with_operator_env(
        home.path(),
        "https://unreachable.invalid",
        &[
            "--json",
            "--connect-timeout",
            "0s",
            "config",
            "show",
            "--resolved",
        ],
        &[],
        None,
    );
    assert_eq!(invalid_duration.status.code(), Some(1));
    assert!(invalid_duration.stdout.is_empty());
    assert_json_error_contains(&invalid_duration.stderr, "between 1ms and 24h");

    let invalid_order = run_cli_with_operator_env(
        home.path(),
        "https://unreachable.invalid",
        &[
            "--json",
            "--connect-timeout",
            "500ms",
            "--read-timeout",
            "2s",
            "--request-timeout",
            "1s",
            "config",
            "show",
            "--resolved",
        ],
        &[],
        None,
    );
    assert_eq!(invalid_order.status.code(), Some(1));
    assert!(invalid_order.stdout.is_empty());
    assert_json_error_contains(
        &invalid_order.stderr,
        "API read timeout cannot exceed the overall request timeout",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_mutations_honor_human_output_and_automation_idempotency() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let human = run_cli(
        home.path(),
        &server.base_url,
        &[
            "tasks",
            "create",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--title",
            "Human-readable result",
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        human.status.success(),
        "human create failed: {}",
        human.stderr
    );
    assert!(human.stdout.starts_with("Task "));
    assert!(!human.stdout.trim_start().starts_with('{'));

    let non_interactive = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "--non-interactive",
            "tasks",
            "create",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--title",
            "Unsafe retry",
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(!non_interactive.status.success());
    let error = parse_stderr_json(&non_interactive.stderr);
    assert_eq!(error["error"]["code"], "validation");
    assert!(
        error["error"]["message"]
            .as_str()
            .expect("message")
            .contains("requires --idempotency-key")
    );
}

#[test]
fn cli_structured_inputs_conflict_with_scalar_fields_at_parse_time() {
    let home = TempDir::new().expect("temp home");
    let input_path = home.path().join("task.json");
    std::fs::write(&input_path, r#"{"title":"from file"}"#).expect("write task input");
    let output = run_cli(
        home.path(),
        "https://sealtask.com",
        &[
            "--json",
            "tasks",
            "create",
            "--work-list-id",
            &Uuid::now_v7().to_string(),
            "--input-file",
            input_path.to_str().expect("UTF-8 input path"),
            "--title",
            "silently ignored before",
        ],
        None,
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_json_error_contains(&output.stderr, "--input-file");
    assert_json_error_contains(&output.stderr, "--title");
}

#[test]
fn cli_public_name_is_sealtask() {
    let home = TempDir::new().expect("temp home");
    let help = run_cli(home.path(), "https://sealtask.com", &["--help"], None);
    assert!(help.status.success(), "help failed: {}", help.stderr);
    assert!(
        help.stdout.contains("Usage: sealtask"),
        "unexpected help output: {}",
        help.stdout
    );

    let info = run_cli(
        home.path(),
        "https://sealtask.com",
        &["--json", "info"],
        None,
    );
    assert!(info.status.success(), "info failed: {}", info.stderr);
    assert_eq!(parse_stdout_json(&info.stdout)["commandName"], "sealtask");
}

#[test]
fn cli_discovery_artifacts_are_local_complete_and_pipe_safe() {
    let home = TempDir::new().expect("temp home");
    let mut completion = Command::cargo_bin("sealtask").expect("binary");
    completion
        .env("HOME", home.path())
        .env("SEALTASK_API_URL", "not a URL")
        .env("SEALTASK_CONNECT_TIMEOUT", "not a duration")
        .env("SEALTASK_COLOR", "never")
        .env("SEALTASK_PAGER_MODE", "never")
        .env("SEALTASK_PROGRESS", "never")
        .current_dir(home.path())
        .args(["completion", "zsh"]);
    let completion = completion.output().expect("generate completion");
    assert!(
        completion.status.success(),
        "completion failed: {}",
        String::from_utf8_lossy(&completion.stderr)
    );
    let completion_stdout =
        String::from_utf8(completion.stdout).expect("completion output should be UTF-8");
    assert!(completion_stdout.contains("#compdef sealtask"));
    assert!(!home.path().join(".sealtask").exists());

    for (shell, marker) in [
        ("bash", "_sealtask"),
        ("fish", "complete -c sealtask"),
        ("powershell", "Register-ArgumentCompleter"),
    ] {
        let output = run_cli_exact(home.path(), &["completion", shell], None);
        assert!(
            output.status.success(),
            "{shell} completion failed: {}",
            output.stderr
        );
        assert!(output.stderr.is_empty(), "{shell}: {}", output.stderr);
        assert!(
            output.stdout.contains(marker),
            "{shell} output missing {marker:?}"
        );
    }
    for unsupported in ["power-shell", "elvish"] {
        let output = run_cli_exact(home.path(), &["completion", unsupported], None);
        assert_eq!(output.status.code(), Some(2), "{unsupported}");
    }

    let man = run_cli_exact(home.path(), &["man", "lists", "get"], None);
    assert!(man.status.success(), "man failed: {}", man.stderr);
    assert!(man.stderr.is_empty());
    assert!(man.stdout.contains(".TH sealtask-projects-get 1"));
    assert!(man.stdout.contains("sealtask projects get"));

    let broken_pipe = run_cli_with_closed_stdout(
        home.path(),
        "https://sealtask.com",
        &["completion", "bash"],
        None,
    );
    assert!(
        broken_pipe.status.success(),
        "broken completion pipe failed: {}",
        broken_pipe.stderr
    );

    let man_dir = home.path().join("man");
    let man_set = run_cli_exact(
        home.path(),
        &[
            "man",
            "--output-dir",
            man_dir.to_str().expect("UTF-8 man directory"),
        ],
        None,
    );
    assert!(
        man_set.status.success(),
        "man set failed: {}",
        man_set.stderr
    );
    assert!(man_dir.join("sealtask.1").is_file());
    assert!(
        man_dir
            .join("sealtask-tasks-attachments-upload.1")
            .is_file()
    );
    assert!(!man_dir.join("sealtask-inspect.1").exists());
    assert!(
        std::fs::read_dir(&man_dir)
            .expect("read man directory")
            .all(|entry| !entry
                .expect("man directory entry")
                .file_name()
                .to_string_lossy()
                .contains("-help"))
    );
}

#[test]
fn cli_long_help_groups_options_and_covers_realistic_examples() {
    let home = TempDir::new().expect("temp home");
    let help = run_cli_exact(home.path(), &["tasks", "create", "--help"], None);
    assert!(help.status.success(), "help failed: {}", help.stderr);
    for expected in [
        "Target:",
        "Fields:",
        "Input:",
        "Advanced:",
        "Connection:",
        "Examples:",
        "sealtask tasks create --title \"Ship 0.4\" --due tomorrow",
    ] {
        assert!(
            help.stdout.contains(expected),
            "missing {expected:?} in:\n{}",
            help.stdout
        );
    }
}

#[test]
fn cli_discovery_rejects_modes_that_would_corrupt_generated_output() {
    let home = TempDir::new().expect("temp home");
    for args in [
        &["--json", "completion", "fish"][..],
        &["-v", "man", "tasks", "list"][..],
        &["--color", "never", "completion", "bash"][..],
        &["--quiet", "man"][..],
    ] {
        let output = run_cli_exact(home.path(), args, None);
        assert_eq!(output.status.code(), Some(1), "{args:?}: {}", output.stderr);
        assert!(output.stdout.is_empty(), "{args:?}: {}", output.stdout);
        assert!(
            output.stderr.contains("corrupt generated output")
                || output.stderr.contains("raw terminal integration artifact")
                || output.stderr.contains("exact raw artifact"),
            "{args:?}: {}",
            output.stderr
        );
    }
}

#[test]
fn cli_root_without_arguments_matches_release_a_help_contract() {
    let home = TempDir::new().expect("temp home");
    let output = run_cli_exact(home.path(), &[], None);

    assert!(
        output.status.success(),
        "root guidance failed: {}",
        output.stderr
    );
    assert!(output.stderr.is_empty());
    assert_eq!(
        output.stdout,
        include_str!("golden/release_a_root_help.txt")
    );
}

#[test]
fn cli_root_guidance_is_consistent_with_human_global_options() {
    let home = TempDir::new().expect("temp home");
    for args in [
        &["--profile", "operator"][..],
        &["--format", "table"][..],
        &["--non-interactive"][..],
    ] {
        let output = run_cli_exact(home.path(), args, None);
        assert!(
            output.status.success(),
            "root guidance failed for {args:?}: {}",
            output.stderr
        );
        assert!(output.stderr.is_empty());
        assert_eq!(
            output.stdout,
            include_str!("golden/release_a_root_help.txt")
        );
    }
}

#[test]
fn cli_root_json_error_is_one_structured_document() {
    let home = TempDir::new().expect("temp home");
    let output = run_cli_exact(home.path(), &["--json"], None);

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(output.stderr.lines().count(), 1);
    let error = parse_stderr_json(&output.stderr);
    assert_eq!(error["error"]["code"], "validation");
    assert!(
        error["error"]["message"]
            .as_str()
            .expect("error message")
            .contains("sealtask --help")
    );
}

#[test]
fn cli_human_errors_include_stable_code_and_recovery_hint() {
    let home = TempDir::new().expect("temp home");
    let output = run_cli_exact(home.path(), &["--api-url", "not-a-url", "info"], None);

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(output.stderr.starts_with("error [validation]:"));
    assert!(
        output
            .stderr
            .ends_with("hint: Review command help and the rejected input field.\n")
    );
}

#[test]
fn cli_all_machine_deletes_require_yes_without_contacting_the_api() {
    let home = TempDir::new().expect("temp home");
    let work_list_id = Uuid::now_v7().to_string();
    let task_id = Uuid::now_v7().to_string();
    let comment_id = Uuid::now_v7().to_string();
    let note_id = Uuid::now_v7().to_string();
    let attachment_id = Uuid::now_v7().to_string();
    let cases = [
        (
            "task",
            vec![
                "--json",
                "--non-interactive",
                "tasks",
                "delete",
                "--work-list-id",
                &work_list_id,
                "--task-id",
                &task_id,
            ],
        ),
        (
            "comment",
            vec![
                "--json",
                "--non-interactive",
                "comments",
                "delete",
                "--work-list-id",
                &work_list_id,
                "--task-id",
                &task_id,
                "--comment-id",
                &comment_id,
            ],
        ),
        (
            "note",
            vec![
                "--json",
                "--non-interactive",
                "notes",
                "delete",
                "--work-list-id",
                &work_list_id,
                "--note-id",
                &note_id,
            ],
        ),
        (
            "attachment",
            vec![
                "--json",
                "--non-interactive",
                "tasks",
                "attachments",
                "delete",
                "--work-list-id",
                &work_list_id,
                "--task-id",
                &task_id,
                "--attachment-id",
                &attachment_id,
            ],
        ),
    ];

    for (entity, args) in cases {
        let output = run_cli(home.path(), "https://sealtask.com", &args, None);

        assert_eq!(output.status.code(), Some(1), "{entity}: {}", output.stderr);
        assert!(output.stdout.is_empty(), "{entity}");
        assert_eq!(output.stderr.lines().count(), 1, "{entity}");
        let error = parse_stderr_json(&output.stderr);
        assert_eq!(error["error"]["code"], "validation", "{entity}");
        let message = error["error"]["message"].as_str().expect("error message");
        assert!(message.contains(entity), "{entity}: {message}");
        assert!(message.contains("requires --yes"), "{entity}: {message}");
    }
}

#[test]
fn cli_destructive_help_exposes_explicit_confirmation() {
    let home = TempDir::new().expect("temp home");
    let cases = [
        &["tasks", "delete", "--help"][..],
        &["comments", "delete", "--help"][..],
        &["notes", "delete", "--help"][..],
        &["tasks", "attachments", "delete", "--help"][..],
    ];

    for args in cases {
        let output = run_cli_exact(home.path(), args, None);
        assert!(output.status.success(), "{args:?}: {}", output.stderr);
        assert!(
            output.stdout.contains("--yes"),
            "{args:?}: {}",
            output.stdout
        );
    }
}

#[test]
fn cli_json_auth_prompts_never_pollute_machine_streams_without_a_terminal() {
    let home = TempDir::new().expect("temp home");
    for args in [
        &["--json", "auth", "login"][..],
        &["--format", "json-pretty", "auth", "login"][..],
    ] {
        let output = run_cli_exact(home.path(), args, None);

        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert!(output.stdout.is_empty(), "{args:?}: {}", output.stdout);
        let error = parse_stderr_json(&output.stderr);
        assert_eq!(error["error"]["code"], "validation", "{args:?}");
        assert!(!output.stderr.contains("Email:"), "{args:?}");
        assert!(!output.stderr.contains("Password:"), "{args:?}");
    }
}

#[test]
fn cli_machine_delete_with_stdin_reserved_still_requires_yes_before_reading() {
    let home = TempDir::new().expect("temp home");
    let work_list_id = Uuid::now_v7().to_string();
    let task_id = Uuid::now_v7().to_string();
    let output = run_cli(
        home.path(),
        "https://sealtask.com",
        &[
            "--json",
            "tasks",
            "delete",
            "--work-list-id",
            &work_list_id,
            "--task-id",
            &task_id,
            "--input-stdin",
        ],
        Some("this payload must not be parsed"),
    );

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(output.stderr.lines().count(), 1);
    let error = parse_stderr_json(&output.stderr);
    assert_eq!(error["error"]["code"], "validation");
    let message = error["error"]["message"].as_str().expect("error message");
    assert!(message.contains("requires --yes"));
    assert!(!message.contains("JSON"));
}

#[test]
fn cli_logged_out_human_status_gives_the_next_command() {
    let home = TempDir::new().expect("temp home");
    let output = run_cli(
        home.path(),
        "https://sealtask.com",
        &["auth", "status"],
        None,
    );

    assert!(output.status.success(), "status failed: {}", output.stderr);
    assert!(output.stdout.contains("Workspace data: locked"));
    assert!(output.stdout.contains("Next: sealtask auth login"));
}

#[test]
fn cli_logged_in_but_locked_status_gives_the_unlock_command() {
    let fixture = TestFixture::new();
    let home = TempDir::new().expect("temp home");
    let api_url = "https://operator-status.sealtask.example";
    seed_credentials(home.path(), &fixture, api_url);

    let output = run_cli(home.path(), api_url, &["auth", "status"], None);

    assert!(output.status.success(), "status failed: {}", output.stderr);
    assert!(output.stdout.contains("Workspace data: locked"));
    assert!(output.stdout.contains("Saved unlock key:"));
    assert!(output.stdout.contains("Next: sealtask auth unlock"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_task_detail_table_lists_attachments() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "tasks",
            "get",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--task-id",
            &fixture.task_id.to_string(),
            "--password-stdin",
        ],
        Some(&fixture.password),
    );

    assert!(
        output.status.success(),
        "task get failed: {}",
        output.stderr
    );
    assert!(output.stdout.contains("Attachments"));
    let attachment_id_prefix = unique_uuid_prefix(
        fixture.text_attachment.id,
        &[
            fixture.text_attachment.id,
            fixture.docx_attachment.id,
            fixture.binary_attachment.id,
            fixture.hostile_attachment.id,
        ],
    );
    assert!(
        output.stdout.contains(&attachment_id_prefix),
        "attachment table did not include the selectable ID prefix: {}",
        output.stdout
    );
    assert!(output.stdout.contains("notes.md"));
    assert!(output.stdout.contains("spec.pdf"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_reads_text_attachment_to_stdout() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let attachment_id_prefix = unique_uuid_prefix(
        fixture.text_attachment.id,
        &[
            fixture.text_attachment.id,
            fixture.docx_attachment.id,
            fixture.binary_attachment.id,
            fixture.hostile_attachment.id,
        ],
    );
    let output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "tasks",
            "attachments",
            "read",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--task-id",
            &fixture.task_id.to_string(),
            "--attachment-id",
            &attachment_id_prefix,
            "--password-stdin",
        ],
        Some(&fixture.password),
    );

    assert!(
        output.status.success(),
        "attachment read failed: {}",
        output.stderr
    );
    assert_eq!(output.stdout, "# Heading\n\nAttachment body\n");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_attachment_read_exits_zero_when_stdout_pipe_closes() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let output = run_cli_with_closed_stdout(
        home.path(),
        &server.base_url,
        &[
            "tasks",
            "attachments",
            "read",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--task-id",
            &fixture.task_id.to_string(),
            "--attachment-id",
            &fixture.text_attachment.id.to_string(),
            "--password-stdin",
        ],
        Some(&fixture.password),
    );

    assert!(
        output.status.success(),
        "attachment read with closed stdout failed: {}",
        output.stderr
    );
    assert_eq!(output.stderr, "");
    assert_eq!(output.stdout, "");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_reads_markdown_text_attachment_as_json() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "tasks",
            "attachments",
            "read",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--task-id",
            &fixture.task_id.to_string(),
            "--attachment-id",
            &fixture.text_attachment.id.to_string(),
            "--password-stdin",
        ],
        Some(&fixture.password),
    );

    assert!(
        output.status.success(),
        "markdown text attachment json read failed: {}",
        output.stderr
    );
    let attachment_json = parse_stdout_json(&output.stdout);
    assert_eq!(
        attachment_json["attachment"]["id"],
        fixture.text_attachment.id.to_string()
    );
    assert_eq!(attachment_json["contentFormat"], "markdown");
    assert_eq!(attachment_json["sourceKind"], "plain_text");
    assert_eq!(attachment_json["text"], "# Heading\n\nAttachment body\n");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_reads_docx_attachment_to_markdown_stdout() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "tasks",
            "attachments",
            "read",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--task-id",
            &fixture.task_id.to_string(),
            "--attachment-id",
            &fixture.docx_attachment.id.to_string(),
            "--password-stdin",
        ],
        Some(&fixture.password),
    );

    assert!(
        output.status.success(),
        "docx attachment read failed: {}",
        output.stderr
    );
    assert_eq!(output.stdout, "Heading\n\nDOCX body\n\n");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_reads_docx_attachment_as_json() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "tasks",
            "attachments",
            "read",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--task-id",
            &fixture.task_id.to_string(),
            "--attachment-id",
            &fixture.docx_attachment.id.to_string(),
            "--password-stdin",
        ],
        Some(&fixture.password),
    );

    assert!(
        output.status.success(),
        "docx attachment json read failed: {}",
        output.stderr
    );
    let attachment_json = parse_stdout_json(&output.stdout);
    assert_eq!(
        attachment_json["attachment"]["id"],
        fixture.docx_attachment.id.to_string()
    );
    assert_eq!(attachment_json["attachment"]["fileName"], "spec.docx");
    assert_eq!(attachment_json["contentFormat"], "markdown");
    assert_eq!(attachment_json["sourceKind"], "docx_rendered");
    assert_eq!(attachment_json["text"], "Heading\n\nDOCX body\n\n");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_rejects_binary_attachment_reads() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state.clone()).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "tasks",
            "attachments",
            "read",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--task-id",
            &fixture.task_id.to_string(),
            "--attachment-id",
            &fixture.binary_attachment.id.to_string(),
            "--password-stdin",
        ],
        Some(&fixture.password),
    );

    assert!(
        !output.status.success(),
        "binary attachment read unexpectedly succeeded"
    );
    assert!(output.stderr.contains("use download instead"));
    let state = state.lock().expect("state lock");
    assert_eq!(state.attachment_download_requests, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_downloads_binary_attachment_to_default_filename() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state).await;
    let home = TempDir::new().expect("temp home");
    let download_dir = TempDir::new().expect("download dir");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let output = run_cli_in_dir(
        home.path(),
        download_dir.path(),
        &server.base_url,
        &[
            "tasks",
            "attachments",
            "download",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--task-id",
            &fixture.task_id.to_string(),
            "--attachment-id",
            &fixture.binary_attachment.id.to_string(),
            "--password-stdin",
        ],
        Some(&fixture.password),
    );

    assert!(
        output.status.success(),
        "attachment download failed: {}",
        output.stderr
    );
    let saved_path = download_dir.path().join("spec.pdf");
    assert_eq!(
        std::fs::read(&saved_path).expect("saved attachment"),
        fixture.binary_attachment.plaintext_bytes
    );
    assert!(output.stdout.contains("Saved attachment"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_download_respects_output_path_and_force() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state).await;
    let home = TempDir::new().expect("temp home");
    let download_dir = TempDir::new().expect("download dir");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let custom_relative_path = std::path::Path::new("nested").join("custom.bin");
    let custom_path = download_dir.path().join(&custom_relative_path);
    std::fs::create_dir_all(custom_path.parent().expect("parent")).expect("create parent");
    std::fs::write(&custom_path, b"existing").expect("write existing");

    let first_output = run_cli_in_dir(
        home.path(),
        download_dir.path(),
        &server.base_url,
        &[
            "tasks",
            "attachments",
            "download",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--task-id",
            &fixture.task_id.to_string(),
            "--attachment-id",
            &fixture.binary_attachment.id.to_string(),
            "--output",
            custom_relative_path.to_str().expect("utf8 path"),
            "--password-stdin",
        ],
        Some(&fixture.password),
    );

    assert!(
        !first_output.status.success(),
        "download unexpectedly overwrote file"
    );
    assert!(
        first_output.stderr.contains("already exists"),
        "unexpected stderr: {}",
        first_output.stderr
    );

    let second_output = run_cli_in_dir(
        home.path(),
        download_dir.path(),
        &server.base_url,
        &[
            "--json",
            "tasks",
            "attachments",
            "download",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--task-id",
            &fixture.task_id.to_string(),
            "--attachment-id",
            &fixture.binary_attachment.id.to_string(),
            "--output",
            custom_relative_path.to_str().expect("utf8 path"),
            "--force",
            "--password-stdin",
        ],
        Some(&fixture.password),
    );

    assert!(
        second_output.status.success(),
        "forced download failed: {}",
        second_output.stderr
    );
    assert_eq!(
        std::fs::read(&custom_path).expect("forced download"),
        fixture.binary_attachment.plaintext_bytes
    );
    let result_json: Value = parse_stdout_json(&second_output.stdout);
    assert_eq!(result_json["fileName"], "spec.pdf");
    assert_eq!(
        result_json["outputPath"],
        custom_relative_path.display().to_string()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_download_sanitizes_default_attachment_filename() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state).await;
    let home = TempDir::new().expect("temp home");
    let download_dir = TempDir::new().expect("download dir");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let output = run_cli_in_dir(
        home.path(),
        download_dir.path(),
        &server.base_url,
        &[
            "tasks",
            "attachments",
            "download",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--task-id",
            &fixture.task_id.to_string(),
            "--attachment-id",
            &fixture.hostile_attachment.id.to_string(),
            "--password-stdin",
        ],
        Some(&fixture.password),
    );

    assert!(
        output.status.success(),
        "sanitized download failed: {}",
        output.stderr
    );
    let saved_path = download_dir.path().join("unsafe.txt");
    assert_eq!(
        std::fs::read(&saved_path).expect("sanitized attachment"),
        fixture.hostile_attachment.plaintext_bytes
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_attachment_download_rejects_size_mismatch() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    {
        let mut guard = state.lock().expect("state lock");
        guard.attachment_size_mismatch = true;
    }
    let server = spawn_server(state).await;
    let home = TempDir::new().expect("temp home");
    let download_dir = TempDir::new().expect("download dir");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let output = run_cli_in_dir(
        home.path(),
        download_dir.path(),
        &server.base_url,
        &[
            "tasks",
            "attachments",
            "download",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--task-id",
            &fixture.task_id.to_string(),
            "--attachment-id",
            &fixture.binary_attachment.id.to_string(),
            "--password-stdin",
        ],
        Some(&fixture.password),
    );

    assert!(
        !output.status.success(),
        "size mismatch download unexpectedly succeeded"
    );
    assert!(output.stderr.contains("download size mismatch"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_rejects_input_stdin_with_password_stdin() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "comments",
            "create",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--task-id",
            &fixture.task_id.to_string(),
            "--input-stdin",
            "--password-stdin",
        ],
        Some(r#"{"body":"hello"}"#),
    );

    assert!(!output.status.success(), "command unexpectedly succeeded");
    assert!(output.stderr.contains("--input-stdin"));
    assert!(output.stderr.contains("--password-stdin"));
    assert!(output.stderr.contains("cannot be used with"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_unlock_daemon_enables_later_decrypt_without_password_flag() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state.clone()).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let unlock_output = run_cli(
        home.path(),
        &server.base_url,
        &["auth", "unlock", "--ttl-seconds", "300", "--password-stdin"],
        Some(&fixture.password),
    );
    assert!(
        unlock_output.status.success(),
        "unlock failed: {}",
        unlock_output.stderr
    );
    assert!(unlock_output.stdout.contains("Next: sealtask lists"));

    let task_output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "tasks",
            "get",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--task-id",
            &fixture.task_id.to_string(),
        ],
        None,
    );
    assert!(
        task_output.status.success(),
        "task get without password flag failed: {}",
        task_output.stderr
    );
    let task_json: Value = parse_stdout_json(&task_output.stdout);
    assert_eq!(task_json["title"], "Existing task");

    let lock_output = run_cli(home.path(), &server.base_url, &["auth", "lock"], None);
    assert!(
        lock_output.status.success(),
        "lock failed: {}",
        lock_output.stderr
    );
    assert!(lock_output.stdout.contains("Next: sealtask auth unlock"));
    assert_eq!(
        state
            .lock()
            .expect("state lock")
            .opaque_export_key_start_count,
        0,
        "legacy v1 unlock must remain offline"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_keychain_store_bootstraps_later_decrypt_without_password_flag() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state.clone()).await;
    let home = TempDir::new().expect("temp home");
    let keychain_dir = TempDir::new().expect("temp keychain");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let store_output = run_cli_with_test_keychain(
        home.path(),
        &server.base_url,
        keychain_dir.path(),
        &["--json", "auth", "keychain", "store", "--password-stdin"],
        Some(&fixture.password),
    );
    assert!(
        store_output.status.success(),
        "keychain store failed: {}",
        store_output.stderr
    );
    let store_json: Value = parse_stdout_json(&store_output.stdout);
    assert_eq!(store_json["persistedBootstrap"]["status"], "available");

    let initial_status = run_cli_with_test_keychain(
        home.path(),
        &server.base_url,
        keychain_dir.path(),
        &["--json", "auth", "status"],
        None,
    );
    assert!(
        initial_status.status.success(),
        "status failed: {}",
        initial_status.stderr
    );
    let initial_status_json: Value = parse_stdout_json(&initial_status.stdout);
    assert_eq!(initial_status_json["loggedIn"], true);
    assert_eq!(initial_status_json["sessionState"], "active");
    assert_eq!(initial_status_json["unlockDaemon"]["active"], false);
    assert_eq!(
        initial_status_json["persistedBootstrap"]["status"],
        "available"
    );

    let task_output = run_cli_with_test_keychain(
        home.path(),
        &server.base_url,
        keychain_dir.path(),
        &[
            "--json",
            "tasks",
            "get",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--task-id",
            &fixture.task_id.to_string(),
        ],
        None,
    );
    assert!(
        task_output.status.success(),
        "task get without password flag failed: {}",
        task_output.stderr
    );
    let task_json: Value = parse_stdout_json(&task_output.stdout);
    assert_eq!(task_json["title"], "Existing task");

    let seeded_status = run_cli_with_test_keychain(
        home.path(),
        &server.base_url,
        keychain_dir.path(),
        &["--json", "auth", "status"],
        None,
    );
    assert!(
        seeded_status.status.success(),
        "status after bootstrap failed: {}",
        seeded_status.stderr
    );
    let seeded_status_json: Value = parse_stdout_json(&seeded_status.stdout);
    assert_eq!(seeded_status_json["unlockDaemon"]["active"], true);
    assert_eq!(
        seeded_status_json["persistedBootstrap"]["status"],
        "available"
    );

    let _ = run_cli(home.path(), &server.base_url, &["auth", "lock"], None);
    assert_eq!(
        state
            .lock()
            .expect("state lock")
            .opaque_export_key_start_count,
        0,
        "legacy v1 keychain bootstrap must remain offline"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_v2_single_command_unlock_derives_the_opaque_export_key() {
    let fixture = TestFixture::new_v2();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state.clone()).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "lists",
            "get",
            &fixture.work_list_id.to_string(),
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        output.status.success(),
        "v2 single-command unlock failed: {}",
        output.stderr
    );
    assert_eq!(
        parse_stdout_json(&output.stdout)["title"],
        "Fixture Work List"
    );
    assert_eq!(
        state
            .lock()
            .expect("state lock")
            .opaque_export_key_start_count,
        1
    );

    let credentials = std::fs::read_to_string(home.path().join(".sealtask/credentials.json"))
        .expect("read credentials");
    assert!(!credentials.contains(&STANDARD_NO_PAD.encode(fixture.opaque_export_key)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_v2_selector_mutation_reuses_one_password_derived_key() {
    let fixture = TestFixture::new_v2();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state.clone()).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "tasks",
            "update",
            "Existing task",
            "--project",
            "Fixture Work List",
            "--section",
            "Planning",
            "--title",
            "V2 selector update",
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        output.status.success(),
        "v2 selector mutation failed: {}",
        output.stderr
    );
    let output = parse_stdout_json(&output.stdout);
    assert_eq!(output["title"], "V2 selector update");
    assert_eq!(output["sectionId"], fixture.first_section_id.to_string());
    assert_eq!(
        state
            .lock()
            .expect("state lock")
            .opaque_export_key_start_count,
        1,
        "project, task, section, and mutation discovery must share one command key"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_v2_unlock_daemon_enables_later_decrypt_without_password_flag() {
    let fixture = TestFixture::new_v2();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state.clone()).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let unlock_output = run_cli(
        home.path(),
        &server.base_url,
        &["auth", "unlock", "--ttl-seconds", "300", "--password-stdin"],
        Some(&fixture.password),
    );
    assert!(
        unlock_output.status.success(),
        "v2 daemon unlock failed: {}",
        unlock_output.stderr
    );

    let task_output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "tasks",
            "get",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--task-id",
            &fixture.task_id.to_string(),
        ],
        None,
    );
    assert!(
        task_output.status.success(),
        "v2 daemon-backed task get failed: {}",
        task_output.stderr
    );
    assert_eq!(
        parse_stdout_json(&task_output.stdout)["title"],
        "Existing task"
    );
    assert_eq!(
        state
            .lock()
            .expect("state lock")
            .opaque_export_key_start_count,
        1
    );

    let lock_output = run_cli(home.path(), &server.base_url, &["auth", "lock"], None);
    assert!(lock_output.status.success(), "v2 daemon lock failed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_v2_keychain_bootstrap_refreshes_credentials_without_persisting_the_export_key() {
    let fixture = TestFixture::new_v2();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state.clone()).await;
    let home = TempDir::new().expect("temp home");
    let keychain_dir = TempDir::new().expect("temp keychain");
    seed_credentials_with_expiry(
        home.path(),
        &fixture,
        &server.base_url,
        Utc::now() - Duration::minutes(1),
        Utc::now() + Duration::days(1),
    );

    let store_output = run_cli_with_test_keychain(
        home.path(),
        &server.base_url,
        keychain_dir.path(),
        &["--json", "auth", "keychain", "store", "--password-stdin"],
        Some(&fixture.password),
    );
    assert!(
        store_output.status.success(),
        "v2 keychain bootstrap failed: {}",
        store_output.stderr
    );
    assert_eq!(
        parse_stdout_json(&store_output.stdout)["persistedBootstrap"]["status"],
        "available"
    );

    let task_output = run_cli_with_test_keychain(
        home.path(),
        &server.base_url,
        keychain_dir.path(),
        &[
            "--json",
            "tasks",
            "get",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--task-id",
            &fixture.task_id.to_string(),
        ],
        None,
    );
    assert!(
        task_output.status.success(),
        "v2 keychain-backed task get failed: {}",
        task_output.stderr
    );
    assert_eq!(
        parse_stdout_json(&task_output.stdout)["title"],
        "Existing task"
    );

    let state = state.lock().expect("state lock");
    assert_eq!(state.refresh_request_count, 1);
    assert_eq!(state.opaque_export_key_start_count, 1);
    drop(state);

    let encoded_export_key = STANDARD_NO_PAD.encode(fixture.opaque_export_key);
    let credentials = std::fs::read_to_string(home.path().join(".sealtask/credentials.json"))
        .expect("read refreshed credentials");
    let keychain_secret = read_stored_test_keychain_secret(keychain_dir.path());
    assert!(!credentials.contains(&encoded_export_key));
    assert_eq!(keychain_secret, vec![0x11; 32]);

    let _ = run_cli(home.path(), &server.base_url, &["auth", "lock"], None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_v2_wrong_password_does_not_persist_a_partial_keychain_secret() {
    let fixture = TestFixture::new_v2();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state.clone()).await;
    let home = TempDir::new().expect("temp home");
    let keychain_dir = TempDir::new().expect("temp keychain");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let wrong_password = "definitely-not-the-account-password";
    let output = run_cli_with_test_keychain(
        home.path(),
        &server.base_url,
        keychain_dir.path(),
        &["--json", "auth", "keychain", "store", "--password-stdin"],
        Some(wrong_password),
    );

    assert!(
        !output.status.success(),
        "wrong password unexpectedly worked"
    );
    assert!(
        output.stdout.is_empty(),
        "unexpected stdout: {}",
        output.stdout
    );
    assert!(!output.stderr.contains(wrong_password));
    assert_eq!(
        std::fs::read_dir(keychain_dir.path())
            .expect("list keychain dir")
            .count(),
        0
    );
    assert_eq!(
        state
            .lock()
            .expect("state lock")
            .opaque_export_key_start_count,
        1
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_logout_locks_daemon() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let unlock_output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "auth",
            "unlock",
            "--ttl-seconds",
            "300",
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        unlock_output.status.success(),
        "unlock failed: {}",
        unlock_output.stderr
    );
    let unlock_json: Value = parse_stdout_json(&unlock_output.stdout);
    assert_eq!(unlock_json["unlocked"], true);
    assert_eq!(unlock_json["ttlSeconds"], 300);

    let logout_output = run_cli(
        home.path(),
        &server.base_url,
        &["--json", "auth", "logout"],
        None,
    );
    assert!(
        logout_output.status.success(),
        "logout failed: {}",
        logout_output.stderr
    );
    let logout_json: Value = parse_stdout_json(&logout_output.stdout);
    assert_eq!(logout_json["loggedOut"], true);

    let status_output = run_cli(
        home.path(),
        &server.base_url,
        &["--json", "auth", "status"],
        None,
    );
    assert!(
        status_output.status.success(),
        "status failed: {}",
        status_output.stderr
    );
    let status_json: Value = parse_stdout_json(&status_output.stdout);
    assert_eq!(status_json["loggedIn"], false);
    assert_eq!(status_json["unlockDaemon"]["active"], false);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_logout_does_not_revoke_a_concurrently_refreshed_session() {
    let fixture = TestFixture::new();
    let server = spawn_refresh_logout_race_server(&fixture).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials_with_expiry(
        home.path(),
        &fixture,
        &server.base_url,
        Utc::now() - Duration::minutes(5),
        Utc::now() + Duration::days(1),
    );

    let refresh = spawn_cli_process(home.path(), &server.base_url, &["--json", "lists", "--raw"]);
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        server.refresh_committed.notified(),
    )
    .await
    .expect("refresh should commit while retaining the credential lock");

    let logout = spawn_cli_process(home.path(), &server.base_url, &["--json", "auth", "logout"]);
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    server.release_refresh_response.notify_one();

    let (refresh_output, logout_output) =
        tokio::join!(wait_for_cli_process(refresh), wait_for_cli_process(logout));
    assert!(
        refresh_output.status.success(),
        "refreshing command failed: {}",
        refresh_output.stderr
    );
    assert!(
        !logout_output.status.success(),
        "stale logout unexpectedly succeeded: {}",
        logout_output.stdout
    );
    let logout_error = parse_stderr_json(&logout_output.stderr);
    assert_eq!(logout_error["error"]["code"], "conflict");
    assert!(
        logout_error["error"]["message"]
            .as_str()
            .expect("logout conflict message")
            .contains("credentials changed while the command was running"),
        "unexpected logout error: {}",
        logout_output.stderr
    );

    let state = server.state.lock().expect("race state lock");
    assert_eq!(state.refresh_requests, 1);
    assert_eq!(state.logout_requests, 0);
    assert_eq!(state.work_list_requests, 1);
    assert!(!state.revoked);
    drop(state);

    let credentials_path = home.path().join(".sealtask").join("credentials.json");
    let persisted: Credentials =
        serde_json::from_slice(&std::fs::read(credentials_path).expect("read rotated credentials"))
            .expect("parse rotated credentials");
    assert_eq!(persisted.access_token, "race-access-token");
    assert_eq!(persisted.refresh_token, "race-refresh-token");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_logout_clears_persisted_bootstrap() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state).await;
    let home = TempDir::new().expect("temp home");
    let keychain_dir = TempDir::new().expect("temp keychain");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let store_output = run_cli_with_test_keychain(
        home.path(),
        &server.base_url,
        keychain_dir.path(),
        &["auth", "keychain", "store", "--password-stdin"],
        Some(&fixture.password),
    );
    assert!(
        store_output.status.success(),
        "keychain store failed: {}",
        store_output.stderr
    );

    let logout_output = run_cli_with_test_keychain(
        home.path(),
        &server.base_url,
        keychain_dir.path(),
        &["auth", "logout"],
        None,
    );
    assert!(
        logout_output.status.success(),
        "logout failed: {}",
        logout_output.stderr
    );
    assert!(logout_output.stdout.contains("Next: sealtask auth login"));

    seed_credentials(home.path(), &fixture, &server.base_url);
    let task_output = run_cli_with_test_keychain(
        home.path(),
        &server.base_url,
        keychain_dir.path(),
        &[
            "--json",
            "tasks",
            "get",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--task-id",
            &fixture.task_id.to_string(),
        ],
        None,
    );
    assert!(
        !task_output.status.success(),
        "task get unexpectedly succeeded after logout"
    );
    assert!(
        task_output
            .stderr
            .contains("No unlocked workspace-data session or saved unlock key is available"),
        "unexpected stderr: {}",
        task_output.stderr
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_json_logout_revoke_warning_is_machine_readable() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    state.lock().expect("state lock").logout_status = StatusCode::BAD_GATEWAY;
    let server = spawn_server(state).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let output = run_cli(
        home.path(),
        &server.base_url,
        &["--json", "auth", "logout"],
        None,
    );
    assert!(output.status.success(), "logout failed: {}", output.stderr);

    let stdout_json = parse_stdout_json(&output.stdout);
    assert_eq!(stdout_json["loggedOut"], true);

    assert_json_warning_contains(&output.stderr, "logout_revoke_failed", "502 Bad Gateway");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_json_logout_keychain_clear_warning_is_machine_readable() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state).await;
    let home = TempDir::new().expect("temp home");
    let keychain_dir = TempDir::new().expect("temp keychain");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let store_output = run_cli_with_test_keychain(
        home.path(),
        &server.base_url,
        keychain_dir.path(),
        &["auth", "keychain", "store", "--password-stdin"],
        Some(&fixture.password),
    );
    assert!(
        store_output.status.success(),
        "keychain store failed: {}",
        store_output.stderr
    );

    replace_stored_test_keychain_secret_with_directory(keychain_dir.path());

    let output = run_cli_with_test_keychain(
        home.path(),
        &server.base_url,
        keychain_dir.path(),
        &["--json", "auth", "logout"],
        None,
    );
    assert!(output.status.success(), "logout failed: {}", output.stderr);

    let stdout_json = parse_stdout_json(&output.stdout);
    assert_eq!(stdout_json["loggedOut"], true);

    assert_json_warning_contains(
        &output.stderr,
        "logout_persisted_bootstrap_clear_failed",
        "failed to clear platform keychain entry",
    );
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_json_logout_warning_and_cleanup_error_share_one_stderr_document() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    state.lock().expect("state lock").logout_status = StatusCode::BAD_GATEWAY;
    let server = spawn_server(state).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let socket_path = home.path().join(".sealtask").join("unlock.sock");
    let (release_fake_daemon, fake_daemon) = spawn_hanging_unlock_daemon(&socket_path);

    let started_at = std::time::Instant::now();
    let output = run_cli(
        home.path(),
        &server.base_url,
        &["--json", "auth", "logout"],
        None,
    );
    let elapsed = started_at.elapsed();
    release_fake_daemon
        .send(())
        .expect("release hanging fake daemon");
    fake_daemon.join().expect("join fake daemon");

    assert!(
        !output.status.success(),
        "logout unexpectedly succeeded: {}",
        output.stdout
    );
    assert!(
        output.stdout.is_empty(),
        "unexpected stdout: {}",
        output.stdout
    );

    let stderr_json = parse_stderr_json(&output.stderr);
    assert_eq!(stderr_json["warnings"][0]["code"], "logout_revoke_failed");
    assert!(
        stderr_json["warnings"][0]["message"]
            .as_str()
            .expect("warning message")
            .contains("502 Bad Gateway"),
        "unexpected stderr: {}",
        output.stderr
    );
    assert_eq!(stderr_json["error"]["code"], "unexpected");
    assert!(
        stderr_json["error"]["message"]
            .as_str()
            .expect("error message")
            .contains("failed to clear unlock daemon session"),
        "unexpected stderr: {}",
        output.stderr
    );
    assert!(
        stderr_json["error"]["message"]
            .as_str()
            .expect("error message")
            .contains("failed to read unlock daemon response"),
        "unexpected stderr: {}",
        output.stderr
    );
    assert!(
        elapsed < std::time::Duration::from_secs(3),
        "logout remained blocked on the unlock daemon for {elapsed:?}"
    );
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_unlock_creates_user_only_socket_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let unlock_output = run_cli(
        home.path(),
        &server.base_url,
        &["auth", "unlock", "--ttl-seconds", "300", "--password-stdin"],
        Some(&fixture.password),
    );
    assert!(
        unlock_output.status.success(),
        "unlock failed: {}",
        unlock_output.stderr
    );

    let socket_path = home.path().join(".sealtask").join("unlock.sock");
    let mode = std::fs::metadata(&socket_path)
        .expect("socket metadata")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o600);

    let _ = run_cli(home.path(), &server.base_url, &["auth", "lock"], None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_status_reports_stored_session_daemon_state_when_api_url_differs() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let unlock_output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "auth",
            "unlock",
            "--ttl-seconds",
            "300",
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        unlock_output.status.success(),
        "unlock failed: {}",
        unlock_output.stderr
    );
    let unlock_json: Value = parse_stdout_json(&unlock_output.stdout);
    assert_eq!(unlock_json["unlocked"], true);

    let status_output = run_cli(
        home.path(),
        "https://sealtask.com",
        &["--json", "auth", "status"],
        None,
    );
    assert!(
        status_output.status.success(),
        "status failed: {}",
        status_output.stderr
    );
    let status_json: Value = parse_stdout_json(&status_output.stdout);
    assert_eq!(status_json["unlockDaemon"]["active"], true);
    assert_eq!(
        status_json["apiUrlMismatch"]["currentApiUrl"],
        "https://sealtask.com"
    );
    assert_eq!(
        status_json["apiUrlMismatch"]["storedApiUrl"],
        server.base_url
    );

    let _ = run_cli(home.path(), &server.base_url, &["auth", "lock"], None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_should_query_unlock_daemon_status_when_credentials_are_missing() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state).await;
    let home = TempDir::new().expect("temp home");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let unlock_output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "auth",
            "unlock",
            "--ttl-seconds",
            "300",
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        unlock_output.status.success(),
        "unlock failed: {}",
        unlock_output.stderr
    );

    std::fs::remove_file(home.path().join(".sealtask/credentials.json"))
        .expect("remove credentials without clearing daemon session");
    let status_output = run_cli(
        home.path(),
        &server.base_url,
        &["--json", "auth", "status"],
        None,
    );
    assert!(
        status_output.status.success(),
        "status failed: {}",
        status_output.stderr
    );
    let status_json: Value = parse_stdout_json(&status_output.stdout);
    assert_eq!(status_json["loggedIn"], false);
    assert_eq!(status_json["unlockDaemon"]["active"], true);

    let _ = run_cli(home.path(), &server.base_url, &["auth", "lock"], None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_non_interactive_login_requires_email() {
    let home = TempDir::new().expect("temp home");

    let output = run_cli(
        home.path(),
        "https://sealtask.com",
        &["--json", "--non-interactive", "auth", "login"],
        None,
    );
    assert!(!output.status.success(), "login unexpectedly succeeded");
    assert!(
        output.stdout.is_empty(),
        "unexpected stdout: {}",
        output.stdout
    );

    assert_json_error_message(
        &output.stderr,
        "--non-interactive auth login requires --email",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_non_interactive_login_requires_password_stdin() {
    assert_json_password_stdin_required(
        &[
            "--json",
            "--non-interactive",
            "auth",
            "login",
            "--email",
            "agent@example.com",
        ],
        "--non-interactive auth login requires --password-stdin",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_login_stdin_sends_whitespace_factor_to_the_mfa_endpoint_unchanged() {
    const EMAIL: &str = "raw-mfa-cli@example.test";
    const PASSWORD: &str = "process-password";
    const RAW_FACTOR: &str = " \t ";
    const CHALLENGE_TOKEN: &str = "process-level-mfa-challenge";

    let observed_code = Arc::new(Mutex::new(None));
    let server =
        spawn_raw_mfa_login_server(EMAIL, PASSWORD, CHALLENGE_TOKEN, observed_code.clone()).await;
    let home = TempDir::new().expect("temp home");
    let stdin = format!("  {PASSWORD}  \r\n{RAW_FACTOR}\r\n");

    let output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "auth",
            "login",
            "--email",
            EMAIL,
            "--password-stdin",
        ],
        Some(&stdin),
    );

    assert!(!output.status.success(), "denied MFA login must fail");
    assert!(
        output.stdout.is_empty(),
        "unexpected stdout: {}",
        output.stdout
    );
    let error = parse_stderr_json(&output.stderr);
    assert_eq!(error["error"]["code"], "validation");
    assert_eq!(
        error["error"]["message"],
        "mock endpoint rejected the supplied MFA factor"
    );
    assert_ne!(error["error"]["code"], "mfa_input_required");
    assert!(!output.stderr.contains(PASSWORD));
    assert!(!output.stderr.contains(CHALLENGE_TOKEN));
    assert_eq!(
        observed_code.lock().expect("observed code lock").as_deref(),
        Some(RAW_FACTOR)
    );
    assert!(
        !home.path().join(".sealtask/credentials.json").exists(),
        "a denied MFA attempt must not persist credentials"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_login_stdin_persists_only_final_no_mfa_credentials() {
    const EMAIL: &str = "process-no-mfa@example.test";
    const PASSWORD: &str = "process-no-mfa-password";
    const CHALLENGE_TOKEN: &str = "unused-process-no-mfa-challenge";

    let observed_code = Arc::new(Mutex::new(None));
    let server = spawn_raw_login_server(
        EMAIL,
        PASSWORD,
        CHALLENGE_TOKEN,
        observed_code.clone(),
        RawLoginFinishOutcome::Authenticated,
        RawMfaVerifyOutcome::Reject,
    )
    .await;
    let home = TempDir::new().expect("temp home");
    let stdin = format!("{PASSWORD}\n");

    let output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "auth",
            "login",
            "--email",
            EMAIL,
            "--password-stdin",
        ],
        Some(&stdin),
    );

    assert!(output.status.success(), "login failed: {}", output.stderr);
    assert!(
        output.stderr.is_empty(),
        "unexpected stderr: {}",
        output.stderr
    );
    assert!(observed_code.lock().expect("observed code lock").is_none());
    assert_final_process_login_credentials(home.path(), PASSWORD, CHALLENGE_TOKEN, None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_should_replace_active_credentials_when_login_requests_another_account() {
    const EMAIL: &str = "process-login@example.test";
    const PASSWORD: &str = "process-account-switch-password";
    const CHALLENGE_TOKEN: &str = "unused-account-switch-challenge";

    let observed_logout_refresh_token = Arc::new(Mutex::new(None));
    let server = spawn_raw_login_server_with_logout_observer(
        EMAIL,
        PASSWORD,
        CHALLENGE_TOKEN,
        Arc::new(Mutex::new(None)),
        RawLoginFinishOutcome::Authenticated,
        RawMfaVerifyOutcome::Reject,
        observed_logout_refresh_token.clone(),
    )
    .await;
    let fixture = TestFixture::new();
    let home = TempDir::new().expect("temp home");
    let keychain_dir = TempDir::new().expect("temp keychain");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let store_output = run_cli_with_test_keychain(
        home.path(),
        &server.base_url,
        keychain_dir.path(),
        &["--json", "auth", "keychain", "store", "--password-stdin"],
        Some(&fixture.password),
    );
    assert!(
        store_output.status.success(),
        "keychain store failed: {}",
        store_output.stderr
    );
    assert_eq!(
        std::fs::read_dir(keychain_dir.path())
            .expect("read test keychain directory")
            .count(),
        1,
        "old account should have one persisted bootstrap secret"
    );

    let unlock_output = run_cli_with_test_keychain(
        home.path(),
        &server.base_url,
        keychain_dir.path(),
        &[
            "--json",
            "auth",
            "unlock",
            "--ttl-seconds",
            "300",
            "--password-stdin",
        ],
        Some(&fixture.password),
    );
    assert!(
        unlock_output.status.success(),
        "unlock failed: {}",
        unlock_output.stderr
    );

    let output = run_cli_with_test_keychain(
        home.path(),
        &server.base_url,
        keychain_dir.path(),
        &[
            "--json",
            "auth",
            "login",
            "--email",
            EMAIL,
            "--password-stdin",
        ],
        Some(&format!("{PASSWORD}\n")),
    );

    assert!(
        output.status.success(),
        "account-switch login failed: {}",
        output.stderr
    );
    let result = parse_stdout_json(&output.stdout);
    assert_eq!(result["alreadyLoggedIn"], false);
    assert_eq!(result["email"], EMAIL);
    assert!(
        output.stderr.is_empty(),
        "successful previous-session revocation should not warn: {}",
        output.stderr
    );

    let credentials: Credentials = serde_json::from_slice(
        &std::fs::read(home.path().join(".sealtask/credentials.json"))
            .expect("read switched credentials"),
    )
    .expect("parse switched credentials");
    assert_eq!(credentials.email, EMAIL);
    assert_eq!(credentials.access_token, "process-final-access-token");
    assert_eq!(
        observed_logout_refresh_token
            .lock()
            .expect("logout observer lock")
            .as_deref(),
        Some(fixture.refresh_token.as_str())
    );
    assert_eq!(
        std::fs::read_dir(keychain_dir.path())
            .expect("read cleared test keychain directory")
            .count(),
        0,
        "account switch should clear the old account keychain entry"
    );

    std::fs::remove_file(home.path().join(".sealtask/credentials.json"))
        .expect("remove switched credentials before checking daemon state");
    let status_output = run_cli_with_test_keychain(
        home.path(),
        &server.base_url,
        keychain_dir.path(),
        &["--json", "auth", "status"],
        None,
    );
    assert!(
        status_output.status.success(),
        "status failed: {}",
        status_output.stderr
    );
    let status = parse_stdout_json(&status_output.stdout);
    assert_eq!(status["loggedIn"], false);
    assert_eq!(status["unlockDaemon"]["active"], false);

    let lock_output = run_cli_with_test_keychain(
        home.path(),
        &server.base_url,
        keychain_dir.path(),
        &["auth", "lock"],
        None,
    );
    assert!(
        lock_output.status.success(),
        "daemon cleanup failed: {}",
        lock_output.stderr
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_login_stdin_completes_totp_and_backup_code_before_persisting() {
    const EMAIL: &str = "process-mfa-success@example.test";
    const PASSWORD: &str = "process-mfa-success-password";
    const CHALLENGE_TOKEN: &str = "process-mfa-success-challenge";

    for code in ["012345", "ST2-00112233-44556677-8899AABB-CCDDEEFF"] {
        let observed_code = Arc::new(Mutex::new(None));
        let server = spawn_raw_login_server(
            EMAIL,
            PASSWORD,
            CHALLENGE_TOKEN,
            observed_code.clone(),
            RawLoginFinishOutcome::MfaRequired,
            RawMfaVerifyOutcome::Authenticate {
                expected_code: code.to_string(),
            },
        )
        .await;
        let home = TempDir::new().expect("temp home");
        let stdin = format!("{PASSWORD}\n{code}\n");

        let output = run_cli(
            home.path(),
            &server.base_url,
            &[
                "--json",
                "auth",
                "login",
                "--email",
                EMAIL,
                "--password-stdin",
            ],
            Some(&stdin),
        );

        assert!(
            output.status.success(),
            "MFA login for {code} failed: {}",
            output.stderr
        );
        assert!(
            output.stderr.is_empty(),
            "unexpected stderr: {}",
            output.stderr
        );
        assert_eq!(
            observed_code.lock().expect("observed code lock").as_deref(),
            Some(code)
        );
        assert_final_process_login_credentials(home.path(), PASSWORD, CHALLENGE_TOKEN, Some(code));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_login_stdin_missing_factor_fails_without_prompting_or_persisting() {
    const EMAIL: &str = "process-mfa-missing@example.test";
    const PASSWORD: &str = "process-mfa-missing-password";
    const CHALLENGE_TOKEN: &str = "process-mfa-missing-challenge";

    let observed_code = Arc::new(Mutex::new(None));
    let server = spawn_raw_login_server(
        EMAIL,
        PASSWORD,
        CHALLENGE_TOKEN,
        observed_code.clone(),
        RawLoginFinishOutcome::MfaRequired,
        RawMfaVerifyOutcome::Reject,
    )
    .await;
    let home = TempDir::new().expect("temp home");
    let output = run_cli(
        home.path(),
        &server.base_url,
        &[
            "--json",
            "auth",
            "login",
            "--email",
            EMAIL,
            "--password-stdin",
        ],
        Some(&format!("{PASSWORD}\n")),
    );

    assert!(
        !output.status.success(),
        "missing MFA input unexpectedly succeeded"
    );
    assert!(
        output.stdout.is_empty(),
        "unexpected stdout: {}",
        output.stdout
    );
    let error = parse_stderr_json(&output.stderr);
    assert_eq!(error["error"]["code"], "mfa_input_required");
    assert!(!output.stderr.contains(PASSWORD));
    assert!(!output.stderr.contains(CHALLENGE_TOKEN));
    assert!(observed_code.lock().expect("observed code lock").is_none());
    assert!(
        !home.path().join(".sealtask/credentials.json").exists(),
        "an incomplete MFA login must not persist credentials"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_login_process_maps_upgrade_and_terminal_or_retryable_mfa_without_secret_persistence() {
    const EMAIL: &str = "process-mfa-errors@example.test";
    const PASSWORD: &str = "process-mfa-errors-password";
    const CODE: &str = "012345";
    const CHALLENGE_TOKEN: &str = "process-mfa-errors-challenge";

    let upgrade_server = spawn_raw_login_server(
        EMAIL,
        PASSWORD,
        CHALLENGE_TOKEN,
        Arc::new(Mutex::new(None)),
        RawLoginFinishOutcome::UpgradeRequired,
        RawMfaVerifyOutcome::Reject,
    )
    .await;
    let upgrade_home = TempDir::new().expect("upgrade home");
    let upgrade = run_cli(
        upgrade_home.path(),
        &upgrade_server.base_url,
        &[
            "--json",
            "auth",
            "login",
            "--email",
            EMAIL,
            "--password-stdin",
        ],
        Some(&format!("{PASSWORD}\n")),
    );
    assert!(!upgrade.status.success());
    assert_eq!(
        parse_stderr_json(&upgrade.stderr)["error"]["code"],
        "validation"
    );
    assert!(upgrade.stderr.contains("must be upgraded"));
    assert!(!upgrade.stderr.contains(PASSWORD));
    assert!(!upgrade.stderr.contains(CHALLENGE_TOKEN));
    assert!(
        !upgrade_home
            .path()
            .join(".sealtask/credentials.json")
            .exists()
    );

    for outcome in [
        RawMfaVerifyOutcome::Expired,
        RawMfaVerifyOutcome::RateLimited,
        RawMfaVerifyOutcome::ServiceUnavailable,
        RawMfaVerifyOutcome::TotpLocked,
    ] {
        let observed_code = Arc::new(Mutex::new(None));
        let server = spawn_raw_login_server(
            EMAIL,
            PASSWORD,
            CHALLENGE_TOKEN,
            observed_code.clone(),
            RawLoginFinishOutcome::MfaRequired,
            outcome,
        )
        .await;
        let home = TempDir::new().expect("MFA error home");
        let output = run_cli(
            home.path(),
            &server.base_url,
            &[
                "--json",
                "auth",
                "login",
                "--email",
                EMAIL,
                "--password-stdin",
            ],
            Some(&format!("{PASSWORD}\n{CODE}\n")),
        );

        assert!(!output.status.success(), "MFA error unexpectedly succeeded");
        assert!(
            output.stdout.is_empty(),
            "unexpected stdout: {}",
            output.stdout
        );
        let error = parse_stderr_json(&output.stderr);
        assert_eq!(error["error"]["code"], "validation");
        assert!(!output.stderr.contains(PASSWORD));
        assert!(!output.stderr.contains(CODE));
        assert!(!output.stderr.contains(CHALLENGE_TOKEN));
        assert_eq!(
            observed_code.lock().expect("observed code lock").as_deref(),
            Some(CODE)
        );
        assert!(
            !home.path().join(".sealtask/credentials.json").exists(),
            "a failed MFA result must not persist credentials"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_non_interactive_unlock_requires_password_stdin() {
    assert_json_password_stdin_required(
        &["--json", "--non-interactive", "auth", "unlock"],
        "--non-interactive auth unlock requires --password-stdin",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_non_interactive_keychain_store_requires_password_stdin() {
    assert_json_password_stdin_required(
        &["--json", "--non-interactive", "auth", "keychain", "store"],
        "--non-interactive auth keychain store requires --password-stdin",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_json_unknown_argument_errors_are_machine_readable() {
    let home = TempDir::new().expect("temp home");

    let output = run_cli(
        home.path(),
        "https://sealtask.com",
        &["--json", "--bogus"],
        None,
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(
        output.stdout.is_empty(),
        "unexpected stdout: {}",
        output.stdout
    );

    assert_json_error_contains(&output.stderr, "--bogus");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_json_invalid_value_errors_are_machine_readable() {
    let home = TempDir::new().expect("temp home");

    let output = run_cli(
        home.path(),
        "https://sealtask.com",
        &["--json", "auth", "unlock", "--ttl-seconds", "nope"],
        None,
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(
        output.stdout.is_empty(),
        "unexpected stdout: {}",
        output.stdout
    );

    assert_json_error_contains(&output.stderr, "--ttl-seconds");
    assert_json_error_contains(&output.stderr, "nope");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_should_reject_zero_and_overflowing_unlock_ttls_before_unlocking() {
    for ttl in ["0", "18446744073709551615"] {
        let home = TempDir::new().expect("temp home");
        let output = run_cli(
            home.path(),
            "https://sealtask.com",
            &[
                "--json",
                "auth",
                "unlock",
                "--ttl-seconds",
                ttl,
                "--password-stdin",
            ],
            Some("unused-password"),
        );

        assert!(
            !output.status.success(),
            "invalid TTL unexpectedly succeeded"
        );
        assert!(
            output.stdout.is_empty(),
            "unexpected stdout: {}",
            output.stdout
        );
        let error = parse_stderr_json(&output.stderr);
        assert_eq!(error["error"]["code"], "validation");
        assert!(
            error["error"]["message"]
                .as_str()
                .expect("TTL error message")
                .contains("unlock TTL")
        );
        assert!(!output.stderr.contains("unused-password"));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_decrypted_commands_fail_non_interactively_without_unlock_or_keychain() {
    let fixture = TestFixture::new();
    let state = Arc::new(Mutex::new(TestState::new(fixture.clone())));
    let server = spawn_server(state).await;
    let home = TempDir::new().expect("temp home");
    let keychain_dir = TempDir::new().expect("temp keychain");
    seed_credentials(home.path(), &fixture, &server.base_url);

    let task_output = run_cli_with_test_keychain(
        home.path(),
        &server.base_url,
        keychain_dir.path(),
        &[
            "--json",
            "tasks",
            "get",
            "--work-list-id",
            &fixture.work_list_id.to_string(),
            "--task-id",
            &fixture.task_id.to_string(),
        ],
        None,
    );
    assert!(
        !task_output.status.success(),
        "task get unexpectedly succeeded without any local unlock source"
    );
    assert!(
        task_output
            .stderr
            .contains("No unlocked workspace-data session or saved unlock key is available"),
        "unexpected stderr: {}",
        task_output.stderr
    );
    assert_json_error_contains(
        &task_output.stderr,
        "No unlocked workspace-data session or saved unlock key is available",
    );
}

struct TestServer {
    base_url: String,
    _task: tokio::task::JoinHandle<()>,
}

struct DoctorHealthServer {
    base_url: String,
    observations: Arc<Mutex<Vec<DoctorHealthRequest>>>,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for DoctorHealthServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[derive(Clone)]
struct DoctorHealthState {
    observations: Arc<Mutex<Vec<DoctorHealthRequest>>>,
    response_delay: Option<std::time::Duration>,
}

struct DoctorHealthRequest {
    user_agent: Option<String>,
    request_id: Option<String>,
}

struct RefreshLogoutRaceServer {
    base_url: String,
    state: Arc<Mutex<RefreshLogoutRaceInner>>,
    refresh_committed: Arc<Notify>,
    release_refresh_response: Arc<Notify>,
    _task: tokio::task::JoinHandle<()>,
}

#[derive(Clone)]
struct RefreshLogoutRaceAppState {
    state: Arc<Mutex<RefreshLogoutRaceInner>>,
    refresh_committed: Arc<Notify>,
    release_refresh_response: Arc<Notify>,
}

struct RefreshLogoutRaceInner {
    initial_refresh_token: String,
    refresh_requests: usize,
    logout_requests: usize,
    work_list_requests: usize,
    revoked: bool,
}

#[derive(Clone)]
struct RawMfaLoginState {
    email: String,
    challenge_token: String,
    setup: Vec<u8>,
    password_file: Vec<u8>,
    observed_code: Arc<Mutex<Option<String>>>,
    observed_logout_refresh_token: Arc<Mutex<Option<String>>>,
    login_finish: RawLoginFinishOutcome,
    mfa_verify: RawMfaVerifyOutcome,
}

#[derive(Clone, Copy)]
enum RawLoginFinishOutcome {
    Authenticated,
    MfaRequired,
    UpgradeRequired,
}

#[derive(Clone)]
enum RawMfaVerifyOutcome {
    Authenticate { expected_code: String },
    Expired,
    Reject,
    RateLimited,
    ServiceUnavailable,
    TotpLocked,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawMfaLoginStartRequest {
    email: String,
    client_login_state: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawMfaVerifyRequest {
    challenge_token: String,
    code: String,
}

async fn spawn_raw_mfa_login_server(
    email: &str,
    password: &str,
    challenge_token: &str,
    observed_code: Arc<Mutex<Option<String>>>,
) -> TestServer {
    spawn_raw_login_server(
        email,
        password,
        challenge_token,
        observed_code,
        RawLoginFinishOutcome::MfaRequired,
        RawMfaVerifyOutcome::Reject,
    )
    .await
}

async fn spawn_raw_login_server(
    email: &str,
    password: &str,
    challenge_token: &str,
    observed_code: Arc<Mutex<Option<String>>>,
    login_finish: RawLoginFinishOutcome,
    mfa_verify: RawMfaVerifyOutcome,
) -> TestServer {
    spawn_raw_login_server_with_logout_observer(
        email,
        password,
        challenge_token,
        observed_code,
        login_finish,
        mfa_verify,
        Arc::new(Mutex::new(None)),
    )
    .await
}

async fn spawn_raw_login_server_with_logout_observer(
    email: &str,
    password: &str,
    challenge_token: &str,
    observed_code: Arc<Mutex<Option<String>>>,
    login_finish: RawLoginFinishOutcome,
    mfa_verify: RawMfaVerifyOutcome,
    observed_logout_refresh_token: Arc<Mutex<Option<String>>>,
) -> TestServer {
    const SERVER_ID: &[u8] = b"worklist.api";

    let mut rng = OsRng;
    let setup = ServerSetup::<ClientCipherSuite>::new(&mut rng);
    let registration =
        ClientRegistration::<ClientCipherSuite>::start(&mut rng, password.as_bytes())
            .expect("start mock OPAQUE registration");
    let registration_response = ServerRegistration::<ClientCipherSuite>::start(
        &setup,
        registration.message,
        email.as_bytes(),
    )
    .expect("start mock OPAQUE server registration");
    let identifiers = Identifiers {
        client: Some(email.as_bytes()),
        server: Some(SERVER_ID),
    };
    let registration_upload = registration
        .state
        .finish(
            &mut rng,
            password.as_bytes(),
            RegistrationResponse::<ClientCipherSuite>::deserialize(
                &registration_response.message.serialize(),
            )
            .expect("deserialize mock OPAQUE registration response"),
            ClientRegistrationFinishParameters::new(identifiers, None),
        )
        .expect("finish mock OPAQUE client registration");
    let password_file =
        ServerRegistration::<ClientCipherSuite>::finish(registration_upload.message);

    let state = RawMfaLoginState {
        email: email.to_string(),
        challenge_token: challenge_token.to_string(),
        setup: setup.serialize().to_vec(),
        password_file: password_file.serialize().to_vec(),
        observed_code,
        observed_logout_refresh_token,
        login_finish,
        mfa_verify,
    };
    let app = Router::new()
        .route("/auth/opaque/login/start", post(raw_mfa_login_start))
        .route("/auth/opaque/login/finish", post(raw_mfa_login_finish))
        .route("/auth/mfa/login/verify", post(raw_mfa_verify))
        .route("/auth/logout", post(raw_login_logout))
        .with_state(state);

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind raw MFA login server");
    let addr = listener.local_addr().expect("raw MFA login server address");
    let task = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("serve raw MFA login server");
    });

    TestServer {
        base_url: format!("http://{addr}"),
        _task: task,
    }
}

async fn raw_login_logout(
    State(state): State<RawMfaLoginState>,
    Json(request): Json<RefreshRequestBody>,
) -> StatusCode {
    state
        .observed_logout_refresh_token
        .lock()
        .expect("logout observer lock")
        .replace(request.refresh_token);
    StatusCode::OK
}

async fn raw_mfa_login_finish(State(state): State<RawMfaLoginState>) -> (StatusCode, Json<Value>) {
    match state.login_finish {
        RawLoginFinishOutcome::Authenticated => (StatusCode::OK, Json(raw_successful_auth_body())),
        RawLoginFinishOutcome::MfaRequired => (
            StatusCode::OK,
            Json(json!({
                "status": "second_factor_required",
                "challengeToken": state.challenge_token,
                "methods": ["totp", "backup_code"],
                "expiresIn": 300,
                "attemptsRemaining": 8,
                "requiresLegacyPassword": false
            })),
        ),
        RawLoginFinishOutcome::UpgradeRequired => (
            StatusCode::CONFLICT,
            Json(json!({
                "error": "mfa_client_upgrade_required",
                "message": "upgrade the client"
            })),
        ),
    }
}

async fn raw_mfa_login_start(
    State(state): State<RawMfaLoginState>,
    Json(request): Json<RawMfaLoginStartRequest>,
) -> (StatusCode, Json<Value>) {
    const SERVER_ID: &[u8] = b"worklist.api";

    assert_eq!(request.email, state.email);
    let client_message = STANDARD_NO_PAD
        .decode(request.client_login_state)
        .expect("decode mock OPAQUE client login state");
    let credential_request = CredentialRequest::<ClientCipherSuite>::deserialize(&client_message)
        .expect("deserialize mock OPAQUE client login state");
    let setup = ServerSetup::<ClientCipherSuite>::deserialize(&state.setup)
        .expect("deserialize mock OPAQUE setup");
    let password_file = ServerRegistration::<ClientCipherSuite>::deserialize(&state.password_file)
        .expect("deserialize mock OPAQUE password file");
    let identifiers = Identifiers {
        client: Some(state.email.as_bytes()),
        server: Some(SERVER_ID),
    };
    let mut rng = OsRng;
    let login = ServerLogin::<ClientCipherSuite>::start(
        &mut rng,
        &setup,
        Some(password_file),
        credential_request,
        state.email.as_bytes(),
        ServerLoginParameters {
            context: None,
            identifiers,
        },
    )
    .expect("start mock OPAQUE login");

    (
        StatusCode::OK,
        Json(json!({
            "serverLoginState": STANDARD_NO_PAD.encode(login.message.serialize()),
            "sessionToken": "mock-opaque-session",
            "expiresIn": 300
        })),
    )
}

async fn raw_mfa_verify(
    State(state): State<RawMfaLoginState>,
    Json(request): Json<RawMfaVerifyRequest>,
) -> (StatusCode, Json<Value>) {
    assert_eq!(request.challenge_token, state.challenge_token);
    state
        .observed_code
        .lock()
        .expect("observed code lock")
        .replace(request.code.clone());
    match state.mfa_verify {
        RawMfaVerifyOutcome::Authenticate { expected_code } if request.code == expected_code => {
            (StatusCode::OK, Json(raw_successful_auth_body()))
        }
        RawMfaVerifyOutcome::Expired => (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "error": "mfa_challenge_invalid_or_expired",
                "message": format!(
                    "expired {} for {}",
                    request.code, state.challenge_token
                )
            })),
        ),
        RawMfaVerifyOutcome::RateLimited => (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "error": "rate_limited",
                "message": format!(
                    "retry {} for {}",
                    request.code, state.challenge_token
                )
            })),
        ),
        RawMfaVerifyOutcome::ServiceUnavailable => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "error": "mfa_service_unavailable",
                "message": format!(
                    "unavailable {} for {}",
                    request.code, state.challenge_token
                )
            })),
        ),
        RawMfaVerifyOutcome::TotpLocked => (
            StatusCode::CONFLICT,
            Json(json!({
                "error": "mfa_totp_locked",
                "message": format!(
                    "locked {} for {}",
                    request.code, state.challenge_token
                )
            })),
        ),
        RawMfaVerifyOutcome::Authenticate { .. } | RawMfaVerifyOutcome::Reject => (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "error": "invalid_mfa_code",
                "message": "mock endpoint rejected the supplied MFA factor",
                "attemptsRemaining": 7,
                "expiresIn": 240
            })),
        ),
    }
}

fn raw_successful_auth_body() -> Value {
    json!({
        "accessToken": "process-final-access-token",
        "refreshToken": "process-final-refresh-token",
        "expiresIn": 900,
        "refreshExpiresIn": 2_592_000,
        "tokenType": "Bearer",
        "user": {
            "id": "01900000-0000-7000-8000-000000000001",
            "email": "process-login@example.test",
            "name": "Process Login",
            "timezone": "UTC",
            "avatarColor": "blue",
            "themePreference": "system",
            "emailVerified": true
        },
        "dataKeyCiphertext": "process-encrypted-data-key"
    })
}

fn assert_final_process_login_credentials(
    home: &FsPath,
    password: &str,
    challenge_token: &str,
    code: Option<&str>,
) {
    let bytes = std::fs::read(home.join(".sealtask/credentials.json"))
        .expect("read final process credentials");
    let stored = String::from_utf8(bytes.clone()).expect("credentials UTF-8");
    let credentials: Credentials =
        serde_json::from_slice(&bytes).expect("parse final process credentials");

    assert_eq!(credentials.access_token, "process-final-access-token");
    assert_eq!(credentials.refresh_token, "process-final-refresh-token");
    for secret in [Some(password), Some(challenge_token), code]
        .into_iter()
        .flatten()
    {
        assert!(!stored.contains(secret), "credentials leaked login input");
    }
}

#[derive(Clone)]
struct TestAttachmentFixture {
    id: Uuid,
    file_name: String,
    content_type: String,
    plaintext_bytes: Vec<u8>,
    ciphertext_bytes: Vec<u8>,
    blob_key: Vec<u8>,
}

const FIXTURE_EMAIL: &str = "fixture@example.test";

#[derive(Clone, Copy)]
enum DataKeyWrapperFixture {
    LegacyPasswordV1,
    OpaqueExportKeyV2,
}

struct OpaqueAccountFixture {
    setup: Vec<u8>,
    password_file: Vec<u8>,
    export_key: [u8; OPAQUE_EXPORT_KEY_BYTES],
}

#[derive(Clone)]
struct TestFixture {
    password: String,
    access_token: String,
    refresh_token: String,
    work_list_id: Uuid,
    first_section_id: Uuid,
    done_section_id: Uuid,
    task_id: Uuid,
    comment_id: Uuid,
    membership_id: Uuid,
    owner_user_id: Uuid,
    workspace_id: Uuid,
    mentioned_user_id: Uuid,
    data_key: SymmetricKey,
    list_key: SymmetricKey,
    binding_key: SymmetricKey,
    opaque_setup: Vec<u8>,
    opaque_password_file: Vec<u8>,
    opaque_export_key: [u8; OPAQUE_EXPORT_KEY_BYTES],
    data_key_ciphertext: String,
    work_list_key_ciphertext: String,
    work_list_payload_ciphertext: String,
    task_title_ciphertext: String,
    comment_body_ciphertext: String,
    existing_task_body: TaskPayloadBody,
    existing_comment_body: CommentPayloadBody,
    text_attachment: TestAttachmentFixture,
    docx_attachment: TestAttachmentFixture,
    binary_attachment: TestAttachmentFixture,
    hostile_attachment: TestAttachmentFixture,
}

struct TestState {
    fixture: TestFixture,
    current_access_token: String,
    current_refresh_token: String,
    logout_status: StatusCode,
    refresh_request_count: usize,
    opaque_export_key_start_count: usize,
    created_task_body: Option<TaskPayloadBody>,
    created_task_request: Option<Value>,
    updated_task_body: Option<TaskPayloadBody>,
    updated_task_request: Option<Value>,
    current_task_body: TaskPayloadBody,
    moved_task_body: Option<MoveTaskRequestBody>,
    task_section_id: Option<Uuid>,
    task_is_completed: bool,
    task_completed_at: Option<DateTime<Utc>>,
    task_archived_at: Option<DateTime<Utc>>,
    task_updated_at: DateTime<Utc>,
    reject_next_task_update_as_conflict: bool,
    tasks_empty: bool,
    my_tasks_count: usize,
    my_tasks_queries: Vec<(i64, bool)>,
    list_task_include_archived: Vec<bool>,
    single_section: bool,
    work_list_archived_at: Option<DateTime<Utc>>,
    archive_work_list_count: usize,
    unarchive_work_list_count: usize,
    list_work_list_include_archived: Vec<bool>,
    archive_task_count: usize,
    unarchive_task_count: usize,
    created_comment_body: Option<CommentPayloadBody>,
    updated_comment_body: Option<CommentPayloadBody>,
    list_comments_count: usize,
    deleted_comment_audit_patch: Option<AuditPatchRequest>,
    deleted_comment_id: Option<Uuid>,
    deleted_task_audit_patch: Option<AuditPatchRequest>,
    deleted_task_id: Option<Uuid>,
    invalid_work_list_payload: bool,
    invalid_task_payload: bool,
    invalid_comment_payload: bool,
    invalid_task_attachment_metadata: bool,
    invalid_comment_attachment_metadata: bool,
    attachment_size_mismatch: bool,
    attachment_download_requests: usize,
    attachment_upload_expected_bytes: HashMap<Uuid, u64>,
    attachment_uploads: HashMap<Uuid, Vec<u8>>,
    completed_attachment_ids: HashSet<Uuid>,
    deleted_attachment_ids: Vec<Uuid>,
    notes: HashMap<Uuid, StoredNote>,
    note_create_operations: HashMap<String, (String, Uuid)>,
    d6_watch_enabled: bool,
    d6_watch_task_reads: usize,
    d6_watch_call_order: Vec<&'static str>,
    d6_activity_reads: usize,
    d7_task_create_committed: Option<Arc<Notify>>,
    d7_task_create_release_response: Option<Arc<Notify>>,
    d7_task_update_committed: Option<Arc<Notify>>,
    d7_task_update_release_response: Option<Arc<Notify>>,
    base_url: Option<String>,
}

#[derive(Clone)]
struct StoredNote {
    id: Uuid,
    title_ciphertext: String,
    payload_ciphertext: String,
    is_private: bool,
    note_key_ciphertext: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TestState {
    fn new(fixture: TestFixture) -> Self {
        let current_task_body = fixture.existing_task_body.clone();
        Self {
            current_access_token: fixture.access_token.clone(),
            current_refresh_token: fixture.refresh_token.clone(),
            logout_status: StatusCode::OK,
            refresh_request_count: 0,
            opaque_export_key_start_count: 0,
            fixture,
            created_task_body: None,
            created_task_request: None,
            updated_task_body: None,
            updated_task_request: None,
            current_task_body,
            moved_task_body: None,
            task_section_id: None,
            task_is_completed: false,
            task_completed_at: None,
            task_archived_at: None,
            task_updated_at: Utc::now(),
            reject_next_task_update_as_conflict: false,
            tasks_empty: false,
            my_tasks_count: 1,
            my_tasks_queries: Vec::new(),
            list_task_include_archived: Vec::new(),
            single_section: false,
            work_list_archived_at: None,
            archive_work_list_count: 0,
            unarchive_work_list_count: 0,
            list_work_list_include_archived: Vec::new(),
            archive_task_count: 0,
            unarchive_task_count: 0,
            created_comment_body: None,
            updated_comment_body: None,
            list_comments_count: 0,
            deleted_comment_audit_patch: None,
            deleted_comment_id: None,
            deleted_task_audit_patch: None,
            deleted_task_id: None,
            invalid_work_list_payload: false,
            invalid_task_payload: false,
            invalid_comment_payload: false,
            invalid_task_attachment_metadata: false,
            invalid_comment_attachment_metadata: false,
            attachment_size_mismatch: false,
            attachment_download_requests: 0,
            attachment_upload_expected_bytes: HashMap::new(),
            attachment_uploads: HashMap::new(),
            completed_attachment_ids: HashSet::new(),
            deleted_attachment_ids: Vec::new(),
            notes: HashMap::new(),
            note_create_operations: HashMap::new(),
            d6_watch_enabled: false,
            d6_watch_task_reads: 0,
            d6_watch_call_order: Vec::new(),
            d6_activity_reads: 0,
            d7_task_create_committed: None,
            d7_task_create_release_response: None,
            d7_task_update_committed: None,
            d7_task_update_release_response: None,
            base_url: None,
        }
    }
}

impl TestFixture {
    fn new() -> Self {
        Self::new_with_data_key_wrapper(DataKeyWrapperFixture::LegacyPasswordV1)
    }

    fn new_v2() -> Self {
        Self::new_with_data_key_wrapper(DataKeyWrapperFixture::OpaqueExportKeyV2)
    }

    fn new_with_data_key_wrapper(wrapper: DataKeyWrapperFixture) -> Self {
        let password = "correct horse battery staple".to_string();
        let opaque_account = make_opaque_account_fixture(FIXTURE_EMAIL, &password);
        let data_key = SymmetricKey::new([0x11; 32]);
        let list_key = SymmetricKey::new([0x22; 32]);
        let binding_key = derive_payload_binding_key(&list_key).expect("binding key");
        let salt = [0x33; 32];

        let work_list_id = Uuid::now_v7();
        let first_section_id = Uuid::now_v7();
        let done_section_id = Uuid::now_v7();
        let task_id = Uuid::now_v7();
        let comment_id = Uuid::now_v7();
        let membership_id = Uuid::now_v7();
        let owner_user_id = Uuid::now_v7();
        let workspace_id = Uuid::now_v7();
        let mentioned_user_id = Uuid::now_v7();

        let data_key_ciphertext = match wrapper {
            DataKeyWrapperFixture::LegacyPasswordV1 => {
                encode_data_key_ciphertext(&password, &salt, &data_key)
                    .expect("legacy data key ciphertext")
            }
            DataKeyWrapperFixture::OpaqueExportKeyV2 => {
                encode_opaque_data_key_ciphertext(&opaque_account.export_key, &data_key)
                    .expect("OPAQUE data key ciphertext")
            }
        };
        let work_list_key_ciphertext =
            encode_membership_key_ciphertext(&data_key, &list_key).expect("membership key");
        let work_list_payload_ciphertext =
            encode_work_list_payload_ciphertext(&list_key, first_section_id, done_section_id)
                .expect("work list payload");
        let text_attachment = make_attachment_fixture(
            &list_key,
            Uuid::now_v7(),
            "notes.md",
            "text/markdown",
            b"# Heading\n\nAttachment body\n".to_vec(),
            [0x44; 32],
        );
        let docx_attachment = make_attachment_fixture(
            &list_key,
            Uuid::now_v7(),
            "spec.docx",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            docx_fixture_bytes(),
            [0x45; 32],
        );
        let binary_attachment = make_attachment_fixture(
            &list_key,
            Uuid::now_v7(),
            "spec.pdf",
            "application/pdf",
            b"%PDF-binary".to_vec(),
            [0x55; 32],
        );
        let hostile_attachment = make_attachment_fixture(
            &list_key,
            Uuid::now_v7(),
            "../../unsafe.txt",
            "text/plain",
            b"unsafe but readable\n".to_vec(),
            [0x66; 32],
        );

        let existing_task_body = TaskPayloadBody {
            title: "Existing task".to_string(),
            rich_text: plaintext_rich_text("Existing task body"),
            checklist: Some(vec![sealtask_client_crypto::ChecklistItemPayload {
                id: Uuid::now_v7().to_string(),
                title: "Keep checklist".to_string(),
                is_done: false,
                completed_at: None,
                assignee_user_ids: Some(vec![mentioned_user_id.to_string()]),
            }]),
            attachments: Some(vec![
                attachment_payload_value(&text_attachment),
                attachment_payload_value(&docx_attachment),
                attachment_payload_value(&binary_attachment),
                attachment_payload_value(&hostile_attachment),
            ]),
            references: Some(vec![json_value_to_flexible(
                json!({"label": "ref", "uri": "https://example.test"}),
            )]),
            mentions: Some(vec![mentioned_user_id.to_string()]),
            client_meta: Some(FlexibleValue::Map(vec![
                (
                    FlexibleValue::Text("source".to_string()),
                    FlexibleValue::Text("fixture".to_string()),
                ),
                (
                    FlexibleValue::Text("blob".to_string()),
                    FlexibleValue::Bytes(vec![1, 2, 3, 4]),
                ),
            ])),
            recurrence_state: Some(json_value_to_flexible(json!({
                "template_id": Uuid::now_v7().to_string()
            }))),
        };
        let task_title_ciphertext = seal_text_value("Existing task").expect("task title").base64;

        let existing_comment_body = CommentPayloadBody {
            content: plaintext_rich_text("Existing comment").expect("comment rich text"),
            mentions: Some(vec![mentioned_user_id.to_string()]),
            attachments: Some(vec![attachment_payload_value(&text_attachment)]),
            client_meta: Some(FlexibleValue::Map(vec![
                (
                    FlexibleValue::Text("source".to_string()),
                    FlexibleValue::Text("fixture".to_string()),
                ),
                (
                    FlexibleValue::Text("blob".to_string()),
                    FlexibleValue::Bytes(vec![9, 8, 7]),
                ),
            ])),
        };
        let comment_body_ciphertext = encrypt_comment_payload(
            &build_comment_payload_envelope(existing_comment_body.clone(), 1),
            &list_key,
        )
        .expect("comment payload")
        .base64;

        Self {
            password,
            access_token: "test-access-token".to_string(),
            refresh_token: "refresh-token".to_string(),
            work_list_id,
            first_section_id,
            done_section_id,
            task_id,
            comment_id,
            membership_id,
            owner_user_id,
            workspace_id,
            mentioned_user_id,
            data_key,
            list_key,
            binding_key,
            opaque_setup: opaque_account.setup,
            opaque_password_file: opaque_account.password_file,
            opaque_export_key: opaque_account.export_key,
            data_key_ciphertext,
            work_list_key_ciphertext,
            work_list_payload_ciphertext,
            task_title_ciphertext,
            comment_body_ciphertext,
            existing_task_body,
            existing_comment_body,
            text_attachment,
            docx_attachment,
            binary_attachment,
            hostile_attachment,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateTaskRequestBody {
    title_ciphertext: String,
    title_ciphertext_proof: String,
    payload_ciphertext: String,
    payload_ciphertext_proof: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateTaskRequestBody {
    expected_updated_at: Option<DateTime<Utc>>,
    title_ciphertext: Option<String>,
    title_ciphertext_proof: Option<String>,
    payload_ciphertext: Option<String>,
    payload_ciphertext_proof: Option<String>,
    attachment_ids: Option<Vec<Uuid>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateNoteRequestBody {
    idempotency_key: String,
    idempotency_commitment: String,
    title_ciphertext: String,
    title_ciphertext_proof: String,
    payload_ciphertext: String,
    payload_ciphertext_proof: String,
    #[serde(default)]
    is_private: bool,
    note_key_ciphertext: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateNoteRequestBody {
    expected_updated_at: Option<DateTime<Utc>>,
    title_ciphertext: Option<String>,
    title_ciphertext_proof: Option<String>,
    payload_ciphertext: Option<String>,
    payload_ciphertext_proof: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InitiateAttachmentRequestBody {
    operation_id: Uuid,
    ciphertext_bytes: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompleteAttachmentRequestBody {
    ciphertext_bytes: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateCommentRequestBody {
    body_ciphertext: String,
    body_ciphertext_proof: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateCommentRequestBody {
    body_ciphertext: Option<String>,
    body_ciphertext_proof: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteRequestBody {
    audit_patch: Option<AuditPatchRequest>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MoveTaskRequestBody {
    expected_updated_at: Option<DateTime<Utc>>,
    section_id: Option<Uuid>,
    insert_before_task_id: Option<Uuid>,
    section_boundary: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RefreshRequestBody {
    refresh_token: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpaqueExportKeyStartRequestBody {
    client_login_state: String,
}

#[derive(Deserialize)]
struct IncludeArchivedQuery {
    #[serde(rename = "includeArchived")]
    include_archived: Option<bool>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MyTasksQuery {
    limit: Option<i64>,
    offset: Option<i64>,
    include_completed: Option<bool>,
}

async fn spawn_server(state: Arc<Mutex<TestState>>) -> TestServer {
    let app = Router::new()
        .route("/auth/refresh", post(refresh_session))
        .route(
            "/auth/opaque/export-key/start",
            post(start_opaque_export_key),
        )
        .route("/auth/logout", post(logout_session))
        .route("/work-lists", get(list_work_lists))
        .route("/work-lists/{id}", get(get_work_list))
        .route("/work-lists/{id}/archive", post(archive_work_list))
        .route("/work-lists/{id}/unarchive", post(unarchive_work_list))
        .route("/work-lists/{id}/audit-log", get(project_audit_log))
        .route("/work-lists/{id}/sse-token", post(issue_sse_token))
        .route("/work-lists/{id}/events", get(project_events))
        .route("/work-lists/{id}/tasks", get(list_tasks).post(create_task))
        .route("/work-lists/{id}/notes", get(list_notes).post(create_note))
        .route(
            "/work-lists/{id}/notes/{note_id}",
            get(get_note).patch(update_note).delete(delete_note),
        )
        .route(
            "/work-lists/{id}/attachments",
            post(initiate_attachment_upload),
        )
        .route(
            "/work-lists/{id}/attachments/{attachment_id}/complete",
            post(complete_attachment_upload),
        )
        .route(
            "/work-lists/{id}/attachments/{attachment_id}",
            axum::routing::delete(delete_attachment),
        )
        .route(
            "/work-lists/{id}/attachments/{attachment_id}/download",
            get(get_attachment_download),
        )
        .route(
            "/work-lists/{id}/tasks/{task_id}",
            get(get_task).patch(update_task).delete(delete_task),
        )
        .route("/work-lists/{id}/tasks/{task_id}/move", post(move_task))
        .route(
            "/work-lists/{id}/tasks/{task_id}/archive",
            post(archive_task),
        )
        .route(
            "/work-lists/{id}/tasks/{task_id}/unarchive",
            post(unarchive_task),
        )
        .route(
            "/work-lists/{id}/tasks/{task_id}/comments",
            get(list_comments).post(create_comment),
        )
        .route(
            "/work-lists/{id}/tasks/{task_id}/comments/{comment_id}",
            patch(update_comment).delete(delete_comment),
        )
        .route("/me/tasks", get(list_my_tasks))
        .route("/me/activity", get(my_activity))
        .route("/downloads/{attachment_id}", get(download_attachment_bytes))
        .route("/uploads/{attachment_id}", put(upload_attachment_bytes))
        .with_state(state.clone());

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let addr = listener.local_addr().expect("local addr");
    {
        let mut guard = state.lock().expect("state lock");
        guard.base_url = Some(format!("http://{}", addr));
    }
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve app");
    });

    TestServer {
        base_url: format!("http://{}", addr),
        _task: task,
    }
}

async fn spawn_doctor_health_server(
    response_delay: Option<std::time::Duration>,
) -> DoctorHealthServer {
    let observations = Arc::new(Mutex::new(Vec::new()));
    let state = DoctorHealthState {
        observations: Arc::clone(&observations),
        response_delay,
    };
    let app = Router::new()
        .route("/health", get(doctor_health))
        .with_state(state);
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind doctor health listener");
    let address = listener.local_addr().expect("doctor health address");
    let task = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("serve doctor health app");
    });

    DoctorHealthServer {
        base_url: format!("http://{address}"),
        observations,
        task,
    }
}

async fn doctor_health(State(state): State<DoctorHealthState>, headers: HeaderMap) -> StatusCode {
    let request = DoctorHealthRequest {
        user_agent: headers
            .get("user-agent")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned),
        request_id: headers
            .get("x-request-id")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned),
    };
    state
        .observations
        .lock()
        .expect("observations lock")
        .push(request);
    if let Some(response_delay) = state.response_delay {
        tokio::time::sleep(response_delay).await;
    }
    StatusCode::OK
}

async fn spawn_refresh_logout_race_server(fixture: &TestFixture) -> RefreshLogoutRaceServer {
    let state = Arc::new(Mutex::new(RefreshLogoutRaceInner {
        initial_refresh_token: fixture.refresh_token.clone(),
        refresh_requests: 0,
        logout_requests: 0,
        work_list_requests: 0,
        revoked: false,
    }));
    let refresh_committed = Arc::new(Notify::new());
    let release_refresh_response = Arc::new(Notify::new());
    let app_state = RefreshLogoutRaceAppState {
        state: Arc::clone(&state),
        refresh_committed: Arc::clone(&refresh_committed),
        release_refresh_response: Arc::clone(&release_refresh_response),
    };
    let app = Router::new()
        .route("/auth/refresh", post(refresh_logout_race_refresh))
        .route("/auth/logout", post(refresh_logout_race_logout))
        .route("/work-lists", get(refresh_logout_race_work_lists))
        .with_state(app_state);
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind refresh/logout race listener");
    let address = listener.local_addr().expect("race listener address");
    let task = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("serve refresh/logout race API");
    });

    RefreshLogoutRaceServer {
        base_url: format!("http://{address}"),
        state,
        refresh_committed,
        release_refresh_response,
        _task: task,
    }
}

async fn refresh_logout_race_refresh(
    State(state): State<RefreshLogoutRaceAppState>,
    Json(payload): Json<RefreshRequestBody>,
) -> (StatusCode, Json<serde_json::Value>) {
    {
        let mut inner = state.state.lock().expect("race state lock");
        assert_eq!(payload.refresh_token, inner.initial_refresh_token);
        inner.refresh_requests += 1;
    }
    state.refresh_committed.notify_one();
    state.release_refresh_response.notified().await;

    (
        StatusCode::OK,
        Json(json!({
            "accessToken": "race-access-token",
            "refreshToken": "race-refresh-token",
            "expiresIn": 3600,
            "refreshExpiresIn": 3600,
            "tokenType": "Bearer"
        })),
    )
}

async fn refresh_logout_race_logout(
    State(state): State<RefreshLogoutRaceAppState>,
    Json(payload): Json<RefreshRequestBody>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut inner = state.state.lock().expect("race state lock");
    inner.logout_requests += 1;
    if payload.refresh_token == inner.initial_refresh_token
        || payload.refresh_token == "race-refresh-token"
    {
        inner.revoked = true;
    }
    (
        StatusCode::OK,
        Json(json!({
            "loggedOut": true
        })),
    )
}

async fn refresh_logout_race_work_lists(
    State(state): State<RefreshLogoutRaceAppState>,
    headers: HeaderMap,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut inner = state.state.lock().expect("race state lock");
    inner.work_list_requests += 1;
    let authorized = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        == Some("Bearer race-access-token")
        && !inner.revoked;
    if authorized {
        (StatusCode::OK, Json(json!([])))
    } else {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "error": "unauthorized",
                "message": "session revoked"
            })),
        )
    }
}

async fn start_opaque_export_key(
    State(state): State<Arc<Mutex<TestState>>>,
    headers: HeaderMap,
    Json(payload): Json<OpaqueExportKeyStartRequestBody>,
) -> (StatusCode, Json<serde_json::Value>) {
    const SERVER_ID: &[u8] = b"worklist.api";

    authorize(&state, &headers);
    let mut state = state.lock().expect("state lock");
    state.opaque_export_key_start_count += 1;
    let credential_request = CredentialRequest::<ClientCipherSuite>::deserialize(&decode_b64(
        &payload.client_login_state,
    ))
    .expect("deserialize OPAQUE export-key request");
    let setup = ServerSetup::<ClientCipherSuite>::deserialize(&state.fixture.opaque_setup)
        .expect("deserialize OPAQUE setup");
    let password_file =
        ServerRegistration::<ClientCipherSuite>::deserialize(&state.fixture.opaque_password_file)
            .expect("deserialize OPAQUE password file");
    let identifiers = Identifiers {
        client: Some(FIXTURE_EMAIL.as_bytes()),
        server: Some(SERVER_ID),
    };
    let mut rng = OsRng;
    let login = ServerLogin::<ClientCipherSuite>::start(
        &mut rng,
        &setup,
        Some(password_file),
        credential_request,
        FIXTURE_EMAIL.as_bytes(),
        ServerLoginParameters {
            context: None,
            identifiers,
        },
    )
    .expect("start OPAQUE export-key login");

    (
        StatusCode::OK,
        Json(json!({
            "serverLoginState": STANDARD_NO_PAD.encode(login.message.serialize())
        })),
    )
}

async fn refresh_session(
    State(state): State<Arc<Mutex<TestState>>>,
    Json(payload): Json<RefreshRequestBody>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut state = state.lock().expect("state lock");
    assert_eq!(payload.refresh_token, state.current_refresh_token);

    state.refresh_request_count += 1;
    state.current_access_token = format!("refreshed-access-token-{}", state.refresh_request_count);
    state.current_refresh_token =
        format!("refreshed-refresh-token-{}", state.refresh_request_count);
    let access_token = state.current_access_token.clone();
    let refresh_token = state.current_refresh_token.clone();

    (
        StatusCode::OK,
        Json(json!({
            "accessToken": access_token,
            "refreshToken": refresh_token,
            "expiresIn": 3600,
            "refreshExpiresIn": 3600,
            "tokenType": "Bearer"
        })),
    )
}

async fn logout_session(
    State(state): State<Arc<Mutex<TestState>>>,
    Json(payload): Json<RefreshRequestBody>,
) -> (StatusCode, Json<serde_json::Value>) {
    let state = state.lock().expect("state lock");
    assert_eq!(payload.refresh_token, state.current_refresh_token);

    (
        state.logout_status,
        Json(json!({
            "loggedOut": state.logout_status.is_success()
        })),
    )
}

async fn list_work_lists(
    Query(query): Query<IncludeArchivedQuery>,
    State(state): State<Arc<Mutex<TestState>>>,
    headers: HeaderMap,
) -> (StatusCode, Json<serde_json::Value>) {
    authorize(&state, &headers);
    let mut state = state.lock().expect("state lock");
    let include_archived = query.include_archived.unwrap_or(false);
    state.list_work_list_include_archived.push(include_archived);
    let payload = if state.work_list_archived_at.is_some() && !include_archived {
        json!([])
    } else {
        json!([work_list_summary_json(&state)])
    };
    (StatusCode::OK, Json(payload))
}

async fn get_work_list(
    Path(work_list_id): Path<Uuid>,
    State(state): State<Arc<Mutex<TestState>>>,
    headers: HeaderMap,
) -> (StatusCode, Json<serde_json::Value>) {
    authorize(&state, &headers);
    let state = state.lock().expect("state lock");
    assert_eq!(work_list_id, state.fixture.work_list_id);

    let payload = json!({
        "id": state.fixture.work_list_id,
        "ownerUserId": state.fixture.owner_user_id,
        "workspaceId": state.fixture.workspace_id,
        "titleCiphertext": seal_text_value("Fixture Work List").expect("title").base64,
        "descriptionCiphertext": null,
        "payloadCiphertext": work_list_payload_ciphertext(&state),
        "timezone": "UTC",
        "sectionSnapshots": section_snapshots_json(&state),
        "archivedAt": state.work_list_archived_at,
        "createdAt": Utc::now(),
        "updatedAt": Utc::now(),
        "membership": membership_json(&state),
        "members": [
            membership_json(&state)
        ]
    });

    (StatusCode::OK, Json(payload))
}

async fn archive_work_list(
    Path(work_list_id): Path<Uuid>,
    State(state): State<Arc<Mutex<TestState>>>,
    headers: HeaderMap,
    _payload: Json<serde_json::Value>,
) -> (StatusCode, Json<serde_json::Value>) {
    authorize(&state, &headers);
    let mut state = state.lock().expect("state lock");
    assert_eq!(work_list_id, state.fixture.work_list_id);
    state.archive_work_list_count += 1;
    state.work_list_archived_at = Some(Utc::now());
    let payload = work_list_summary_json(&state);
    (StatusCode::OK, Json(payload))
}

async fn unarchive_work_list(
    Path(work_list_id): Path<Uuid>,
    State(state): State<Arc<Mutex<TestState>>>,
    headers: HeaderMap,
    _payload: Json<serde_json::Value>,
) -> (StatusCode, Json<serde_json::Value>) {
    authorize(&state, &headers);
    let mut state = state.lock().expect("state lock");
    assert_eq!(work_list_id, state.fixture.work_list_id);
    state.unarchive_work_list_count += 1;
    state.work_list_archived_at = None;
    let payload = work_list_summary_json(&state);
    (StatusCode::OK, Json(payload))
}

async fn list_notes(
    Path(work_list_id): Path<Uuid>,
    State(state): State<Arc<Mutex<TestState>>>,
    headers: HeaderMap,
) -> (StatusCode, Json<Value>) {
    authorize(&state, &headers);
    let state = state.lock().expect("state lock");
    assert_eq!(work_list_id, state.fixture.work_list_id);
    let notes = state
        .notes
        .values()
        .map(|note| note_response_json(&state, note))
        .collect::<Vec<_>>();
    (StatusCode::OK, Json(json!({ "notes": notes })))
}

async fn get_note(
    Path((work_list_id, note_id)): Path<(Uuid, Uuid)>,
    State(state): State<Arc<Mutex<TestState>>>,
    headers: HeaderMap,
) -> (StatusCode, Json<Value>) {
    authorize(&state, &headers);
    let state = state.lock().expect("state lock");
    assert_eq!(work_list_id, state.fixture.work_list_id);
    match state.notes.get(&note_id) {
        Some(note) => (StatusCode::OK, Json(note_response_json(&state, note))),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "not_found", "message": "note not found" })),
        ),
    }
}

async fn create_note(
    Path(work_list_id): Path<Uuid>,
    State(state): State<Arc<Mutex<TestState>>>,
    headers: HeaderMap,
    Json(payload): Json<CreateNoteRequestBody>,
) -> (StatusCode, Json<Value>) {
    authorize(&state, &headers);
    let mut state = state.lock().expect("state lock");
    assert_eq!(work_list_id, state.fixture.work_list_id);
    validate_note_ciphertexts(
        &state.fixture,
        payload.is_private,
        payload.note_key_ciphertext.as_deref(),
        &payload.title_ciphertext,
        &payload.title_ciphertext_proof,
        &payload.payload_ciphertext,
        &payload.payload_ciphertext_proof,
    );
    if let Some((commitment, note_id)) = state
        .note_create_operations
        .get(&payload.idempotency_key)
        .cloned()
    {
        if commitment != payload.idempotency_commitment {
            return (
                StatusCode::CONFLICT,
                Json(json!({
                    "error": "conflict",
                    "message": "idempotencyKey was already used for a different note create"
                })),
            );
        }
        let note = state.notes.get(&note_id).expect("idempotent note remains");
        return (StatusCode::OK, Json(note_response_json(&state, note)));
    }
    let now = Utc::now();
    let note = StoredNote {
        id: Uuid::now_v7(),
        title_ciphertext: payload.title_ciphertext,
        payload_ciphertext: payload.payload_ciphertext,
        is_private: payload.is_private,
        note_key_ciphertext: payload.note_key_ciphertext,
        created_at: now,
        updated_at: now,
    };
    let response = note_response_json(&state, &note);
    state.note_create_operations.insert(
        payload.idempotency_key,
        (payload.idempotency_commitment, note.id),
    );
    state.notes.insert(note.id, note);
    (StatusCode::CREATED, Json(response))
}

async fn update_note(
    Path((work_list_id, note_id)): Path<(Uuid, Uuid)>,
    State(state): State<Arc<Mutex<TestState>>>,
    headers: HeaderMap,
    Json(payload): Json<UpdateNoteRequestBody>,
) -> (StatusCode, Json<Value>) {
    authorize(&state, &headers);
    let mut state = state.lock().expect("state lock");
    assert_eq!(work_list_id, state.fixture.work_list_id);
    let Some(current) = state.notes.get(&note_id).cloned() else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "not_found", "message": "note not found" })),
        );
    };
    assert_eq!(payload.expected_updated_at, Some(current.updated_at));
    let title_ciphertext = payload
        .title_ciphertext
        .unwrap_or_else(|| current.title_ciphertext.clone());
    let payload_ciphertext = payload
        .payload_ciphertext
        .unwrap_or_else(|| current.payload_ciphertext.clone());
    validate_note_ciphertexts(
        &state.fixture,
        current.is_private,
        current.note_key_ciphertext.as_deref(),
        &title_ciphertext,
        payload
            .title_ciphertext_proof
            .as_deref()
            .expect("updated note title proof"),
        &payload_ciphertext,
        payload
            .payload_ciphertext_proof
            .as_deref()
            .expect("updated note payload proof"),
    );
    let note = StoredNote {
        title_ciphertext,
        payload_ciphertext,
        updated_at: current.updated_at + Duration::milliseconds(1),
        ..current
    };
    let response = note_response_json(&state, &note);
    state.notes.insert(note.id, note);
    (StatusCode::OK, Json(response))
}

async fn delete_note(
    Path((work_list_id, note_id)): Path<(Uuid, Uuid)>,
    State(state): State<Arc<Mutex<TestState>>>,
    headers: HeaderMap,
    _payload: Option<Json<DeleteRequestBody>>,
) -> StatusCode {
    authorize(&state, &headers);
    let mut state = state.lock().expect("state lock");
    assert_eq!(work_list_id, state.fixture.work_list_id);
    if state.notes.remove(&note_id).is_some() {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn initiate_attachment_upload(
    Path(work_list_id): Path<Uuid>,
    State(state): State<Arc<Mutex<TestState>>>,
    headers: HeaderMap,
    Json(payload): Json<InitiateAttachmentRequestBody>,
) -> (StatusCode, Json<Value>) {
    authorize(&state, &headers);
    let mut state = state.lock().expect("state lock");
    assert_eq!(work_list_id, state.fixture.work_list_id);
    assert!(!payload.operation_id.is_nil());
    let attachment_id = Uuid::now_v7();
    state
        .attachment_upload_expected_bytes
        .insert(attachment_id, payload.ciphertext_bytes);
    let base_url = state.base_url.clone().expect("base URL");
    (
        StatusCode::CREATED,
        Json(json!({
            "attachmentId": attachment_id,
            "uploadUrl": format!("{base_url}/uploads/{attachment_id}"),
            "uploadHeaders": { "content-type": "application/octet-stream" },
            "expiresAt": Utc::now() + Duration::minutes(5)
        })),
    )
}

async fn upload_attachment_bytes(
    Path(attachment_id): Path<Uuid>,
    State(state): State<Arc<Mutex<TestState>>>,
    headers: HeaderMap,
    body: Bytes,
) -> StatusCode {
    assert_eq!(
        headers
            .get("content-type")
            .and_then(|value| value.to_str().ok()),
        Some("application/octet-stream")
    );
    let mut state = state.lock().expect("state lock");
    assert_eq!(
        state.attachment_upload_expected_bytes.get(&attachment_id),
        Some(&(body.len() as u64))
    );
    state
        .attachment_uploads
        .insert(attachment_id, body.to_vec());
    StatusCode::OK
}

async fn complete_attachment_upload(
    Path((work_list_id, attachment_id)): Path<(Uuid, Uuid)>,
    State(state): State<Arc<Mutex<TestState>>>,
    headers: HeaderMap,
    Json(payload): Json<CompleteAttachmentRequestBody>,
) -> StatusCode {
    authorize(&state, &headers);
    let mut state = state.lock().expect("state lock");
    assert_eq!(work_list_id, state.fixture.work_list_id);
    assert_eq!(
        state.attachment_upload_expected_bytes.get(&attachment_id),
        Some(&payload.ciphertext_bytes)
    );
    assert_eq!(
        state
            .attachment_uploads
            .get(&attachment_id)
            .map(|bytes| bytes.len() as u64),
        Some(payload.ciphertext_bytes)
    );
    state.completed_attachment_ids.insert(attachment_id);
    StatusCode::NO_CONTENT
}

async fn delete_attachment(
    Path((work_list_id, attachment_id)): Path<(Uuid, Uuid)>,
    State(state): State<Arc<Mutex<TestState>>>,
    headers: HeaderMap,
) -> StatusCode {
    authorize(&state, &headers);
    let mut state = state.lock().expect("state lock");
    assert_eq!(work_list_id, state.fixture.work_list_id);
    if task_attachment_ids(&state.current_task_body).contains(&attachment_id) {
        return StatusCode::CONFLICT;
    }
    state
        .attachment_upload_expected_bytes
        .remove(&attachment_id);
    state.attachment_uploads.remove(&attachment_id);
    state.completed_attachment_ids.remove(&attachment_id);
    state.deleted_attachment_ids.push(attachment_id);
    StatusCode::NO_CONTENT
}

async fn get_attachment_download(
    Path((work_list_id, attachment_id)): Path<(Uuid, Uuid)>,
    State(state): State<Arc<Mutex<TestState>>>,
    headers: HeaderMap,
) -> (StatusCode, Json<serde_json::Value>) {
    authorize(&state, &headers);
    let state = state.lock().expect("state lock");
    assert_eq!(work_list_id, state.fixture.work_list_id);
    let attachment = attachment_by_id(&state.fixture, attachment_id).expect("attachment fixture");
    let base_url = state.base_url.clone().expect("base url");

    (
        StatusCode::OK,
        Json(json!({
            "downloadUrl": format!("{base_url}/downloads/{}", attachment.id),
            "downloadHeaders": {
                "if-match": "test-etag"
            },
            "expiresAt": Utc::now() + Duration::minutes(5)
        })),
    )
}

async fn list_tasks(
    Path(work_list_id): Path<Uuid>,
    Query(query): Query<IncludeArchivedQuery>,
    State(state): State<Arc<Mutex<TestState>>>,
    headers: HeaderMap,
) -> (StatusCode, Json<serde_json::Value>) {
    authorize(&state, &headers);
    let include_archived = query.include_archived.unwrap_or(false);
    let mut state = state.lock().expect("state lock");
    assert_eq!(work_list_id, state.fixture.work_list_id);
    state.list_task_include_archived.push(include_archived);
    if state.d6_watch_enabled {
        state.d6_watch_call_order.push("list");
        state.d6_watch_task_reads += 1;
    }

    let tasks = if state.tasks_empty || (state.task_archived_at.is_some() && !include_archived) {
        Vec::new()
    } else {
        let mut task = task_response_json(&state);
        if state.d6_watch_enabled && state.d6_watch_task_reads >= 2 {
            task["commentCount"] = json!(2);
        }
        vec![task]
    };
    let payload = json!({
        "tasks": tasks,
        "archivedCounts": [
            {
                "sectionId": null,
                "count": 0
            }
        ]
    });

    (StatusCode::OK, Json(payload))
}

async fn project_audit_log(
    Path(work_list_id): Path<Uuid>,
    State(state): State<Arc<Mutex<TestState>>>,
    headers: HeaderMap,
) -> (StatusCode, Json<serde_json::Value>) {
    authorize(&state, &headers);
    let state = state.lock().expect("state lock");
    assert_eq!(work_list_id, state.fixture.work_list_id);
    (
        StatusCode::OK,
        Json(json!({
            "events": [audit_event_json(&state, 1, Utc::now())],
            "nextCursor": null,
        })),
    )
}

async fn my_activity(
    State(state): State<Arc<Mutex<TestState>>>,
    headers: HeaderMap,
) -> (StatusCode, Json<serde_json::Value>) {
    authorize(&state, &headers);
    let mut state = state.lock().expect("state lock");
    state.d6_activity_reads += 1;
    let now = Utc::now();
    let events = if state.d6_activity_reads == 1 {
        vec![
            audit_event_json(&state, 2, now - Duration::minutes(2)),
            audit_event_json(&state, 1, now - Duration::minutes(3)),
        ]
    } else {
        vec![
            audit_event_json(&state, 3, now - Duration::minutes(1)),
            audit_event_json(&state, 2, now - Duration::minutes(2)),
            audit_event_json(&state, 1, now - Duration::minutes(3)),
        ]
    };
    (
        StatusCode::OK,
        Json(json!({
            "events": events,
            "nextCursor": null,
        })),
    )
}

fn audit_event_json(
    state: &TestState,
    ordinal: u128,
    occurred_at: DateTime<Utc>,
) -> serde_json::Value {
    json!({
        "id": Uuid::from_u128(state.fixture.task_id.as_u128().wrapping_add(ordinal)),
        "workspaceId": state.fixture.workspace_id,
        "workListId": state.fixture.work_list_id,
        "taskId": state.fixture.task_id,
        "commentId": null,
        "entityType": "task",
        "entityId": state.fixture.task_id,
        "action": format!("updated_{ordinal}"),
        "scopeLevel": "task",
        "actorUserId": state.fixture.owner_user_id,
        "actorUserName": "Fixture Operator",
        "actorMembershipId": state.fixture.membership_id,
        "actorType": "user",
        "sourceKind": "api",
        "targetVersion": i64::try_from(ordinal).expect("small audit ordinal"),
        "clientVersion": "test-client",
        "occurredAt": occurred_at,
        "changes": [],
        "payload": {
            "payloadCiphertext": "D6_PAYLOAD_CIPHERTEXT_CANARY",
            "payloadHmac": "D6_PAYLOAD_HMAC_CANARY",
            "payloadVersion": 1,
            "fieldDescriptors": [],
        },
    })
}

async fn issue_sse_token(
    Path(work_list_id): Path<Uuid>,
    State(state): State<Arc<Mutex<TestState>>>,
    headers: HeaderMap,
) -> (StatusCode, Json<serde_json::Value>) {
    authorize(&state, &headers);
    let mut state = state.lock().expect("state lock");
    assert_eq!(work_list_id, state.fixture.work_list_id);
    state.d6_watch_call_order.push("token");
    (
        StatusCode::OK,
        Json(json!({
            "token": "D6_SECRET_STREAM_TOKEN",
            "expiresIn": 120,
        })),
    )
}

async fn project_events(
    Path(work_list_id): Path<Uuid>,
    State(state): State<Arc<Mutex<TestState>>>,
) -> (StatusCode, [(&'static str, &'static str); 1], String) {
    let mut state = state.lock().expect("state lock");
    assert_eq!(work_list_id, state.fixture.work_list_id);
    state.d6_watch_call_order.push("connect");
    let event = json!({
        "eventId": Uuid::now_v7(),
        "workListId": work_list_id,
        "actorUserId": state.fixture.owner_user_id,
        "actorMembershipId": state.fixture.membership_id,
        "actorInstanceId": "D6_ACTOR_INSTANCE_CANARY",
        "occurredAt": Utc::now(),
        "type": "task_updated",
        "taskId": state.fixture.task_id,
        "fields": ["commentCount"],
    });
    let body = format!(
        "event: board_event\ndata: {event}\n\nevent: resync\ndata: {{\"missedEvents\":2}}\n\n"
    );
    (
        StatusCode::OK,
        [("content-type", "text/event-stream")],
        body,
    )
}

async fn list_my_tasks(
    Query(query): Query<MyTasksQuery>,
    State(state): State<Arc<Mutex<TestState>>>,
    headers: HeaderMap,
) -> (StatusCode, Json<serde_json::Value>) {
    authorize(&state, &headers);
    let mut state = state.lock().expect("state lock");
    let limit = query.limit.unwrap_or(100).clamp(1, 100);
    let offset = query.offset.unwrap_or(0).max(0);
    let include_completed = query.include_completed.unwrap_or(false);
    state.my_tasks_queries.push((offset, include_completed));

    let total = if state.tasks_empty || (state.task_is_completed && !include_completed) {
        0
    } else {
        state.my_tasks_count
    };
    let start = usize::try_from(offset).unwrap_or(usize::MAX).min(total);
    let page_limit = usize::try_from(limit).unwrap_or(usize::MAX);
    let end = start.saturating_add(page_limit).min(total);
    let work_list_title_ciphertext = seal_text_value("Fixture Work List").expect("title").base64;
    let task_payload_ciphertext = task_payload_ciphertext(&state);
    let tasks = (start..end)
        .map(|index| {
            let task_id =
                Uuid::from_u128(state.fixture.task_id.as_u128().wrapping_add(index as u128));
            json!({
                "id": task_id,
                "workListId": state.fixture.work_list_id,
                "workListTitleCiphertext": work_list_title_ciphertext,
                "createdByMembershipId": state.fixture.membership_id,
                "titleCiphertext": state.fixture.task_title_ciphertext,
                "payloadCiphertext": task_payload_ciphertext,
                "sectionId": state.task_section_id,
                "priority": null,
                "dueAt": null,
                "startAt": null,
                "completedAt": state.task_completed_at,
                "isCompleted": state.task_is_completed,
                "createdAt": Utc::now(),
                "updatedAt": state.task_updated_at,
                "commentCount": 1,
                "delegations": [
                    {
                        "id": Uuid::from_u128(
                            state.fixture.membership_id.as_u128().wrapping_add(index as u128)
                        ),
                        "taskId": task_id,
                        "membershipId": state.fixture.membership_id,
                        "role": "assigned",
                        "status": "pending",
                        "noteCiphertext": null,
                        "createdAt": Utc::now(),
                        "updatedAt": Utc::now()
                    }
                ]
            })
        })
        .collect::<Vec<_>>();
    let payload = json!({
        "total": total,
        "tasks": tasks,
        "limit": limit,
        "offset": offset
    });

    (StatusCode::OK, Json(payload))
}

async fn get_task(
    Path((work_list_id, task_id)): Path<(Uuid, Uuid)>,
    State(state): State<Arc<Mutex<TestState>>>,
    headers: HeaderMap,
) -> (StatusCode, Json<serde_json::Value>) {
    authorize(&state, &headers);
    let state = state.lock().expect("state lock");
    assert_eq!(work_list_id, state.fixture.work_list_id);
    assert_eq!(task_id, state.fixture.task_id);

    let payload = json!({
        "id": state.fixture.task_id,
        "workListId": state.fixture.work_list_id,
        "createdByMembershipId": state.fixture.membership_id,
        "titleCiphertext": state.fixture.task_title_ciphertext,
        "payloadCiphertext": task_payload_ciphertext(&state),
        "sectionId": state.task_section_id,
        "priority": null,
        "position": "a",
        "dueAt": null,
        "startAt": null,
        "completedAt": state.task_completed_at,
        "archivedAt": state.task_archived_at,
        "isCompleted": state.task_is_completed,
        "recurrenceId": null,
        "recurrenceSchedule": null,
        "recurrenceIteration": null,
        "materializedAt": null,
        "createdAt": Utc::now(),
        "updatedAt": state.task_updated_at,
        "commentCount": 1,
        "delegations": [],
        "comments": [
            {
                "id": state.fixture.comment_id,
                "taskId": state.fixture.task_id,
                "authorMembershipId": state.fixture.membership_id,
                "bodyCiphertext": comment_body_ciphertext(&state),
                "createdAt": Utc::now(),
                "updatedAt": Utc::now()
            }
        ]
    });

    (StatusCode::OK, Json(payload))
}

async fn create_task(
    Path(work_list_id): Path<Uuid>,
    State(state): State<Arc<Mutex<TestState>>>,
    headers: HeaderMap,
    Json(payload_json): Json<Value>,
) -> (StatusCode, Json<serde_json::Value>) {
    authorize(&state, &headers);
    let (response, committed, release_response) = {
        let mut state = state.lock().expect("state lock");
        assert_eq!(work_list_id, state.fixture.work_list_id);
        let payload: CreateTaskRequestBody =
            serde_json::from_value(payload_json.clone()).expect("task create request");
        state.created_task_request = Some(payload_json.clone());

        let title_bytes = decode_b64(&payload.title_ciphertext);
        let title_proof =
            compute_payload_proof(&title_bytes, &state.fixture.binding_key).expect("title proof");
        assert_eq!(title_proof, payload.title_ciphertext_proof);

        let payload_bytes = decode_b64(&payload.payload_ciphertext);
        let payload_proof = compute_payload_proof(&payload_bytes, &state.fixture.binding_key)
            .expect("payload proof");
        assert_eq!(payload_proof, payload.payload_ciphertext_proof);

        let decrypted = decrypt_task_payload(&state.fixture.list_key, &payload_bytes)
            .expect("decrypt created task");
        assert_eq!(
            decrypt_encrypted_text_value(&title_bytes, &state.fixture.list_key, TASK_TITLE_CONTEXT,)
                .expect("decrypt created task title"),
            decrypted.body.title
        );
        state.created_task_body = Some(decrypted.body.clone());

        let response = json!({
            "id": Uuid::now_v7(),
            "workListId": state.fixture.work_list_id,
            "createdByMembershipId": state.fixture.membership_id,
            "titleCiphertext": payload.title_ciphertext,
            "payloadCiphertext": payload.payload_ciphertext,
            "sectionId": payload_json.get("sectionId").cloned().unwrap_or(Value::Null),
            "priority": payload_json.get("priority").cloned().unwrap_or(Value::Null),
            "position": "b",
            "dueAt": payload_json.get("dueAt").cloned().unwrap_or(Value::Null),
            "startAt": payload_json.get("startAt").cloned().unwrap_or(Value::Null),
            "completedAt": null,
            "archivedAt": null,
            "isCompleted": false,
            "recurrenceId": null,
            "recurrenceSchedule": null,
            "recurrenceIteration": null,
            "materializedAt": null,
            "createdAt": Utc::now(),
            "updatedAt": Utc::now(),
            "commentCount": 0,
            "delegations": []
        });
        (
            response,
            state.d7_task_create_committed.clone(),
            state.d7_task_create_release_response.clone(),
        )
    };
    if let Some(committed) = committed {
        committed.notify_one();
    }
    if let Some(release_response) = release_response {
        release_response.notified().await;
    }

    (StatusCode::OK, Json(response))
}

async fn update_task(
    Path((work_list_id, task_id)): Path<(Uuid, Uuid)>,
    State(state): State<Arc<Mutex<TestState>>>,
    headers: HeaderMap,
    Json(payload_json): Json<Value>,
) -> (StatusCode, Json<serde_json::Value>) {
    authorize(&state, &headers);
    let (response, committed, release_response) = {
        let mut state = state.lock().expect("state lock");
        assert_eq!(work_list_id, state.fixture.work_list_id);
        assert_eq!(task_id, state.fixture.task_id);
        let payload: UpdateTaskRequestBody =
            serde_json::from_value(payload_json.clone()).expect("task update request");
        assert_eq!(payload.expected_updated_at, Some(state.task_updated_at));
        state.updated_task_request = Some(payload_json.clone());
        if state.reject_next_task_update_as_conflict {
            state.reject_next_task_update_as_conflict = false;
            return (
                StatusCode::CONFLICT,
                Json(json!({
                    "error": "conflict",
                    "message": "task changed while it was being edited; reload and try again"
                })),
            );
        }

        if let Some(payload_ciphertext) = payload.payload_ciphertext.as_ref() {
            let payload_bytes = decode_b64(payload_ciphertext);
            let payload_proof = compute_payload_proof(&payload_bytes, &state.fixture.binding_key)
                .expect("payload proof");
            assert_eq!(
                payload.payload_ciphertext_proof.as_deref(),
                Some(payload_proof.as_str())
            );

            let decrypted = decrypt_task_payload(&state.fixture.list_key, &payload_bytes)
                .expect("decrypt updated task");
            state.updated_task_body = Some(decrypted.body.clone());
            if let Some(attachment_ids) = payload.attachment_ids.as_ref() {
                let payload_ids = task_attachment_ids(&decrypted.body);
                assert_eq!(
                    payload_ids.iter().copied().collect::<HashSet<_>>(),
                    attachment_ids.iter().copied().collect::<HashSet<_>>()
                );
                let previous_ids = task_attachment_ids(&state.current_task_body);
                for removed in previous_ids
                    .into_iter()
                    .filter(|id| !payload_ids.contains(id))
                {
                    state.deleted_attachment_ids.push(removed);
                    state.attachment_uploads.remove(&removed);
                    state.completed_attachment_ids.remove(&removed);
                }
                for added in payload_ids
                    .iter()
                    .filter(|id| !task_attachment_ids(&state.current_task_body).contains(id))
                {
                    assert!(
                        state.completed_attachment_ids.contains(added),
                        "new task attachment must be completed before attachment"
                    );
                }
            }
            state.current_task_body = decrypted.body.clone();
        }

        if let Some(title_ciphertext) = payload.title_ciphertext.as_ref() {
            let title_bytes = decode_b64(title_ciphertext);
            let title_proof = compute_payload_proof(&title_bytes, &state.fixture.binding_key)
                .expect("title proof");
            assert_eq!(
                payload.title_ciphertext_proof.as_deref(),
                Some(title_proof.as_str())
            );
            assert_eq!(
                decrypt_encrypted_text_value(
                    &title_bytes,
                    &state.fixture.list_key,
                    TASK_TITLE_CONTEXT,
                )
                .expect("decrypt updated task title"),
                state.current_task_body.title
            );
        }

        state.task_updated_at += Duration::milliseconds(1);

        let response = json!({
            "id": state.fixture.task_id,
            "workListId": state.fixture.work_list_id,
            "createdByMembershipId": state.fixture.membership_id,
            "titleCiphertext": payload.title_ciphertext.unwrap_or_else(|| state.fixture.task_title_ciphertext.clone()),
            "payloadCiphertext": task_payload_ciphertext(&state),
            "sectionId": payload_json.get("sectionId").cloned().unwrap_or_else(|| json!(state.task_section_id)),
            "priority": payload_json.get("priority").cloned().unwrap_or(Value::Null),
            "position": "a",
            "dueAt": payload_json.get("dueAt").cloned().unwrap_or(Value::Null),
            "startAt": payload_json.get("startAt").cloned().unwrap_or(Value::Null),
            "completedAt": null,
            "archivedAt": null,
            "isCompleted": false,
            "recurrenceId": null,
            "recurrenceSchedule": null,
            "recurrenceIteration": null,
            "materializedAt": null,
            "createdAt": Utc::now(),
            "updatedAt": state.task_updated_at,
            "commentCount": 1,
            "delegations": []
        });
        (
            response,
            state.d7_task_update_committed.clone(),
            state.d7_task_update_release_response.clone(),
        )
    };
    if let Some(committed) = committed {
        committed.notify_one();
    }
    if let Some(release_response) = release_response {
        release_response.notified().await;
    }

    (StatusCode::OK, Json(response))
}

async fn move_task(
    Path((work_list_id, task_id)): Path<(Uuid, Uuid)>,
    State(state): State<Arc<Mutex<TestState>>>,
    headers: HeaderMap,
    Json(payload): Json<MoveTaskRequestBody>,
) -> (StatusCode, Json<serde_json::Value>) {
    authorize(&state, &headers);
    let mut state = state.lock().expect("state lock");
    assert_eq!(work_list_id, state.fixture.work_list_id);
    assert_eq!(task_id, state.fixture.task_id);
    assert_eq!(payload.expected_updated_at, Some(state.task_updated_at));
    state.moved_task_body = Some(payload.clone());
    let desired_completion = match payload.section_boundary.as_deref() {
        Some("first") => Some(false),
        Some("last") => Some(true),
        Some(boundary) => panic!("unexpected section boundary: {boundary}"),
        None => None,
    };
    if desired_completion != Some(state.task_is_completed) {
        state.task_section_id = match payload.section_boundary.as_deref() {
            Some("first") => Some(state.fixture.first_section_id),
            Some("last") => Some(state.fixture.done_section_id),
            None => payload.section_id,
            Some(_) => unreachable!("validated section boundary"),
        };
        state.task_is_completed = state.task_section_id == Some(state.fixture.done_section_id);
        if state.task_is_completed {
            state.task_completed_at = Some(Utc::now());
        }
        state.task_updated_at += Duration::milliseconds(1);
    }

    let response = json!({
        "id": state.fixture.task_id,
        "workListId": state.fixture.work_list_id,
        "createdByMembershipId": state.fixture.membership_id,
        "titleCiphertext": state.fixture.task_title_ciphertext,
        "payloadCiphertext": task_payload_ciphertext(&state),
        "sectionId": state.task_section_id,
        "priority": null,
        "position": "moved",
        "dueAt": null,
        "startAt": null,
        "completedAt": state.task_completed_at,
        "archivedAt": null,
        "isCompleted": state.task_is_completed,
        "recurrenceId": null,
        "recurrenceSchedule": null,
        "recurrenceIteration": null,
        "materializedAt": null,
        "createdAt": Utc::now(),
        "updatedAt": state.task_updated_at,
        "commentCount": 1,
        "delegations": []
    });

    (StatusCode::OK, Json(response))
}

async fn archive_task(
    Path((work_list_id, task_id)): Path<(Uuid, Uuid)>,
    State(state): State<Arc<Mutex<TestState>>>,
    headers: HeaderMap,
    _payload: Json<serde_json::Value>,
) -> (StatusCode, Json<serde_json::Value>) {
    authorize(&state, &headers);
    let mut state = state.lock().expect("state lock");
    assert_eq!(work_list_id, state.fixture.work_list_id);
    assert_eq!(task_id, state.fixture.task_id);
    state.archive_task_count += 1;
    let archived_at = Utc::now();
    state.task_archived_at = Some(archived_at);

    let response = json!({
        "id": state.fixture.task_id,
        "workListId": state.fixture.work_list_id,
        "createdByMembershipId": state.fixture.membership_id,
        "titleCiphertext": state.fixture.task_title_ciphertext,
        "payloadCiphertext": task_payload_ciphertext(&state),
        "sectionId": null,
        "priority": null,
        "position": "a",
        "dueAt": null,
        "startAt": null,
        "completedAt": null,
        "archivedAt": archived_at,
        "isCompleted": false,
        "recurrenceId": null,
        "recurrenceSchedule": null,
        "recurrenceIteration": null,
        "materializedAt": null,
        "createdAt": Utc::now(),
        "updatedAt": Utc::now(),
        "commentCount": 1,
        "delegations": []
    });

    (StatusCode::OK, Json(response))
}

async fn unarchive_task(
    Path((work_list_id, task_id)): Path<(Uuid, Uuid)>,
    State(state): State<Arc<Mutex<TestState>>>,
    headers: HeaderMap,
    _payload: Json<serde_json::Value>,
) -> (StatusCode, Json<serde_json::Value>) {
    authorize(&state, &headers);
    let mut state = state.lock().expect("state lock");
    assert_eq!(work_list_id, state.fixture.work_list_id);
    assert_eq!(task_id, state.fixture.task_id);
    state.unarchive_task_count += 1;
    state.task_archived_at = None;

    let response = json!({
        "id": state.fixture.task_id,
        "workListId": state.fixture.work_list_id,
        "createdByMembershipId": state.fixture.membership_id,
        "titleCiphertext": state.fixture.task_title_ciphertext,
        "payloadCiphertext": task_payload_ciphertext(&state),
        "sectionId": null,
        "priority": null,
        "position": "a",
        "dueAt": null,
        "startAt": null,
        "completedAt": null,
        "archivedAt": state.task_archived_at,
        "isCompleted": false,
        "recurrenceId": null,
        "recurrenceSchedule": null,
        "recurrenceIteration": null,
        "materializedAt": null,
        "createdAt": Utc::now(),
        "updatedAt": Utc::now(),
        "commentCount": 1,
        "delegations": []
    });

    (StatusCode::OK, Json(response))
}

async fn delete_task(
    Path((work_list_id, task_id)): Path<(Uuid, Uuid)>,
    State(state): State<Arc<Mutex<TestState>>>,
    headers: HeaderMap,
    payload: Option<Json<DeleteRequestBody>>,
) -> StatusCode {
    authorize(&state, &headers);
    let mut state = state.lock().expect("state lock");
    assert_eq!(work_list_id, state.fixture.work_list_id);
    assert_eq!(task_id, state.fixture.task_id);
    state.deleted_task_audit_patch = delete_request_audit_patch(payload);
    state.deleted_task_id = Some(task_id);
    StatusCode::NO_CONTENT
}

async fn create_comment(
    Path((work_list_id, task_id)): Path<(Uuid, Uuid)>,
    State(state): State<Arc<Mutex<TestState>>>,
    headers: HeaderMap,
    Json(payload): Json<CreateCommentRequestBody>,
) -> (StatusCode, Json<serde_json::Value>) {
    authorize(&state, &headers);
    let mut state = state.lock().expect("state lock");
    assert_eq!(work_list_id, state.fixture.work_list_id);
    assert_eq!(task_id, state.fixture.task_id);

    let body_bytes = decode_b64(&payload.body_ciphertext);
    let body_proof =
        compute_payload_proof(&body_bytes, &state.fixture.binding_key).expect("comment proof");
    assert_eq!(body_proof, payload.body_ciphertext_proof);

    let decrypted = decrypt_comment_payload(&state.fixture.list_key, &body_bytes)
        .expect("decrypt created comment");
    state.created_comment_body = Some(decrypted.body.clone());

    let response = json!({
        "id": Uuid::now_v7(),
        "taskId": state.fixture.task_id,
        "authorMembershipId": state.fixture.membership_id,
        "bodyCiphertext": payload.body_ciphertext,
        "createdAt": Utc::now(),
        "updatedAt": Utc::now()
    });

    (StatusCode::CREATED, Json(response))
}

async fn list_comments(
    Path((work_list_id, task_id)): Path<(Uuid, Uuid)>,
    State(state): State<Arc<Mutex<TestState>>>,
    headers: HeaderMap,
) -> (StatusCode, Json<serde_json::Value>) {
    authorize(&state, &headers);
    let mut state = state.lock().expect("state lock");
    assert_eq!(work_list_id, state.fixture.work_list_id);
    assert_eq!(task_id, state.fixture.task_id);
    state.list_comments_count += 1;

    if state.deleted_comment_id == Some(state.fixture.comment_id) {
        return (StatusCode::OK, Json(json!([])));
    }

    (
        StatusCode::OK,
        Json(json!([
            {
                "id": state.fixture.comment_id,
                "taskId": state.fixture.task_id,
                "authorMembershipId": state.fixture.membership_id,
                "bodyCiphertext": comment_body_ciphertext(&state),
                "createdAt": Utc::now(),
                "updatedAt": Utc::now()
            }
        ])),
    )
}

async fn update_comment(
    Path((work_list_id, task_id, comment_id)): Path<(Uuid, Uuid, Uuid)>,
    State(state): State<Arc<Mutex<TestState>>>,
    headers: HeaderMap,
    Json(payload): Json<UpdateCommentRequestBody>,
) -> (StatusCode, Json<serde_json::Value>) {
    authorize(&state, &headers);
    let mut state = state.lock().expect("state lock");
    assert_eq!(work_list_id, state.fixture.work_list_id);
    assert_eq!(task_id, state.fixture.task_id);
    assert_eq!(comment_id, state.fixture.comment_id);

    let body_ciphertext = payload
        .body_ciphertext
        .as_ref()
        .expect("body ciphertext present");
    let body_bytes = decode_b64(body_ciphertext);
    let body_proof =
        compute_payload_proof(&body_bytes, &state.fixture.binding_key).expect("comment proof");
    assert_eq!(
        payload.body_ciphertext_proof.as_deref(),
        Some(body_proof.as_str())
    );

    let decrypted = decrypt_comment_payload(&state.fixture.list_key, &body_bytes)
        .expect("decrypt updated comment");
    state.updated_comment_body = Some(decrypted.body.clone());

    let response = json!({
        "id": state.fixture.comment_id,
        "taskId": state.fixture.task_id,
        "authorMembershipId": state.fixture.membership_id,
        "bodyCiphertext": body_ciphertext,
        "createdAt": Utc::now(),
        "updatedAt": Utc::now()
    });

    (StatusCode::OK, Json(response))
}

async fn delete_comment(
    Path((work_list_id, task_id, comment_id)): Path<(Uuid, Uuid, Uuid)>,
    State(state): State<Arc<Mutex<TestState>>>,
    headers: HeaderMap,
    payload: Option<Json<DeleteRequestBody>>,
) -> StatusCode {
    authorize(&state, &headers);
    let mut state = state.lock().expect("state lock");
    assert_eq!(work_list_id, state.fixture.work_list_id);
    assert_eq!(task_id, state.fixture.task_id);
    assert_eq!(comment_id, state.fixture.comment_id);
    state.deleted_comment_audit_patch = delete_request_audit_patch(payload);
    state.deleted_comment_id = Some(comment_id);
    StatusCode::NO_CONTENT
}

fn delete_request_audit_patch(
    payload: Option<Json<DeleteRequestBody>>,
) -> Option<AuditPatchRequest> {
    payload.and_then(|Json(payload)| payload.audit_patch)
}

fn delete_input_audit_patch(
    field: &str,
    payload_ciphertext: &str,
    payload_ciphertext_proof: &str,
) -> AuditPatchRequest {
    AuditPatchRequest {
        fields: vec![AuditPatchFieldRequest {
            field: field.to_string(),
            change_kind: "clear".to_string(),
            before_scalar: None,
            after_scalar: None,
            before_ciphertext_digest: None,
            after_ciphertext_digest: None,
        }],
        payload_ciphertext: payload_ciphertext.to_string(),
        payload_ciphertext_proof: payload_ciphertext_proof.to_string(),
        payload_version: 1,
    }
}

fn delete_input(
    field: &str,
    payload_ciphertext: &str,
    payload_ciphertext_proof: &str,
) -> (Value, AuditPatchRequest) {
    let audit_patch = delete_input_audit_patch(field, payload_ciphertext, payload_ciphertext_proof);
    let input = json!({
        "auditPatch": audit_patch.clone()
    });

    (input, audit_patch)
}

async fn download_attachment_bytes(
    Path(attachment_id): Path<Uuid>,
    State(state): State<Arc<Mutex<TestState>>>,
    headers: HeaderMap,
) -> (StatusCode, HeaderMap, Vec<u8>) {
    let token = headers
        .get("if-match")
        .and_then(|value| value.to_str().ok())
        .expect("attachment token header");
    assert_eq!(token, "test-etag");

    let mut state = state.lock().expect("state lock");
    state.attachment_download_requests += 1;
    let mut ciphertext = attachment_by_id(&state.fixture, attachment_id)
        .expect("attachment fixture")
        .ciphertext_bytes
        .clone();
    if state.attachment_size_mismatch {
        ciphertext.pop();
    }
    (StatusCode::OK, HeaderMap::new(), ciphertext)
}

fn run_cli(home: &std::path::Path, api_url: &str, args: &[&str], stdin: Option<&str>) -> CliOutput {
    run_cli_in_dir(home, home, api_url, args, stdin)
}

fn run_cli_exact(home: &std::path::Path, args: &[&str], stdin: Option<&str>) -> CliOutput {
    let mut command = Command::cargo_bin("sealtask").expect("binary");
    command.env("HOME", home);
    command.current_dir(home);
    command.args(args);
    if let Some(stdin) = stdin {
        command.write_stdin(stdin.to_string());
    }

    let output = command.output().expect("run cli");
    CliOutput {
        status: output.status,
        stdout: String::from_utf8(output.stdout).expect("stdout utf8"),
        stderr: String::from_utf8(output.stderr).expect("stderr utf8"),
    }
}

fn run_cli_with_operator_env(
    home: &std::path::Path,
    api_url: &str,
    args: &[&str],
    environment: &[(&str, &str)],
    stdin: Option<&str>,
) -> CliOutput {
    let mut command = Command::cargo_bin("sealtask").expect("binary");
    command.env("HOME", home);
    for name in [
        "SEALTASK_API_URL",
        "SEALTASK_PROFILE",
        "SEALTASK_CONFIG_DIR",
        "SEALTASK_CONNECT_TIMEOUT",
        "SEALTASK_READ_TIMEOUT",
        "SEALTASK_REQUEST_TIMEOUT",
    ] {
        command.env_remove(name);
    }
    for (name, value) in environment {
        command.env(name, value);
    }
    command.current_dir(home);
    command.arg("--api-url").arg(api_url);
    command.args(args);
    if let Some(stdin) = stdin {
        command.write_stdin(stdin.to_string());
    }

    let output = command.output().expect("run cli");
    CliOutput {
        status: output.status,
        stdout: String::from_utf8(output.stdout).expect("stdout utf8"),
        stderr: String::from_utf8(output.stderr).expect("stderr utf8"),
    }
}

fn spawn_cli_process(home: &std::path::Path, api_url: &str, args: &[&str]) -> Child {
    let binary = assert_cmd::cargo::cargo_bin("sealtask");
    let mut command = std::process::Command::new(binary);
    command.env("HOME", home);
    command.current_dir(home);
    command.arg("--api-url").arg(api_url);
    command.args(args);
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    command.spawn().expect("spawn concurrent CLI process")
}

fn spawn_cli_process_with_stdin(
    home: &std::path::Path,
    api_url: &str,
    args: &[&str],
    stdin: &str,
) -> Child {
    let binary = assert_cmd::cargo::cargo_bin("sealtask");
    let mut command = std::process::Command::new(binary);
    command.env("HOME", home);
    command.current_dir(home);
    command.arg("--api-url").arg(api_url);
    command.args(args);
    command.stdin(Stdio::piped());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    let mut child = command.spawn().expect("spawn CLI process with stdin");
    let mut child_stdin = child.stdin.take().expect("CLI stdin pipe");
    child_stdin
        .write_all(stdin.as_bytes())
        .expect("write CLI stdin");
    drop(child_stdin);
    child
}

async fn wait_for_cli_condition(
    child: &mut Child,
    description: &str,
    mut condition: impl FnMut() -> bool,
) {
    let deadline = tokio::time::Instant::now() + CLI_READINESS_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let (stdout, stderr) = take_child_diagnostics(child);
                panic!(
                    "CLI exited with {status} while waiting for {description}\n\
                     bounded stdout: {stdout:?}\n\
                     bounded stderr: {stderr:?}"
                );
            }
            Ok(None) => {}
            Err(error) => {
                let (status, kill_error, stdout, stderr) = terminate_child(child);
                panic!(
                    "could not inspect CLI while waiting for {description}: {error}; \
                     kill error: {kill_error:?}; final status: {status:?}\n\
                     bounded stdout: {stdout:?}\n\
                     bounded stderr: {stderr:?}"
                );
            }
        }
        if condition() {
            return;
        }
        if tokio::time::Instant::now() >= deadline {
            let (status, kill_error, stdout, stderr) = terminate_child(child);
            panic!(
                "timed out after {CLI_READINESS_TIMEOUT:?} waiting for {description}; \
                 kill error: {kill_error:?}; final status: {status:?}\n\
                 bounded stdout: {stdout:?}\n\
                 bounded stderr: {stderr:?}"
            );
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
}

fn terminate_child(
    child: &mut Child,
) -> (
    Option<std::process::ExitStatus>,
    Option<String>,
    String,
    String,
) {
    let kill_error = child.kill().err().map(|error| error.to_string());
    let status = child.wait().ok();
    let (stdout, stderr) = take_child_diagnostics(child);
    (status, kill_error, stdout, stderr)
}

fn take_child_diagnostics(child: &mut Child) -> (String, String) {
    (
        bounded_child_output(child.stdout.take()),
        bounded_child_output(child.stderr.take()),
    )
}

fn bounded_child_output(reader: Option<impl Read>) -> String {
    let Some(reader) = reader else {
        return "<not captured>".to_string();
    };
    let mut bytes = Vec::with_capacity(MAX_CHILD_DIAGNOSTIC_BYTES + 1);
    let mut reader = reader.take((MAX_CHILD_DIAGNOSTIC_BYTES + 1) as u64);
    if let Err(error) = reader.read_to_end(&mut bytes) {
        return format!("<failed to read child output: {error}>");
    }
    let truncated = bytes.len() > MAX_CHILD_DIAGNOSTIC_BYTES;
    bytes.truncate(MAX_CHILD_DIAGNOSTIC_BYTES);
    let mut output = String::from_utf8_lossy(&bytes).into_owned();
    if truncated {
        output.push_str("\n<output truncated>");
    }
    output
}

#[cfg(unix)]
fn signal_process(child: &mut Child, signal: i32) {
    // SAFETY: `child.id()` identifies the live process spawned by this test,
    // and `kill` neither retains nor dereferences any Rust memory.
    let result = unsafe { libc::kill(child.id().cast_signed(), signal) };
    assert_eq!(result, 0, "send signal {signal} to CLI process");
}

async fn wait_for_cli_process(child: Child) -> CliOutput {
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        tokio::task::spawn_blocking(move || child.wait_with_output()),
    )
    .await
    .expect("concurrent CLI process should finish")
    .expect("join concurrent CLI wait")
    .expect("wait for concurrent CLI process");
    CliOutput {
        status: output.status,
        stdout: String::from_utf8(output.stdout).expect("stdout utf8"),
        stderr: String::from_utf8(output.stderr).expect("stderr utf8"),
    }
}

fn run_cli_with_closed_stdout(
    home: &std::path::Path,
    api_url: &str,
    args: &[&str],
    stdin: Option<&str>,
) -> CliOutput {
    let binary = assert_cmd::cargo::cargo_bin("sealtask");
    let mut command = std::process::Command::new(binary);
    command.env("HOME", home);
    command.current_dir(home);
    command.arg("--api-url").arg(api_url);
    command.args(args);
    command.stdin(Stdio::piped());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let mut child = command.spawn().expect("spawn cli");
    drop(child.stdout.take().expect("stdout pipe"));

    if let Some(stdin) = stdin {
        let mut child_stdin = child.stdin.take().expect("stdin pipe");
        child_stdin
            .write_all(stdin.as_bytes())
            .expect("write stdin");
    }

    let output = child.wait_with_output().expect("wait for cli");
    CliOutput {
        status: output.status,
        stdout: String::from_utf8(output.stdout).expect("stdout utf8"),
        stderr: String::from_utf8(output.stderr).expect("stderr utf8"),
    }
}

fn run_cli_with_test_keychain(
    home: &std::path::Path,
    api_url: &str,
    keychain_dir: &std::path::Path,
    args: &[&str],
    stdin: Option<&str>,
) -> CliOutput {
    let mut command = Command::cargo_bin("sealtask").expect("binary");
    command.env("HOME", home);
    command.env("SEALTASK_TEST_KEYCHAIN_DIR", keychain_dir);
    command.current_dir(home);
    command.arg("--api-url").arg(api_url);
    command.args(args);
    if let Some(stdin) = stdin {
        command.write_stdin(stdin.to_string());
    }

    let output = command.output().expect("run cli");
    CliOutput {
        status: output.status,
        stdout: String::from_utf8(output.stdout).expect("stdout utf8"),
        stderr: String::from_utf8(output.stderr).expect("stderr utf8"),
    }
}

fn run_cli_in_dir(
    home: &std::path::Path,
    current_dir: &std::path::Path,
    api_url: &str,
    args: &[&str],
    stdin: Option<&str>,
) -> CliOutput {
    let mut command = Command::cargo_bin("sealtask").expect("binary");
    command.env("HOME", home);
    command.current_dir(current_dir);
    command.arg("--api-url").arg(api_url);
    command.args(args);
    if let Some(stdin) = stdin {
        command.write_stdin(stdin.to_string());
    }

    let output = command.output().expect("run cli");
    CliOutput {
        status: output.status,
        stdout: String::from_utf8(output.stdout).expect("stdout utf8"),
        stderr: String::from_utf8(output.stderr).expect("stderr utf8"),
    }
}

fn seed_credentials(home: &std::path::Path, fixture: &TestFixture, api_url: &str) {
    seed_credentials_with_expiry(
        home,
        fixture,
        api_url,
        Utc::now() + Duration::hours(1),
        Utc::now() + Duration::days(1),
    );
}

fn copy_credentials_to_profile(home: &FsPath, profile: &str) {
    let default_credentials = home.join(".sealtask/credentials.json");
    let profile_dir = home.join(".sealtask/profiles").join(profile);
    std::fs::create_dir_all(&profile_dir).expect("create profile config dir");
    #[cfg(unix)]
    std::fs::set_permissions(&profile_dir, std::fs::Permissions::from_mode(0o700))
        .expect("secure profile config dir");
    std::fs::copy(default_credentials, profile_dir.join("credentials.json"))
        .expect("copy credentials into profile");
}

fn seed_credentials_with_expiry(
    home: &std::path::Path,
    fixture: &TestFixture,
    api_url: &str,
    access_expires_at: DateTime<Utc>,
    refresh_expires_at: DateTime<Utc>,
) {
    let credentials = Credentials {
        api_url: api_url.to_string(),
        access_token: fixture.access_token.clone(),
        refresh_token: fixture.refresh_token.clone(),
        access_expires_at,
        refresh_expires_at,
        user_id: fixture.owner_user_id,
        email: FIXTURE_EMAIL.to_string(),
        data_key_ciphertext: fixture.data_key_ciphertext.clone(),
    };

    let config_dir = home.join(".sealtask");
    std::fs::create_dir_all(&config_dir).expect("create config dir");
    #[cfg(unix)]
    std::fs::set_permissions(&config_dir, std::fs::Permissions::from_mode(0o700))
        .expect("secure config dir");
    let path = config_dir.join("credentials.json");
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(&credentials).expect("serialize creds"),
    )
    .expect("write creds");
    #[cfg(unix)]
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
        .expect("secure credentials");
}

fn authorize(state: &Arc<Mutex<TestState>>, headers: &HeaderMap) {
    let token = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .expect("authorization header");
    let expected = {
        let state = state.lock().expect("state lock");
        format!("Bearer {}", state.current_access_token)
    };
    assert_eq!(token, expected);
}

fn decode_b64(value: &str) -> Vec<u8> {
    STANDARD_NO_PAD
        .decode(value)
        .or_else(|_| base64::engine::general_purpose::STANDARD.decode(value))
        .expect("decode base64")
}

fn make_opaque_account_fixture(email: &str, password: &str) -> OpaqueAccountFixture {
    const SERVER_ID: &[u8] = b"worklist.api";

    let mut rng = OsRng;
    let setup = ServerSetup::<ClientCipherSuite>::new(&mut rng);
    let registration =
        ClientRegistration::<ClientCipherSuite>::start(&mut rng, password.as_bytes())
            .expect("start fixture OPAQUE registration");
    let registration_response = ServerRegistration::<ClientCipherSuite>::start(
        &setup,
        registration.message,
        email.as_bytes(),
    )
    .expect("start fixture OPAQUE server registration");
    let identifiers = Identifiers {
        client: Some(email.as_bytes()),
        server: Some(SERVER_ID),
    };
    let mut registration_finish = registration
        .state
        .finish(
            &mut rng,
            password.as_bytes(),
            RegistrationResponse::<ClientCipherSuite>::deserialize(
                &registration_response.message.serialize(),
            )
            .expect("deserialize fixture OPAQUE registration response"),
            ClientRegistrationFinishParameters::new(identifiers, None),
        )
        .expect("finish fixture OPAQUE client registration");
    let mut export_key = [0u8; OPAQUE_EXPORT_KEY_BYTES];
    export_key.copy_from_slice(registration_finish.export_key.as_slice());
    registration_finish.export_key.as_mut_slice().zeroize();
    let password_file =
        ServerRegistration::<ClientCipherSuite>::finish(registration_finish.message);

    OpaqueAccountFixture {
        setup: setup.serialize().to_vec(),
        password_file: password_file.serialize().to_vec(),
        export_key,
    }
}

fn encode_opaque_data_key_ciphertext(
    export_key: &[u8; OPAQUE_EXPORT_KEY_BYTES],
    data_key: &SymmetricKey,
) -> sealtask_client_core::PublicResult<String> {
    let mut wrapping_key_bytes = [0u8; 32];
    Hkdf::<Sha256>::new(None, export_key)
        .expand(USER_DATA_KEY_OPAQUE_WRAP_INFO, &mut wrapping_key_bytes)
        .map_err(|err| {
            sealtask_client_core::PublicError::crypto(format!(
                "fixture OPAQUE export-key HKDF failed: {err}"
            ))
        })?;
    let wrapping_key = SymmetricKey::new(wrapping_key_bytes);
    wrapping_key_bytes.zeroize();
    let ciphertext = StrongBoxKeyRing::new(wrapping_key)
        .strong_box()
        .encrypt(data_key.as_bytes(), USER_DATA_KEY_OPAQUE_CONTEXT)
        .map_err(|err| {
            sealtask_client_core::PublicError::crypto(format!(
                "failed to encrypt fixture OPAQUE data key: {err}"
            ))
        })?;
    let payload = SealedPayload {
        version: 2,
        ciphertext,
    }
    .to_bytes()?;
    Ok(STANDARD_NO_PAD.encode(payload))
}

fn encode_data_key_ciphertext(
    password: &str,
    salt: &[u8; 32],
    data_key: &SymmetricKey,
) -> sealtask_client_core::PublicResult<String> {
    let wrapping_key = sealtask_client_crypto::KeyDerivationService::new()
        .derive_master_key(password.as_bytes(), salt)?;
    let strong_box = StrongBoxKeyRing::new(wrapping_key).strong_box();
    let sealed = strong_box
        .encrypt(data_key.as_bytes(), USER_DATA_KEY_CONTEXT)
        .expect("seal data key");
    let payload = SealedPayload::new([salt.as_slice(), sealed.as_slice()].concat()).to_bytes()?;
    Ok(STANDARD_NO_PAD.encode(payload))
}

fn encode_membership_key_ciphertext(
    data_key: &SymmetricKey,
    list_key: &SymmetricKey,
) -> sealtask_client_core::PublicResult<String> {
    let strong_box = StrongBoxKeyRing::new(data_key.clone()).strong_box();
    let sealed = strong_box
        .encrypt(list_key.as_bytes(), WORK_LIST_MEMBERSHIP_CONTEXT)
        .expect("seal membership key");
    let payload = SealedPayload::new(sealed).to_bytes()?;
    Ok(STANDARD_NO_PAD.encode(payload))
}

fn encode_work_list_payload_ciphertext(
    list_key: &SymmetricKey,
    first_section_id: Uuid,
    done_section_id: Uuid,
) -> sealtask_client_core::PublicResult<String> {
    let plaintext = serialize_to_cbor(&json!({
        "kind": "work_list",
        "version": 1,
        "body": {
            "title": "Fixture Work List",
            "description": null,
            "sections": [
                {
                    "id": first_section_id.as_bytes(),
                    "name": "Planning"
                },
                {
                    "id": done_section_id.as_bytes(),
                    "name": "Done"
                }
            ],
            "client_meta": {
                "web.view": {
                    "layout": "kanban"
                }
            }
        }
    }))?;
    let strong_box = StrongBoxKeyRing::new(list_key.clone()).strong_box();
    let sealed = strong_box
        .encrypt(plaintext, WORK_LIST_PAYLOAD_CONTEXT)
        .expect("seal work list payload");
    let payload = SealedPayload::new(sealed).to_bytes()?;
    Ok(STANDARD_NO_PAD.encode(payload))
}

fn task_response_json(state: &TestState) -> serde_json::Value {
    json!({
        "id": state.fixture.task_id,
        "workListId": state.fixture.work_list_id,
        "createdByMembershipId": state.fixture.membership_id,
        "titleCiphertext": state.fixture.task_title_ciphertext,
        "payloadCiphertext": task_payload_ciphertext(state),
        "sectionId": state.task_section_id,
        "priority": null,
        "position": "a",
        "dueAt": null,
        "startAt": null,
        "completedAt": state.task_completed_at,
        "archivedAt": state.task_archived_at,
        "isCompleted": state.task_is_completed,
        "recurrenceId": null,
        "recurrenceSchedule": null,
        "recurrenceIteration": null,
        "materializedAt": null,
        "createdAt": Utc::now(),
        "updatedAt": state.task_updated_at,
        "commentCount": 1,
        "delegations": [
            {
                "id": Uuid::now_v7(),
                "taskId": state.fixture.task_id,
                "membershipId": state.fixture.membership_id,
                "role": "assigned",
                "status": "pending",
                "noteCiphertext": null,
                "createdAt": Utc::now(),
                "updatedAt": Utc::now()
            }
        ]
    })
}

fn note_response_json(state: &TestState, note: &StoredNote) -> Value {
    json!({
        "id": note.id,
        "workListId": state.fixture.work_list_id,
        "createdByMembershipId": state.fixture.membership_id,
        "titleCiphertext": note.title_ciphertext,
        "legacyCborFields": [],
        "payloadCiphertext": note.payload_ciphertext,
        "isPrivate": note.is_private,
        "noteKeyCiphertext": note.note_key_ciphertext,
        "createdAt": note.created_at,
        "updatedAt": note.updated_at,
    })
}

fn validate_note_ciphertexts(
    fixture: &TestFixture,
    is_private: bool,
    note_key_ciphertext: Option<&str>,
    title_ciphertext: &str,
    title_proof: &str,
    payload_ciphertext: &str,
    payload_proof: &str,
) {
    let title_bytes = decode_b64(title_ciphertext);
    let payload_bytes = decode_b64(payload_ciphertext);
    assert_eq!(
        compute_payload_proof(&title_bytes, &fixture.binding_key).expect("note title proof"),
        title_proof
    );
    assert_eq!(
        compute_payload_proof(&payload_bytes, &fixture.binding_key).expect("note payload proof"),
        payload_proof
    );
    let note_key = if is_private {
        let wrapped_key = decode_b64(note_key_ciphertext.expect("private note key ciphertext"));
        decrypt_note_key(&wrapped_key, &fixture.data_key).expect("decrypt private note key")
    } else {
        assert!(note_key_ciphertext.is_none());
        fixture.list_key.clone()
    };
    let envelope = decrypt_note_payload(&note_key, &payload_bytes).expect("decrypt note payload");
    assert_eq!(envelope.kind, "note");
    let title = decrypt_encrypted_text_value(&title_bytes, &note_key, NOTE_TITLE_CONTEXT)
        .expect("decrypt note title");
    assert_eq!(title, envelope.body.title);
}

fn work_list_summary_json(state: &TestState) -> serde_json::Value {
    json!({
        "id": state.fixture.work_list_id,
        "ownerUserId": state.fixture.owner_user_id,
        "workspaceId": state.fixture.workspace_id,
        "titleCiphertext": seal_text_value("Fixture Work List").expect("title").base64,
        "descriptionCiphertext": null,
        "payloadCiphertext": work_list_payload_ciphertext(state),
        "timezone": "UTC",
        "sectionSnapshots": section_snapshots_json(state),
        "archivedAt": state.work_list_archived_at,
        "createdAt": Utc::now(),
        "updatedAt": Utc::now(),
        "membership": membership_json(state)
    })
}

fn section_snapshots_json(state: &TestState) -> Value {
    let mut sections = vec![json!({
        "id": state.fixture.first_section_id,
        "position": 0,
        "autoArchiveEnabled": false,
        "autoArchiveAfterDays": null
    })];
    if !state.single_section {
        sections.push(json!({
            "id": state.fixture.done_section_id,
            "position": 1,
            "autoArchiveEnabled": false,
            "autoArchiveAfterDays": null
        }));
    }
    Value::Array(sections)
}

fn membership_json(state: &TestState) -> serde_json::Value {
    json!({
        "id": state.fixture.membership_id,
        "userId": state.fixture.owner_user_id,
        "userEmail": "fixture@example.test",
        "userName": "Fixture",
        "userAvatarColor": "#111111",
        "role": "owner",
        "status": "active",
        "workListKeyCiphertext": state.fixture.work_list_key_ciphertext,
        "recipientCiphertext": null,
        "invitePackageCiphertext": null,
        "saltMember": null,
        "expiresAt": null,
        "joinedAt": Utc::now(),
        "payloadBindingKey": null
    })
}

fn work_list_payload_ciphertext(state: &TestState) -> String {
    if state.invalid_work_list_payload {
        "invalid-work-list-payload".to_string()
    } else {
        state.fixture.work_list_payload_ciphertext.clone()
    }
}

fn task_payload_ciphertext(state: &TestState) -> String {
    if state.invalid_task_payload {
        return "invalid-task-payload".to_string();
    }

    if state.invalid_task_attachment_metadata {
        let mut body = state.current_task_body.clone();
        body.attachments = Some(vec![json_value_to_flexible(json!({
            "id": state.fixture.text_attachment.id.to_string()
        }))]);
        return encrypt_task_payload(
            &build_task_payload_envelope(body, 1),
            &state.fixture.list_key,
        )
        .expect("task payload with invalid attachment metadata")
        .base64;
    }

    encrypt_task_payload(
        &build_task_payload_envelope(state.current_task_body.clone(), 1),
        &state.fixture.list_key,
    )
    .expect("current task payload")
    .base64
}

fn task_attachment_ids(body: &TaskPayloadBody) -> Vec<Uuid> {
    body.attachments
        .as_ref()
        .into_iter()
        .flatten()
        .map(|attachment| {
            let json = flexible_value_to_json(attachment.clone());
            Uuid::parse_str(
                json.get("id")
                    .and_then(Value::as_str)
                    .expect("attachment id string"),
            )
            .expect("attachment UUID")
        })
        .collect()
}

fn unique_uuid_prefix(target: Uuid, candidates: &[Uuid]) -> String {
    let target = target.simple().to_string();
    (8..=target.len())
        .find(|length| {
            candidates
                .iter()
                .filter(|candidate| {
                    candidate
                        .simple()
                        .to_string()
                        .starts_with(&target[..*length])
                })
                .count()
                == 1
        })
        .map_or(target.clone(), |length| target[..length].to_string())
}

fn comment_body_ciphertext(state: &TestState) -> String {
    if state.invalid_comment_payload {
        return "invalid-comment-payload".to_string();
    }

    if state.invalid_comment_attachment_metadata {
        let mut body = state.fixture.existing_comment_body.clone();
        body.attachments = Some(vec![json_value_to_flexible(json!({
            "id": state.fixture.text_attachment.id.to_string()
        }))]);
        return encrypt_comment_payload(
            &build_comment_payload_envelope(body, 1),
            &state.fixture.list_key,
        )
        .expect("comment payload with invalid attachment metadata")
        .base64;
    }

    state.fixture.comment_body_ciphertext.clone()
}

fn attachment_by_id(fixture: &TestFixture, attachment_id: Uuid) -> Option<&TestAttachmentFixture> {
    [
        &fixture.text_attachment,
        &fixture.docx_attachment,
        &fixture.binary_attachment,
        &fixture.hostile_attachment,
    ]
    .into_iter()
    .find(|attachment| attachment.id == attachment_id)
}

fn make_attachment_fixture(
    list_key: &SymmetricKey,
    attachment_id: Uuid,
    file_name: &str,
    content_type: &str,
    plaintext_bytes: Vec<u8>,
    file_key_bytes: [u8; 32],
) -> TestAttachmentFixture {
    let file_key = SymmetricKey::new(file_key_bytes);
    let ciphertext_bytes =
        encrypt_attachment_ciphertext(&file_key, &plaintext_bytes).expect("attachment ciphertext");
    let blob_key =
        encode_attachment_blob_key(list_key, attachment_id, &file_key, &ciphertext_bytes)
            .expect("attachment blob key");
    TestAttachmentFixture {
        id: attachment_id,
        file_name: file_name.to_string(),
        content_type: content_type.to_string(),
        plaintext_bytes,
        ciphertext_bytes,
        blob_key,
    }
}

fn docx_fixture_bytes() -> Vec<u8> {
    const DOCX_FIXTURE_BASE64: &str = "UEsDBBQAAAAIAOp8kVzXeYTq8QAAALgBAAATAAAAW0NvbnRlbnRfVHlwZXNdLnhtbH2QzU7DMBCE730Ky9cqccoBIZSkB36OwKE8wMreJFb9J69b2rdn00KREOVozXwz62nXB+/EHjPZGDq5qhspMOhobBg7+b55ru6koALBgIsBO3lEkut+0W6OCUkwHKiTUynpXinSE3qgOiYMrAwxeyj8zKNKoLcworppmlulYygYSlXmDNkvhGgfcYCdK+LpwMr5loyOpHg4e+e6TkJKzmoorKt9ML+Kqq+SmsmThyabaMkGqa6VzOL1jh/0lSfK1qB4g1xewLNRfcRslIl65xmu/0/649o4DFbjhZ/TUo4aiXh77+qL4sGG71+06jR8/wlQSwMEFAAAAAgA6nyRXCAbhuqyAAAALgEAAAsAAABfcmVscy8ucmVsc43Puw6CMBQG4J2naM4uBQdjDIXFmLAafICmPZRGeklbL7y9HRzEODie23fyN93TzOSOIWpnGdRlBQStcFJbxeAynDZ7IDFxK/nsLDJYMELXFs0ZZ57yTZy0jyQjNjKYUvIHSqOY0PBYOo82T0YXDE+5DIp6Lq5cId1W1Y6GTwPagpAVS3rJIPSyBjIsHv/h3ThqgUcnbgZt+vHlayPLPChMDB4uSCrf7TKzQHNKuorZvgBQSwMEFAAAAAgA6nyRXDbicKixAAAADAEAABEAAAB3b3JkL2RvY3VtZW50LnhtbG2PMQ+CMBCFd35F012KDsYQKIPGuLlo4lrpKST0rmmryL+3xbixfHkv9/Lurmo+ZmBvcL4nrPk6LzgDbEn3+Kz59XJc7TjzQaFWAyHUfALPG5lVY6mpfRnAwGID+nKseReCLYXwbQdG+ZwsYJw9yBkVonVPMZLT1lEL3scFZhCbotgKo3rkMmMstt5JT0nOxsoIlxDkCVQ6qhLJJLqZdjF8OO9vLFUtxpP47Unq/4f8AlBLAQIUAxQAAAAIAOp8kVzXeYTq8QAAALgBAAATAAAAAAAAAAAAAACAAQAAAABbQ29udGVudF9UeXBlc10ueG1sUEsBAhQDFAAAAAgA6nyRXCAbhuqyAAAALgEAAAsAAAAAAAAAAAAAAIABIgEAAF9yZWxzLy5yZWxzUEsBAhQDFAAAAAgA6nyRXDbicKixAAAADAEAABEAAAAAAAAAAAAAAIAB/QEAAHdvcmQvZG9jdW1lbnQueG1sUEsFBgAAAAADAAMAuQAAAN0CAAAAAA==";

    base64::engine::general_purpose::STANDARD
        .decode(DOCX_FIXTURE_BASE64)
        .expect("decode docx fixture")
}

fn encrypt_attachment_ciphertext(
    file_key: &SymmetricKey,
    plaintext_bytes: &[u8],
) -> sealtask_client_core::PublicResult<Vec<u8>> {
    StrongBoxKeyRing::new(file_key.clone())
        .strong_box()
        .encrypt(plaintext_bytes, ATTACHMENT_BLOB_CONTEXT)
        .map_err(|err| {
            sealtask_client_core::PublicError::crypto(format!(
                "failed to seal attachment bytes: {err}"
            ))
        })
}

fn encode_attachment_blob_key(
    list_key: &SymmetricKey,
    _attachment_id: Uuid,
    file_key: &SymmetricKey,
    ciphertext_bytes: &[u8],
) -> sealtask_client_core::PublicResult<Vec<u8>> {
    let blob_ref = AttachmentBlobRef {
        version: ATTACHMENT_BLOB_REF_VERSION,
        ciphertext_bytes: u64::try_from(ciphertext_bytes.len()).expect("ciphertext length"),
        file_key: file_key.as_bytes().to_vec(),
        enc_context: ATTACHMENT_BLOB_CONTEXT_LABEL.to_string(),
    };
    encode_production_attachment_blob_key(list_key, &blob_ref)
}

fn attachment_payload_value(attachment: &TestAttachmentFixture) -> FlexibleValue {
    FlexibleValue::Map(vec![
        (
            FlexibleValue::Text("id".to_string()),
            FlexibleValue::Text(attachment.id.to_string()),
        ),
        (
            FlexibleValue::Text("file_name".to_string()),
            FlexibleValue::Text(attachment.file_name.clone()),
        ),
        (
            FlexibleValue::Text("content_type".to_string()),
            FlexibleValue::Text(attachment.content_type.clone()),
        ),
        (
            FlexibleValue::Text("size_bytes".to_string()),
            FlexibleValue::Integer(
                u64::try_from(attachment.plaintext_bytes.len())
                    .expect("plaintext length")
                    .into(),
            ),
        ),
        (
            FlexibleValue::Text("blob_key".to_string()),
            FlexibleValue::Bytes(attachment.blob_key.clone()),
        ),
    ])
}

fn parse_stdout_json(stdout: &str) -> Value {
    serde_json::from_str(stdout).expect("stdout JSON")
}

fn parse_stderr_json(stderr: &str) -> Value {
    serde_json::from_str(stderr).expect("stderr JSON")
}

fn telemetry_invocation_id(stderr: &str) -> Uuid {
    let invocation_ids = stderr
        .split_whitespace()
        .filter_map(|field| field.strip_prefix("invocation_id="))
        .collect::<HashSet<_>>();
    assert_eq!(
        invocation_ids.len(),
        1,
        "telemetry lines were not correlated: {stderr}"
    );
    invocation_ids
        .into_iter()
        .next()
        .expect("one invocation id")
        .parse()
        .expect("telemetry invocation UUID")
}

fn assert_json_error_message(stderr: &str, expected_message: &str) {
    let error_json = parse_stderr_json(stderr);
    assert_eq!(error_json["error"]["code"], "validation");
    assert_eq!(error_json["error"]["message"], expected_message);
}

fn assert_json_error_contains(stderr: &str, expected_fragment: &str) {
    let error_json = parse_stderr_json(stderr);
    assert_eq!(error_json["error"]["code"], "validation");
    assert!(
        error_json["error"]["message"]
            .as_str()
            .expect("error message")
            .contains(expected_fragment),
        "unexpected stderr: {stderr}"
    );
}

fn assert_json_warning_contains(stderr: &str, expected_code: &str, expected_fragment: &str) {
    let stderr_json = parse_stderr_json(stderr);
    assert_eq!(stderr_json["warnings"][0]["code"], expected_code);
    assert!(
        stderr_json["warnings"][0]["message"]
            .as_str()
            .expect("warning message")
            .contains(expected_fragment),
        "unexpected stderr: {stderr}"
    );
}

fn assert_json_password_stdin_required(args: &[&str], expected_message: &str) {
    let home = TempDir::new().expect("temp home");
    let output = run_cli(home.path(), "https://sealtask.com", args, None);

    assert!(!output.status.success(), "command unexpectedly succeeded");
    assert!(
        output.stdout.is_empty(),
        "unexpected stdout: {}",
        output.stdout
    );
    assert_json_error_message(&output.stderr, expected_message);
}

#[cfg(unix)]
fn spawn_hanging_unlock_daemon(
    socket_path: &FsPath,
) -> (std::sync::mpsc::Sender<()>, std::thread::JoinHandle<()>) {
    use std::os::unix::net::UnixListener;

    if socket_path.exists() {
        std::fs::remove_file(socket_path).expect("remove stale fake daemon socket");
    }

    let listener = UnixListener::bind(socket_path).expect("bind fake daemon socket");
    let socket_path = socket_path.to_path_buf();
    let (release_tx, release_rx) = std::sync::mpsc::channel();

    let thread = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept fake daemon connection");
        let mut request = Vec::new();
        stream
            .read_to_end(&mut request)
            .expect("read fake daemon request");
        release_rx.recv().expect("release fake daemon response");
        drop(stream);
        drop(listener);
        let _ = std::fs::remove_file(socket_path);
    });
    (release_tx, thread)
}

fn replace_stored_test_keychain_secret_with_directory(keychain_dir: &FsPath) {
    let secret_path = std::fs::read_dir(keychain_dir)
        .expect("list keychain dir")
        .map(|entry| entry.expect("dir entry").path())
        .next()
        .expect("stored secret path");
    std::fs::remove_file(&secret_path).expect("remove stored secret");
    std::fs::create_dir(&secret_path).expect("replace stored secret with directory");
}

fn read_stored_test_keychain_secret(keychain_dir: &FsPath) -> Vec<u8> {
    let secret_path = std::fs::read_dir(keychain_dir)
        .expect("list keychain dir")
        .map(|entry| entry.expect("dir entry").path())
        .next()
        .expect("stored secret path");
    std::fs::read(secret_path).expect("read test keychain secret")
}

struct CliOutput {
    status: std::process::ExitStatus,
    stdout: String,
    stderr: String,
}

fn write_json_file(dir: &FsPath, name: &str, value: &serde_json::Value) -> std::path::PathBuf {
    let path = dir.join(name);
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(value).expect("serialize json"),
    )
    .expect("write json file");
    path
}
