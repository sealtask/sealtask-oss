use std::future::Future;

use chrono::{Duration, Utc};
use sealtask_client_api::note_transport::EncodedNoteRequest;
use sealtask_client_api::{
    ApiCancellationToken, CompleteAttachmentUploadRequest, CreateNoteRequest, CreateTaskRequest,
    PublicApiClient,
};
use sealtask_client_auth::Credentials;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::oneshot;
use uuid::Uuid;

struct DelayedResponse {
    api_url: String,
    headers_received: oneshot::Receiver<()>,
    release_body: oneshot::Sender<()>,
    server: tokio::task::JoinHandle<()>,
}

#[tokio::test]
async fn durable_mutation_signal_matches_public_request_boundaries() {
    observe_pending_response(true, |api_url, cancellation| async move {
        let mut client = authenticated_client(&api_url, cancellation);
        client
            .create_task(Uuid::now_v7(), &task_create_request())
            .await
    })
    .await;

    observe_pending_response(false, |api_url, cancellation| async move {
        let mut client = authenticated_client(&api_url, cancellation);
        client.start_opaque_export_key("opaque-state").await
    })
    .await;

    observe_pending_response(false, |api_url, cancellation| async move {
        let mut client = authenticated_client(&api_url, cancellation);
        client.issue_project_sse_token(Uuid::now_v7()).await
    })
    .await;

    observe_pending_response(true, |api_url, cancellation| async move {
        let mut client = authenticated_client(&api_url, cancellation);
        client
            .complete_attachment_upload(
                Uuid::now_v7(),
                Uuid::now_v7(),
                &CompleteAttachmentUploadRequest {
                    ciphertext_bytes: 42,
                },
            )
            .await
    })
    .await;

    observe_pending_response(true, |api_url, cancellation| async move {
        let mut client = authenticated_client(&api_url, cancellation);
        let encoded =
            EncodedNoteRequest::encode(&note_create_request()).expect("encode note request");
        client.create_note_encoded(Uuid::now_v7(), encoded).await
    })
    .await;
}

#[tokio::test]
async fn bounded_mutation_guard_survives_until_the_raw_response_is_consumed() {
    let DelayedResponse {
        api_url,
        headers_received,
        release_body,
        server,
    } = delayed_response().await;
    let cancellation = ApiCancellationToken::new();
    let cancellation_for_request = cancellation.clone();
    let (response_sender, response_receiver) = oneshot::channel();
    let request = tokio::spawn(async move {
        let mut client = authenticated_client(&api_url, cancellation_for_request);
        let encoded =
            EncodedNoteRequest::encode(&note_create_request()).expect("encode note request");
        let response = client.create_note_encoded(Uuid::now_v7(), encoded).await;
        response_sender
            .send(response)
            .map_err(|_| "response receiver dropped")
    });

    headers_received
        .await
        .expect("server observed request headers");
    assert!(cancellation.mutation_request_in_flight());
    release_body.send(()).expect("release response body");
    let response = response_receiver
        .await
        .expect("receive raw response")
        .expect("successful raw response");
    request
        .await
        .expect("request task")
        .expect("deliver response");
    server.await.expect("server task");

    assert!(
        cancellation.mutation_request_in_flight(),
        "the raw response owns the mutation boundary through typed decoding"
    );
    drop(response);
    assert!(!cancellation.mutation_request_in_flight());
}

async fn observe_pending_response<F, Fut, Output>(expected_mutation: bool, request: F)
where
    F: FnOnce(String, ApiCancellationToken) -> Fut,
    Fut: Future<Output = Output> + Send + 'static,
    Output: Send + 'static,
{
    let cancellation = ApiCancellationToken::new();
    let delayed = delayed_response().await;
    let request = tokio::spawn(request(delayed.api_url, cancellation.clone()));

    delayed
        .headers_received
        .await
        .expect("server observed request headers");
    if expected_mutation {
        assert!(
            cancellation.mutation_request_in_flight(),
            "durable mutation must remain guarded while its response body is pending"
        );
    } else {
        assert!(
            !cancellation.mutation_request_in_flight(),
            "control-plane no-replay request must not mark a durable mutation in flight"
        );
    }

    delayed
        .release_body
        .send(())
        .expect("request still waits for the response body");
    let _ = request.await.expect("request task");
    delayed.server.await.expect("server task");
    assert!(
        !cancellation.mutation_request_in_flight(),
        "logical request completion must release its mutation guard"
    );
}

async fn delayed_response() -> DelayedResponse {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let api_url = format!("http://{}", listener.local_addr().expect("address"));
    let (headers_sender, headers_received) = oneshot::channel();
    let (release_body, body_release) = oneshot::channel();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("connection");
        read_request(&mut stream).await;
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n",
            )
            .await
            .expect("response headers");
        headers_sender.send(()).expect("request observer");
        body_release.await.expect("response body release");
        stream.write_all(b"{}").await.expect("response body");
    });
    DelayedResponse {
        api_url,
        headers_received,
        release_body,
        server,
    }
}

async fn read_request(stream: &mut tokio::net::TcpStream) {
    let mut request = Vec::new();
    let mut buffer = [0_u8; 1_024];
    let header_end = loop {
        if let Some(position) = request.windows(4).position(|window| window == b"\r\n\r\n") {
            break position + 4;
        }
        let read = stream.read(&mut buffer).await.expect("request headers");
        assert_ne!(read, 0, "request ended before its headers");
        request.extend_from_slice(&buffer[..read]);
    };
    let headers = std::str::from_utf8(&request[..header_end]).expect("UTF-8 request headers");
    let content_length = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim())
        })
        .map(|value| value.parse::<usize>().expect("numeric content length"))
        .unwrap_or(0);
    while request.len() < header_end + content_length {
        let read = stream.read(&mut buffer).await.expect("request body");
        assert_ne!(read, 0, "request body ended early");
        request.extend_from_slice(&buffer[..read]);
    }
}

fn authenticated_client(api_url: &str, cancellation: ApiCancellationToken) -> PublicApiClient {
    PublicApiClient::with_credentials(
        api_url,
        Credentials {
            api_url: api_url.to_string(),
            access_token: "access-token".to_string(),
            refresh_token: "refresh-token".to_string(),
            access_expires_at: Utc::now() + Duration::hours(1),
            refresh_expires_at: Utc::now() + Duration::hours(2),
            user_id: Uuid::now_v7(),
            email: "operator@example.com".to_string(),
            data_key_ciphertext: "unused".to_string(),
        },
    )
    .expect("authenticated client")
    .with_cancellation_token(cancellation)
}

fn task_create_request() -> CreateTaskRequest {
    CreateTaskRequest {
        title_ciphertext: "title".to_string(),
        title_ciphertext_proof: "title-proof".to_string(),
        payload_ciphertext: "payload".to_string(),
        payload_ciphertext_proof: "payload-proof".to_string(),
        attachment_ids: Vec::new(),
        priority: None,
        due_at: None,
        start_at: None,
        section_id: None,
        idempotency_key: Some("stable-key".to_string()),
        idempotency_commitment: Some("stable-commitment".to_string()),
    }
}

fn note_create_request() -> CreateNoteRequest {
    CreateNoteRequest {
        idempotency_key: "stable-note-key".to_string(),
        idempotency_commitment: "stable-note-commitment".to_string(),
        title_ciphertext: "title".to_string(),
        title_ciphertext_proof: "title-proof".to_string(),
        payload_ciphertext: "payload".to_string(),
        payload_ciphertext_proof: "payload-proof".to_string(),
        is_private: false,
        note_key_ciphertext: None,
        audit_patch: None,
    }
}
