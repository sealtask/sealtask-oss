use crate::client::RuntimeClient;
use crate::models::{
    AgentAttachment, AgentTaskSummary, DownloadedAttachment, ReadableAttachment,
    ReadableAttachmentContentFormat, ReadableAttachmentSourceKind,
};
use crate::projections::read_error_to_public_error;
use std::io::{self, Cursor, Read};
use std::path::Path;
use uuid::Uuid;
use worklist_client_api::DownloadAttachmentResponse;
use worklist_client_core::{PublicError, PublicResult};
use worklist_client_crypto::{
    AttachmentBlobRef, decode_attachment_blob_key, decrypt_attachment_bytes,
};

const DOCX_CONTENT_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
const MAX_DOCX_ARCHIVE_ENTRIES: usize = 2_048;
const MAX_DOCX_ENTRY_UNCOMPRESSED_BYTES: u64 = 16 * 1024 * 1024;
const MAX_DOCX_TOTAL_UNCOMPRESSED_BYTES: u64 = 32 * 1024 * 1024;
const MAX_DOCX_COMPRESSION_RATIO: u64 = 200;
const MAX_DOCX_RENDERED_MARKDOWN_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug)]
struct ResolvedTaskAttachmentDownload {
    attachment: AgentAttachment,
    blob_ref: AttachmentBlobRef,
    download: DownloadAttachmentResponse,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum AttachmentReadStrategy {
    Utf8Text,
    DocxMarkdown,
    Unsupported,
}

impl AgentAttachment {
    fn blob_key(&self) -> &[u8] {
        &self.blob_key
    }

    #[must_use]
    pub(crate) fn read_strategy(&self) -> AttachmentReadStrategy {
        if content_type_is_docx(&self.content_type) {
            return AttachmentReadStrategy::DocxMarkdown;
        }

        if content_type_is_textual(&self.content_type) {
            return AttachmentReadStrategy::Utf8Text;
        }

        if file_extension_is_docx(&self.file_name) {
            return AttachmentReadStrategy::DocxMarkdown;
        }

        if file_extension_is_textual(&self.file_name) {
            return AttachmentReadStrategy::Utf8Text;
        }

        AttachmentReadStrategy::Unsupported
    }
}

impl RuntimeClient {
    pub async fn read_task_attachment(
        &self,
        work_list_id: Uuid,
        task_id: Uuid,
        attachment_id: Uuid,
        password_stdin: bool,
    ) -> PublicResult<ReadableAttachment> {
        let resolved = self
            .resolve_task_attachment_download(work_list_id, task_id, attachment_id, password_stdin)
            .await?;
        let read_strategy = resolved.attachment.read_strategy();
        if let AttachmentReadStrategy::Unsupported = read_strategy {
            return Err(unsupported_attachment_read_error(
                &resolved.attachment.file_name,
            ));
        }
        let DownloadedAttachment { attachment, bytes } =
            download_and_decrypt_attachment(self.http_client(), resolved).await?;
        build_readable_attachment_async(attachment, bytes, read_strategy).await
    }

    pub async fn download_task_attachment(
        &self,
        work_list_id: Uuid,
        task_id: Uuid,
        attachment_id: Uuid,
        password_stdin: bool,
    ) -> PublicResult<DownloadedAttachment> {
        let resolved = self
            .resolve_task_attachment_download(work_list_id, task_id, attachment_id, password_stdin)
            .await?;
        download_and_decrypt_attachment(self.http_client(), resolved).await
    }

    async fn resolve_task_attachment_download(
        &self,
        work_list_id: Uuid,
        task_id: Uuid,
        attachment_id: Uuid,
        password_stdin: bool,
    ) -> PublicResult<ResolvedTaskAttachmentDownload> {
        let (mut client, context) = self
            .load_work_list_context(
                work_list_id,
                password_stdin,
                "Password required to decrypt attachment data.",
            )
            .await?;
        let list_key = self.require_work_list_key(&context)?;
        let task_detail = client.get_task(work_list_id, task_id).await?;
        let task = self.project_task_summary(task_detail.task, Some(&context));
        let attachment = find_task_attachment(&task, attachment_id)?;
        let blob_ref =
            decode_attachment_blob_key(list_key, attachment.blob_key()).map_err(|err| {
                PublicError::validation(format!("failed to decode attachment blob key: {err}"))
            })?;
        let download = client
            .get_attachment_download(work_list_id, attachment_id)
            .await?;
        Ok(ResolvedTaskAttachmentDownload {
            attachment,
            blob_ref,
            download,
        })
    }
}

fn find_task_attachment(
    task: &AgentTaskSummary,
    attachment_id: Uuid,
) -> PublicResult<AgentAttachment> {
    let attachments = match task.attachments.as_ref() {
        Some(attachments) => attachments,
        None if task.read_error.is_some() => {
            return Err(read_error_to_public_error(
                task.read_error.as_ref(),
                "failed to read task attachments",
            ));
        }
        None => {
            return Err(PublicError::validation("task does not include attachments"));
        }
    };

    attachments
        .iter()
        .find(|attachment| attachment.id == attachment_id)
        .cloned()
        .ok_or_else(|| PublicError::validation(format!("attachment {attachment_id} not found")))
}

async fn download_and_decrypt_attachment(
    http_client: &reqwest::Client,
    resolved: ResolvedTaskAttachmentDownload,
) -> PublicResult<DownloadedAttachment> {
    let ResolvedTaskAttachmentDownload {
        attachment,
        blob_ref,
        download,
    } = resolved;
    let response = send_presigned_attachment_download(http_client, &download).await?;
    let ciphertext =
        read_attachment_ciphertext(response, &attachment.file_name, blob_ref.ciphertext_bytes)
            .await?;
    let bytes =
        decrypt_attachment_bytes(&ciphertext, &blob_ref.file_key, Some(&blob_ref.enc_context))?;
    Ok(DownloadedAttachment { attachment, bytes })
}

async fn read_attachment_ciphertext(
    mut response: reqwest::Response,
    file_name: &str,
    expected_bytes: u64,
) -> PublicResult<Vec<u8>> {
    if let Some(content_length) = response.content_length()
        && content_length != expected_bytes
    {
        return Err(attachment_size_mismatch_error(
            file_name,
            expected_bytes,
            content_length,
        ));
    }

    let expected_len = usize::try_from(expected_bytes).map_err(|_| {
        PublicError::validation(format!(
            "attachment '{file_name}' is too large for this platform"
        ))
    })?;
    let mut ciphertext = Vec::with_capacity(expected_len.min(64 * 1024));
    while let Some(chunk) = response.chunk().await.map_err(|err| {
        PublicError::unexpected(format!(
            "failed to read attachment ciphertext: {}",
            err.without_url()
        ))
    })? {
        let received_len = ciphertext
            .len()
            .checked_add(chunk.len())
            .ok_or_else(|| PublicError::validation("attachment download size overflow"))?;
        if received_len > expected_len {
            return Err(attachment_size_mismatch_error(
                file_name,
                expected_bytes,
                received_len as u64,
            ));
        }
        ciphertext.extend_from_slice(&chunk);
    }

    if ciphertext.len() != expected_len {
        return Err(attachment_size_mismatch_error(
            file_name,
            expected_bytes,
            ciphertext.len() as u64,
        ));
    }
    Ok(ciphertext)
}

fn attachment_size_mismatch_error(
    file_name: &str,
    expected_bytes: u64,
    received_bytes: u64,
) -> PublicError {
    PublicError::validation(format!(
        "attachment '{file_name}' download size mismatch: expected {expected_bytes} bytes, got {received_bytes}"
    ))
}

async fn send_presigned_attachment_download(
    client: &reqwest::Client,
    download: &DownloadAttachmentResponse,
) -> PublicResult<reqwest::Response> {
    let mut request = client.get(&download.download_url);
    for (name, value) in &download.download_headers {
        request = request.header(name, value);
    }

    let response = request.send().await.map_err(|err| {
        PublicError::unexpected(format!(
            "failed to download attachment ciphertext: {}",
            err.without_url()
        ))
    })?;
    let status = response.status();
    if !status.is_success() {
        return Err(PublicError::unexpected(format!(
            "attachment download failed with status {}",
            status
        )));
    }

    Ok(response)
}

async fn build_readable_attachment_async(
    attachment: AgentAttachment,
    bytes: Vec<u8>,
    read_strategy: AttachmentReadStrategy,
) -> PublicResult<ReadableAttachment> {
    if read_strategy != AttachmentReadStrategy::DocxMarkdown {
        return build_readable_attachment(attachment, bytes, read_strategy);
    }

    tokio::task::spawn_blocking(move || build_readable_attachment(attachment, bytes, read_strategy))
        .await
        .map_err(|err| {
            PublicError::unexpected(format!("DOCX attachment rendering task failed: {err}"))
        })?
}

pub(crate) fn build_readable_attachment(
    attachment: AgentAttachment,
    bytes: Vec<u8>,
    read_strategy: AttachmentReadStrategy,
) -> PublicResult<ReadableAttachment> {
    let (text, content_format, source_kind) = match read_strategy {
        AttachmentReadStrategy::Utf8Text => (
            decode_attachment_utf8_text(&attachment.file_name, bytes)?,
            if content_type_is_markdown(&attachment.content_type)
                || file_extension_is_markdown(&attachment.file_name)
            {
                ReadableAttachmentContentFormat::Markdown
            } else {
                ReadableAttachmentContentFormat::Text
            },
            ReadableAttachmentSourceKind::PlainText,
        ),
        AttachmentReadStrategy::DocxMarkdown => (
            render_docx_attachment_as_markdown(&attachment.file_name, &bytes)?,
            ReadableAttachmentContentFormat::Markdown,
            ReadableAttachmentSourceKind::DocxRendered,
        ),
        AttachmentReadStrategy::Unsupported => {
            return Err(unsupported_attachment_render_error(&attachment.file_name));
        }
    };

    Ok(ReadableAttachment {
        attachment,
        text,
        content_format,
        source_kind,
    })
}

fn decode_attachment_utf8_text(file_name: &str, bytes: Vec<u8>) -> PublicResult<String> {
    String::from_utf8(bytes).map_err(|err| {
        PublicError::validation(format!(
            "attachment '{}' is not valid UTF-8 text: {}",
            file_name, err
        ))
    })
}

fn render_docx_attachment_as_markdown(file_name: &str, bytes: &[u8]) -> PublicResult<String> {
    validate_docx_archive(file_name, bytes)?;
    let markdown = undocx::builder()
        .skip_images()
        .convert_bytes(bytes)
        .map_err(|err| {
            PublicError::validation(format!(
                "attachment '{}' could not be rendered as Markdown: {}",
                file_name, err
            ))
        })?;
    if markdown.len() > MAX_DOCX_RENDERED_MARKDOWN_BYTES {
        return Err(docx_render_limit_error(
            file_name,
            "rendered Markdown is too large",
        ));
    }

    let normalized = normalize_docx_markdown(&markdown);
    if normalized.len() > MAX_DOCX_RENDERED_MARKDOWN_BYTES {
        return Err(docx_render_limit_error(
            file_name,
            "normalized Markdown is too large",
        ));
    }
    Ok(normalized)
}

fn validate_docx_archive(file_name: &str, bytes: &[u8]) -> PublicResult<()> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|err| {
        PublicError::validation(format!(
            "attachment '{file_name}' is not a valid DOCX archive: {err}"
        ))
    })?;
    if archive.len() > MAX_DOCX_ARCHIVE_ENTRIES {
        return Err(docx_render_limit_error(
            file_name,
            "archive contains too many entries",
        ));
    }

    let mut total_uncompressed_bytes = 0_u64;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|err| {
            PublicError::validation(format!(
                "attachment '{file_name}' contains an invalid DOCX entry: {err}"
            ))
        })?;
        if entry.enclosed_name().is_none() || (!entry.is_dir() && !entry.is_file()) {
            return Err(docx_render_limit_error(
                file_name,
                "archive contains an unsafe entry",
            ));
        }
        if entry.is_dir() {
            continue;
        }

        let declared_size = entry.size();
        let compressed_size = entry.compressed_size();
        validate_docx_entry_limits(file_name, declared_size, compressed_size)?;
        total_uncompressed_bytes = total_uncompressed_bytes
            .checked_add(declared_size)
            .filter(|total| *total <= MAX_DOCX_TOTAL_UNCOMPRESSED_BYTES)
            .ok_or_else(|| {
                docx_render_limit_error(file_name, "archive expands beyond the total size limit")
            })?;

        let already_verified = total_uncompressed_bytes - declared_size;
        let aggregate_remaining = MAX_DOCX_TOTAL_UNCOMPRESSED_BYTES - already_verified;
        let read_limit = MAX_DOCX_ENTRY_UNCOMPRESSED_BYTES.min(aggregate_remaining) + 1;
        let actual_size =
            io::copy(&mut entry.by_ref().take(read_limit), &mut io::sink()).map_err(|err| {
                PublicError::validation(format!(
                    "attachment '{file_name}' contains an unreadable DOCX entry: {err}"
                ))
            })?;
        if actual_size > MAX_DOCX_ENTRY_UNCOMPRESSED_BYTES || actual_size > aggregate_remaining {
            return Err(docx_render_limit_error(
                file_name,
                "archive expands beyond the size limit",
            ));
        }
        if actual_size != declared_size {
            return Err(PublicError::validation(format!(
                "attachment '{file_name}' contains a DOCX entry with an invalid size"
            )));
        }
        validate_docx_entry_limits(file_name, actual_size, compressed_size)?;
    }

    Ok(())
}

fn validate_docx_entry_limits(
    file_name: &str,
    uncompressed_size: u64,
    compressed_size: u64,
) -> PublicResult<()> {
    if uncompressed_size > MAX_DOCX_ENTRY_UNCOMPRESSED_BYTES {
        return Err(docx_render_limit_error(
            file_name,
            "an archive entry expands beyond the per-entry size limit",
        ));
    }
    if uncompressed_size > 0
        && (compressed_size == 0
            || uncompressed_size > compressed_size.saturating_mul(MAX_DOCX_COMPRESSION_RATIO))
    {
        return Err(docx_render_limit_error(
            file_name,
            "an archive entry has an unsafe compression ratio",
        ));
    }
    Ok(())
}

fn docx_render_limit_error(file_name: &str, reason: &str) -> PublicError {
    PublicError::validation(format!(
        "attachment '{file_name}' cannot be safely rendered as Markdown because {reason}; download it instead"
    ))
}

pub(crate) fn normalize_docx_markdown(markdown: &str) -> String {
    let mut normalized = String::new();
    let mut prose_buffer = String::new();
    let mut in_code_fence = false;

    for line in markdown.split_inclusive('\n') {
        if line.trim_start().starts_with("```") {
            if !prose_buffer.is_empty() {
                normalized.push_str(&normalize_non_code_docx_markdown(&prose_buffer));
                prose_buffer.clear();
            }

            normalized.push_str(line);
            in_code_fence = !in_code_fence;
            continue;
        }

        if in_code_fence {
            normalized.push_str(line);
        } else {
            prose_buffer.push_str(line);
        }
    }

    if !prose_buffer.is_empty() {
        normalized.push_str(&normalize_non_code_docx_markdown(&prose_buffer));
    }

    normalized
}

fn normalize_non_code_docx_markdown(markdown: &str) -> String {
    let markdown = normalize_docx_html_tables(markdown);
    let markdown = normalize_docx_inline_tag_pair(&markdown, "strong", "**");
    let markdown = normalize_docx_inline_tag_pair(&markdown, "em", "*");
    let markdown = normalize_docx_inline_tag_pair(&markdown, "s", "~~");
    normalize_docx_inline_tag_pair(&markdown, "del", "~~")
}

fn normalize_docx_inline_tag_pair(markdown: &str, tag: &str, marker: &str) -> String {
    let open_tag = format!("<{tag}>");
    let close_tag = format!("</{tag}>");
    let mut normalized = String::new();
    let mut remaining = markdown;

    while let Some(tag_start) = remaining.find(&open_tag) {
        normalized.push_str(&remaining[..tag_start]);

        let inner_start = tag_start + open_tag.len();
        let Some(inner_end) =
            find_matching_docx_inline_tag_end(remaining, inner_start, &open_tag, &close_tag)
        else {
            normalized.push_str(&remaining[tag_start..]);
            return normalized;
        };

        let inner = remaining[inner_start..inner_end]
            .replace(&open_tag, "")
            .replace(&close_tag, "");
        let inner = inner.as_str();
        let trimmed = inner.trim_matches(char::is_whitespace);
        if trimmed.is_empty() {
            normalized.push_str(inner);
        } else {
            let leading_whitespace_len = inner.len() - inner.trim_start().len();
            let trailing_whitespace_len = inner.len() - inner.trim_end().len();
            normalized.push_str(&inner[..leading_whitespace_len]);
            normalized.push_str(marker);
            normalized.push_str(trimmed);
            normalized.push_str(marker);
            normalized.push_str(&inner[inner.len() - trailing_whitespace_len..]);
        }

        remaining = &remaining[inner_end + close_tag.len()..];
    }

    normalized.push_str(remaining);
    normalized
}

fn find_matching_docx_inline_tag_end(
    markdown: &str,
    inner_start: usize,
    open_tag: &str,
    close_tag: &str,
) -> Option<usize> {
    let mut depth = 1usize;
    let mut search_from = inner_start;

    while search_from < markdown.len() {
        let next_open = markdown[search_from..]
            .find(open_tag)
            .map(|offset| search_from + offset);
        let next_close = markdown[search_from..]
            .find(close_tag)
            .map(|offset| search_from + offset);

        match (next_open, next_close) {
            (Some(open), Some(close)) if open < close => {
                depth += 1;
                search_from = open + open_tag.len();
            }
            (_, Some(close)) => {
                depth -= 1;
                if depth == 0 {
                    return Some(close);
                }
                search_from = close + close_tag.len();
            }
            _ => return None,
        }
    }

    None
}

fn normalize_docx_html_tables(markdown: &str) -> String {
    let mut normalized = String::new();
    let mut remaining = markdown;

    while let Some(table_start) = remaining.find("<table") {
        normalized.push_str(&remaining[..table_start]);

        let Some(table_end) = find_docx_html_table_end(remaining, table_start) else {
            normalized.push_str(&remaining[table_start..]);
            return normalized;
        };

        let table_markdown = html2md::parse_html(&remaining[table_start..table_end]);
        normalized.push_str(table_markdown.trim_matches('\n'));
        remaining = &remaining[table_end..];
    }

    normalized.push_str(remaining);
    normalized
}

fn find_docx_html_table_end(markdown: &str, table_start: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut search_from = table_start;

    while search_from < markdown.len() {
        let next_open = markdown[search_from..]
            .find("<table")
            .map(|offset| search_from + offset);
        let next_close = markdown[search_from..]
            .find("</table>")
            .map(|offset| search_from + offset);

        match (next_open, next_close) {
            (Some(open), Some(close)) if open < close => {
                depth += 1;
                search_from = open + "<table".len();
            }
            (_, Some(close)) => {
                if depth == 0 {
                    return None;
                }

                depth -= 1;
                search_from = close + "</table>".len();

                if depth == 0 {
                    return Some(search_from);
                }
            }
            _ => return None,
        }
    }

    None
}

fn unsupported_attachment_read_error(file_name: &str) -> PublicError {
    PublicError::validation(format!(
        "attachment '{}' is not readable in the CLI; use download instead",
        file_name
    ))
}

fn unsupported_attachment_render_error(file_name: &str) -> PublicError {
    PublicError::validation(format!(
        "attachment '{}' is not readable in the CLI",
        file_name
    ))
}

fn normalized_content_type(content_type: &str) -> String {
    content_type
        .split(';')
        .next()
        .unwrap_or(content_type)
        .trim()
        .to_ascii_lowercase()
}

fn file_extension(file_name: &str) -> Option<String> {
    Path::new(file_name)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|extension| extension.to_ascii_lowercase())
}

fn content_type_is_textual(content_type: &str) -> bool {
    let normalized = normalized_content_type(content_type);
    if normalized.starts_with("text/") {
        return true;
    }

    matches!(normalized.as_str(), "application/json" | "application/xml")
        || normalized.ends_with("+json")
        || normalized.ends_with("+xml")
}

fn file_extension_is_textual(file_name: &str) -> bool {
    let Some(extension) = file_extension(file_name) else {
        return false;
    };

    matches!(
        extension.as_str(),
        "txt" | "md" | "markdown" | "json" | "yaml" | "yml" | "toml" | "csv" | "log"
    )
}

fn content_type_is_docx(content_type: &str) -> bool {
    normalized_content_type(content_type) == DOCX_CONTENT_TYPE
}

fn file_extension_is_docx(file_name: &str) -> bool {
    matches!(file_extension(file_name).as_deref(), Some("docx"))
}

fn content_type_is_markdown(content_type: &str) -> bool {
    normalized_content_type(content_type) == "text/markdown"
}

fn file_extension_is_markdown(file_name: &str) -> bool {
    matches!(
        file_extension(file_name).as_deref(),
        Some("md" | "markdown")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine as _;

    const TEST_DOCX_BASE64: &str = "UEsDBBQAAAAIAOp8kVzXeYTq8QAAALgBAAATAAAAW0NvbnRlbnRfVHlwZXNdLnhtbH2QzU7DMBCE730Ky9cqccoBIZSkB36OwKE8wMreJFb9J69b2rdn00KREOVozXwz62nXB+/EHjPZGDq5qhspMOhobBg7+b55ru6koALBgIsBO3lEkut+0W6OCUkwHKiTUynpXinSE3qgOiYMrAwxeyj8zKNKoLcworppmlulYygYSlXmDNkvhGgfcYCdK+LpwMr5loyOpHg4e+e6TkJKzmoorKt9ML+Kqq+SmsmThyabaMkGqa6VzOL1jh/0lSfK1qB4g1xewLNRfcRslIl65xmu/0/649o4DFbjhZ/TUo4aiXh77+qL4sGG71+06jR8/wlQSwMEFAAAAAgA6nyRXCAbhuqyAAAALgEAAAsAAABfcmVscy8ucmVsc43Puw6CMBQG4J2naM4uBQdjDIXFmLAafICmPZRGeklbL7y9HRzEODie23fyN93TzOSOIWpnGdRlBQStcFJbxeAynDZ7IDFxK/nsLDJYMELXFs0ZZ57yTZy0jyQjNjKYUvIHSqOY0PBYOo82T0YXDE+5DIp6Lq5cId1W1Y6GTwPagpAVS3rJIPSyBjIsHv/h3ThqgUcnbgZt+vHlayPLPChMDB4uSCrf7TKzQHNKuorZvgBQSwMEFAAAAAgA6nyRXDbicKixAAAADAEAABEAAAB3b3JkL2RvY3VtZW50LnhtbG2PMQ+CMBCFd35F012KDsYQKIPGuLlo4lrpKST0rmmryL+3xbixfHkv9/Lurmo+ZmBvcL4nrPk6LzgDbEn3+Kz59XJc7TjzQaFWAyHUfALPG5lVY6mpfRnAwGID+nKseReCLYXwbQdG+ZwsYJw9yBkVonVPMZLT1lEL3scFZhCbotgKo3rkMmMstt5JT0nOxsoIlxDkCVQ6qhLJJLqZdjF8OO9vLFUtxpP47Unq/4f8AlBLAQIUAxQAAAAIAOp8kVzXeYTq8QAAALgBAAATAAAAAAAAAAAAAACAAQAAAABbQ29udGVudF9UeXBlc10ueG1sUEsBAhQDFAAAAAgA6nyRXCAbhuqyAAAALgEAAAsAAAAAAAAAAAAAAIABIgEAAF9yZWxzLy5yZWxzUEsBAhQDFAAAAAgA6nyRXDbicKixAAAADAEAABEAAAAAAAAAAAAAAIAB/QEAAHdvcmQvZG9jdW1lbnQueG1sUEsFBgAAAAADAAMAuQAAAN0CAAAAAA==";

    #[test]
    fn test_should_detect_text_docx_and_binary_attachment_read_strategies() {
        let text_attachment = AgentAttachment {
            id: Uuid::nil(),
            file_name: "notes.md".to_string(),
            content_type: "text/markdown".to_string(),
            size_bytes: 0,
            blob_key: Vec::new(),
        };
        let docx_attachment = AgentAttachment {
            id: Uuid::nil(),
            file_name: "spec.docx".to_string(),
            content_type: DOCX_CONTENT_TYPE.to_string(),
            size_bytes: 0,
            blob_key: Vec::new(),
        };
        let binary_attachment = AgentAttachment {
            id: Uuid::nil(),
            file_name: "spec.pdf".to_string(),
            content_type: "application/pdf".to_string(),
            size_bytes: 0,
            blob_key: Vec::new(),
        };

        assert_eq!(
            text_attachment.read_strategy(),
            AttachmentReadStrategy::Utf8Text
        );
        assert_eq!(
            docx_attachment.read_strategy(),
            AttachmentReadStrategy::DocxMarkdown
        );
        assert_eq!(
            binary_attachment.read_strategy(),
            AttachmentReadStrategy::Unsupported
        );
    }

    #[test]
    fn test_should_return_an_error_when_rendering_an_unsupported_attachment() {
        let attachment = AgentAttachment {
            id: Uuid::nil(),
            file_name: "spec.pdf".to_string(),
            content_type: "application/pdf".to_string(),
            size_bytes: 0,
            blob_key: Vec::new(),
        };

        let error =
            build_readable_attachment(attachment, Vec::new(), AttachmentReadStrategy::Unsupported)
                .expect_err("unsupported attachment rendering must fail without panicking");

        assert!(error.to_string().contains("not readable in the CLI"));
    }

    #[test]
    fn test_should_prefer_text_content_type_over_docx_extension() {
        let attachment = AgentAttachment {
            id: Uuid::nil(),
            file_name: "notes.docx".to_string(),
            content_type: "text/plain".to_string(),
            size_bytes: 0,
            blob_key: Vec::new(),
        };

        assert_eq!(attachment.read_strategy(), AttachmentReadStrategy::Utf8Text);
    }

    #[test]
    fn test_should_report_markdown_content_format_for_markdown_text_attachment() {
        let attachment = AgentAttachment {
            id: Uuid::nil(),
            file_name: "notes.md".to_string(),
            content_type: "text/markdown".to_string(),
            size_bytes: 0,
            blob_key: Vec::new(),
        };

        let readable = build_readable_attachment(
            attachment,
            b"# Heading\n\nAttachment body\n".to_vec(),
            AttachmentReadStrategy::Utf8Text,
        )
        .expect("render markdown text attachment");

        assert_eq!(
            readable.content_format,
            ReadableAttachmentContentFormat::Markdown
        );
        assert_eq!(
            readable.source_kind,
            ReadableAttachmentSourceKind::PlainText
        );
    }

    #[test]
    fn test_should_render_docx_attachment_bytes_as_markdown() {
        let attachment = AgentAttachment {
            id: Uuid::nil(),
            file_name: "spec.docx".to_string(),
            content_type: DOCX_CONTENT_TYPE.to_string(),
            size_bytes: 0,
            blob_key: Vec::new(),
        };
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(TEST_DOCX_BASE64)
            .expect("decode docx fixture");

        let readable =
            build_readable_attachment(attachment, bytes, AttachmentReadStrategy::DocxMarkdown)
                .expect("render docx attachment");

        assert_eq!(readable.text, "Heading\n\nDOCX body\n\n");
        assert_eq!(
            readable.content_format,
            ReadableAttachmentContentFormat::Markdown
        );
        assert_eq!(
            readable.source_kind,
            ReadableAttachmentSourceKind::DocxRendered
        );
    }

    #[test]
    fn test_should_reject_docx_archives_with_unsafe_expansion_ratios() {
        use std::io::Write as _;
        use zip::write::SimpleFileOptions;

        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        writer
            .start_file(
                "word/document.xml",
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated),
            )
            .expect("start compressed entry");
        writer
            .write_all(&vec![b'a'; 1024 * 1024])
            .expect("write highly compressible entry");
        let bytes = writer.finish().expect("finish archive").into_inner();

        let error = render_docx_attachment_as_markdown("expansion.docx", &bytes)
            .expect_err("unsafe DOCX expansion must be rejected before rendering");

        assert!(error.to_string().contains("unsafe compression ratio"));
        assert!(error.to_string().contains("download it instead"));
    }

    #[test]
    fn test_should_normalize_docx_html_fragments_and_preserve_code_fences() {
        let markdown = "\
<strong>UI Implementation Spec</strong>

<table>
  <tr>
    <td><strong>Usage</strong></td>
    <td>Duration</td>
  </tr>
  <tr>
    <td><strong>Slide panels open/close</strong></td>
    <td>0.28s</td>
  </tr>
</table>

<strong>prefers-reduced-motion</strong>

```
<Button variant=\"ghost\" />
<strong>leave me alone</strong>
```
";

        let normalized = normalize_docx_markdown(markdown);

        assert!(normalized.contains("**UI Implementation Spec**"));
        assert!(normalized.contains("**Usage**"));
        assert!(normalized.contains("Duration"));
        assert!(normalized.contains("**Slide panels open/close**"));
        assert!(normalized.contains("0.28s"));
        assert!(normalized.contains("**prefers-reduced-motion**"));
        assert!(!normalized.contains("<table>"));
        assert!(!normalized.contains("<td>"));
        assert!(
            normalized.contains(
                "```\n<Button variant=\"ghost\" />\n<strong>leave me alone</strong>\n```"
            )
        );
    }

    #[test]
    fn test_should_move_outer_inline_tag_whitespace_outside_markers() {
        let normalized = normalize_docx_markdown(
            "<strong>⚠️  Note to engineering: </strong>Ignore legacy prototype remnants.\n",
        );

        assert_eq!(
            normalized,
            "**⚠️  Note to engineering:** Ignore legacy prototype remnants.\n"
        );
    }

    #[test]
    fn test_should_collapse_nested_docx_inline_tags_without_corrupting_markers() {
        let normalized = normalize_docx_markdown(
            "<strong>a <strong>b</strong> c</strong> and <em>x <em>y</em> z</em>\n",
        );

        assert_eq!(normalized, "**a b c** and *x y z*\n");
        assert!(!normalized.contains("<strong>"));
        assert!(!normalized.contains("<em>"));
    }

    #[test]
    fn test_should_preserve_whitespace_only_docx_inline_tags_once() {
        assert_eq!(normalize_docx_markdown("a<strong>  </strong>b\n"), "a  b\n");
    }
}
