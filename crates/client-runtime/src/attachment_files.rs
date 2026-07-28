use crate::operation_cancellation::OperationCancellation;
#[cfg(unix)]
use cap_fs_ext::OpenOptionsSyncExt as _;
use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt as _};
use cap_std::ambient_authority;
use cap_std::fs::{Dir, OpenOptions};
use sealtask_client_core::{PublicError, PublicResult};
use sealtask_client_crypto::MAX_ATTACHMENT_PLAINTEXT_BYTES;
use std::io;
use std::path::{Component, Path};
use tokio::io::{AsyncRead, AsyncReadExt};
use zeroize::Zeroizing;

const DOCX_CONTENT_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
const MAX_ATTACHMENT_FILE_NAME_CHARS: usize = 255;
const MAX_ATTACHMENT_CONTENT_TYPE_CHARS: usize = 256;
const READ_BUFFER_BYTES: usize = 64 * 1024;

pub(crate) async fn read_upload_file_cancellable(
    path: &Path,
    cancellation: &OperationCancellation,
) -> PublicResult<(Zeroizing<Vec<u8>>, u64)> {
    if cancellation.is_cancelled() {
        return Err(PublicError::cancelled("attachment upload cancelled"));
    }
    let path = path.to_path_buf();
    let task = tokio::spawn(async move { read_upload_file(&path).await });
    await_started_file_io(task, cancellation).await
}

pub(crate) async fn await_started_file_io<T>(
    mut task: tokio::task::JoinHandle<PublicResult<T>>,
    cancellation: &OperationCancellation,
) -> PublicResult<T> {
    let joined = tokio::select! {
        biased;
        result = &mut task => result,
        () = cancellation.cancelled() => task.await,
    };
    let result = joined.map_err(|error| {
        if error.is_panic() {
            PublicError::unexpected("attachment file I/O task panicked")
        } else {
            PublicError::unexpected("attachment file I/O task was cancelled")
        }
    })?;
    if cancellation.is_cancelled() {
        Err(PublicError::cancelled("attachment upload cancelled"))
    } else {
        result
    }
}

async fn read_upload_file(path: &Path) -> PublicResult<(Zeroizing<Vec<u8>>, u64)> {
    let directory =
        tokio::task::spawn_blocking(|| Dir::open_ambient_dir(Path::new("."), ambient_authority()))
            .await
            .map_err(|err| {
                PublicError::unexpected(format!("working directory open task failed: {err}"))
            })?
            .map_err(|err| {
                PublicError::unexpected(format!(
                    "failed to open the current working directory: {err}"
                ))
            })?;
    read_upload_file_in(directory, path).await
}

pub(crate) async fn read_upload_file_in(
    directory: Dir,
    path: &Path,
) -> PublicResult<(Zeroizing<Vec<u8>>, u64)> {
    validate_working_directory_relative_path(path, "attachment path")?;
    let relative_path = path.to_path_buf();
    let file = tokio::task::spawn_blocking(move || {
        let mut options = OpenOptions::new();
        options.read(true).follow(FollowSymlinks::No);
        // A FIFO opened read-only blocks until a writer appears. Request
        // nonblocking mode as part of the atomic open so metadata validation
        // can reject special files without ever waiting on a peer.
        #[cfg(unix)]
        options.nonblock(true);
        directory
            .open_with(&relative_path, &options)
            .map(cap_std::fs::File::into_std)
    })
    .await
    .map_err(|err| PublicError::unexpected(format!("attachment file open task failed: {err}")))?
    .map_err(|err| {
        PublicError::validation(format!(
            "attachment path {} must stay within the working directory, identify a regular file, and must not end in a symbolic link or reparse point: {err}",
            path.display()
        ))
    })?;
    let mut file = tokio::fs::File::from_std(file);
    let metadata = file.metadata().await.map_err(|err| {
        PublicError::unexpected(format!(
            "failed to inspect open attachment file {}: {err}",
            path.display()
        ))
    })?;
    if !safe_attachment_input_metadata(&metadata) {
        return Err(PublicError::validation(format!(
            "attachment path {} must identify a regular file and must not be a symbolic link or reparse point",
            path.display()
        )));
    }
    validate_upload_plaintext_size(metadata.len())?;

    let bytes = read_bounded(&mut file, MAX_ATTACHMENT_PLAINTEXT_BYTES)
        .await
        .map_err(|err| {
            PublicError::unexpected(format!(
                "failed to read attachment file {}: {err}",
                path.display()
            ))
        })?;
    let bytes_len = u64::try_from(bytes.len())
        .map_err(|_| PublicError::validation("attachment file is too large for this platform"))?;
    validate_upload_plaintext_size(bytes_len)?;
    Ok((bytes, bytes_len))
}

fn validate_working_directory_relative_path(path: &Path, label: &str) -> PublicResult<()> {
    if path.as_os_str().is_empty() {
        return Err(PublicError::validation(format!("{label} cannot be empty")));
    }

    let mut has_file_name = false;
    for component in path.components() {
        match component {
            Component::Normal(_) => has_file_name = true,
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(PublicError::validation(format!(
                    "{label} must be relative to the current working directory and cannot contain parent-directory traversal"
                )));
            }
        }
    }
    if !has_file_name {
        return Err(PublicError::validation(format!(
            "{label} must identify a file below the current working directory"
        )));
    }
    Ok(())
}

#[cfg(windows)]
fn safe_attachment_input_metadata(metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt as _;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
    metadata.is_file() && metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT == 0
}

#[cfg(not(windows))]
fn safe_attachment_input_metadata(metadata: &std::fs::Metadata) -> bool {
    metadata.is_file() && !metadata.file_type().is_symlink()
}

pub(crate) async fn read_bounded<R: AsyncRead + Unpin>(
    reader: &mut R,
    maximum: u64,
) -> io::Result<Zeroizing<Vec<u8>>> {
    let read_limit = maximum.saturating_add(1);
    let mut remaining = read_limit;
    let mut bytes = Zeroizing::new(Vec::new());
    let mut buffer = Zeroizing::new([0_u8; READ_BUFFER_BYTES]);
    while remaining > 0 {
        let chunk_limit = usize::try_from(remaining)
            .unwrap_or(usize::MAX)
            .min(buffer.len());
        let read = reader.read(&mut buffer[..chunk_limit]).await?;
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..read]);
        remaining -= u64::try_from(read).unwrap_or(remaining);
    }
    Ok(bytes)
}

fn validate_upload_plaintext_size(bytes: u64) -> PublicResult<()> {
    if bytes == 0 || bytes > MAX_ATTACHMENT_PLAINTEXT_BYTES {
        return Err(PublicError::validation(format!(
            "attachment files must contain between 1 and {MAX_ATTACHMENT_PLAINTEXT_BYTES} bytes"
        )));
    }
    Ok(())
}

pub(crate) fn normalize_upload_file_name(
    path: &Path,
    override_name: Option<&str>,
) -> PublicResult<String> {
    let name = match override_name {
        Some(name) => name.trim(),
        None => path
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::trim)
            .ok_or_else(|| {
                PublicError::validation(
                    "attachment file name is not valid UTF-8; provide --file-name",
                )
            })?,
    };
    let chars = name.chars().count();
    if chars == 0
        || chars > MAX_ATTACHMENT_FILE_NAME_CHARS
        || matches!(name, "." | "..")
        || name
            .chars()
            .any(|ch| ch.is_control() || matches!(ch, '/' | '\\'))
    {
        return Err(PublicError::validation(format!(
            "attachment file name must contain between 1 and {MAX_ATTACHMENT_FILE_NAME_CHARS} safe characters"
        )));
    }
    Ok(name.to_string())
}

pub(crate) fn normalize_upload_content_type(
    value: Option<&str>,
    path: &Path,
) -> PublicResult<String> {
    let content_type = match value {
        Some(value) => value.trim().to_string(),
        None => infer_content_type(path).to_string(),
    };
    let chars = content_type.chars().count();
    if chars == 0
        || chars > MAX_ATTACHMENT_CONTENT_TYPE_CHARS
        || content_type.chars().any(char::is_control)
        || !content_type.contains('/')
    {
        return Err(PublicError::validation(format!(
            "attachment content type must contain between 1 and {MAX_ATTACHMENT_CONTENT_TYPE_CHARS} characters and include '/'"
        )));
    }
    Ok(content_type)
}

fn infer_content_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("txt") => "text/plain",
        Some("md" | "markdown") => "text/markdown",
        Some("csv") => "text/csv",
        Some("json") => "application/json",
        Some("html" | "htm") => "text/html",
        Some("pdf") => "application/pdf",
        Some("docx") => DOCX_CONTENT_TYPE,
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        _ => "application/octet-stream",
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn unix_fifo_is_rejected_without_waiting_for_a_writer() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let fifo = temporary.path().join("attachment.fifo");
        let status = std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .expect("run mkfifo");
        assert!(status.success(), "mkfifo must succeed");
        let directory =
            Dir::open_ambient_dir(temporary.path(), ambient_authority()).expect("open test dir");

        let error = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            read_upload_file_in(directory, Path::new("attachment.fifo")),
        )
        .await
        .expect("FIFO validation must not block")
        .expect_err("FIFO must not be accepted as an attachment");

        assert!(matches!(error, PublicError::Validation(_)));
    }
}
