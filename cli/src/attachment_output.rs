use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt as _};
use cap_std::ambient_authority;
use cap_std::fs::{Dir, OpenOptions};
use sealtask_client_core::{PublicError, PublicResult};
use std::fs;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use uuid::Uuid;

pub(crate) fn resolve_attachment_output_path(file_name: &str, output: Option<PathBuf>) -> PathBuf {
    output.unwrap_or_else(|| PathBuf::from(sanitize_attachment_file_name(file_name)))
}

fn sanitize_attachment_file_name(file_name: &str) -> String {
    let candidate = file_name
        .rsplit(['/', '\\'])
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "." && *value != "..")
        .unwrap_or("attachment.bin");
    let mut sanitized: String = candidate
        .chars()
        .map(sanitize_attachment_file_name_char)
        .collect();

    // Windows strips trailing dots/spaces during path resolution. Replace them
    // so a displayed default always names the exact file that will be created.
    let valid_length = sanitized.trim_end_matches(['.', ' ']).len();
    sanitized.replace_range(valid_length.., &"_".repeat(sanitized.len() - valid_length));
    if sanitized.is_empty() || sanitized == "." || sanitized == ".." {
        return "attachment.bin".to_string();
    }

    // Windows device names remain reserved with any extension and regardless
    // of case. Prefixing keeps the server-provided name recognizable and safe.
    let stem = sanitized.split('.').next().unwrap_or_default();
    if is_windows_reserved_file_stem(stem) {
        sanitized.insert(0, '_');
    }
    sanitized
}

fn sanitize_attachment_file_name_char(ch: char) -> char {
    match ch {
        '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
        ch if ch.is_control() => '_',
        _ => ch,
    }
}

fn is_windows_reserved_file_stem(stem: &str) -> bool {
    let stem = stem.trim_end_matches(['.', ' ']).to_ascii_uppercase();
    matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || stem.strip_prefix("COM").is_some_and(|suffix| {
            matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
        })
        || stem.strip_prefix("LPT").is_some_and(|suffix| {
            matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
        })
}

pub(crate) fn write_attachment_file(path: &Path, bytes: &[u8], force: bool) -> PublicResult<()> {
    let directory = Dir::open_ambient_dir(Path::new("."), ambient_authority()).map_err(|err| {
        PublicError::unexpected(format!(
            "failed to open the current working directory: {err}"
        ))
    })?;
    write_attachment_file_in(&directory, path, bytes, force)
}

fn write_attachment_file_in(
    directory: &Dir,
    path: &Path,
    bytes: &[u8],
    force: bool,
) -> PublicResult<()> {
    validate_working_directory_relative_output_path(path)?;
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        directory.create_dir_all(parent).map_err(|err| {
            PublicError::validation(format!(
                "output directory {} must stay within the current working directory and must not traverse an escaping symbolic link or reparse point: {err}",
                parent.display()
            ))
        })?;
    }

    let parent_path = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty());
    let parent = match parent_path {
        Some(parent_path) => directory.open_dir(parent_path),
        None => directory.try_clone(),
    }
    .map_err(|err| {
        PublicError::validation(format!(
            "output directory for {} must stay within the current working directory and must not traverse an escaping symbolic link or reparse point: {err}",
            path.display()
        ))
    })?;
    let file_name = path.file_name().ok_or_else(|| {
        PublicError::validation(
            "output path must identify a file below the current working directory",
        )
    })?;

    install_attachment_file(
        &parent,
        Path::new(file_name),
        path,
        force,
        |file| file.write_all(bytes),
        fs::File::sync_all,
    )
}

#[cfg(test)]
fn replace_attachment_file(
    parent: &Dir,
    file_name: &Path,
    display_path: &Path,
    write_contents: impl FnOnce(&mut fs::File) -> io::Result<()>,
) -> PublicResult<()> {
    install_attachment_file(
        parent,
        file_name,
        display_path,
        true,
        write_contents,
        fs::File::sync_all,
    )
}

fn install_attachment_file(
    parent: &Dir,
    file_name: &Path,
    display_path: &Path,
    force: bool,
    write_contents: impl FnOnce(&mut fs::File) -> io::Result<()>,
    sync_contents: impl FnOnce(&fs::File) -> io::Result<()>,
) -> PublicResult<()> {
    if force {
        validate_force_target(parent, file_name, display_path)?;
    }

    let mut temporary = SiblingTemporaryFile::create(parent, display_path)?;
    let temporary_file = temporary.file_mut()?;
    if let Err(err) = write_contents(temporary_file) {
        let primary = PublicError::unexpected(format!(
            "failed to write temporary output file for {}: {err}",
            display_path.display()
        ));
        return Err(temporary.cleanup_after_error(primary));
    }
    let temporary_file = temporary.file()?;
    if let Err(err) = sync_contents(temporary_file) {
        let primary = PublicError::unexpected(format!(
            "failed to sync temporary output file for {}: {err}",
            display_path.display()
        ));
        return Err(temporary.cleanup_after_error(primary));
    }
    temporary.close();

    if force {
        // Check again immediately before replacement. Renaming over a symlink
        // replaces the link itself, but rejecting non-regular targets keeps
        // the command's overwrite contract explicit across platforms.
        if let Err(primary) = validate_force_target(parent, file_name, display_path) {
            return Err(temporary.cleanup_after_error(primary));
        }
        if let Err(err) = parent.rename(temporary.name(), parent, file_name) {
            let primary = PublicError::unexpected(format!(
                "failed to atomically replace output file {}: {err}",
                display_path.display()
            ));
            return Err(temporary.cleanup_after_error(primary));
        }
        temporary.disarm();
        return Ok(());
    }

    // Publishing with a same-directory hard link is an atomic create-new:
    // an existing path is never replaced, while readers can only observe the
    // fully written and synced inode.
    if let Err(err) = parent.hard_link(temporary.name(), parent, file_name) {
        let primary = if err.kind() == io::ErrorKind::AlreadyExists {
            PublicError::validation(format!(
                "output file {} already exists; use --force to overwrite",
                display_path.display()
            ))
        } else {
            PublicError::unexpected(format!(
                "failed to atomically install output file {}: {err}",
                display_path.display()
            ))
        };
        return Err(temporary.cleanup_after_error(primary));
    }

    temporary.remove_after_publish()
}

fn validate_force_target(parent: &Dir, file_name: &Path, display_path: &Path) -> PublicResult<()> {
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    let file = match parent.open_with(file_name, &options) {
        Ok(file) => file.into_std(),
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(err) => {
            return Err(PublicError::validation(format!(
                "output path {} must identify a regular file and must not end in a symbolic link or reparse point: {err}",
                display_path.display()
            )));
        }
    };
    let metadata = file.metadata().map_err(|err| {
        PublicError::unexpected(format!(
            "failed to inspect open output file {}: {err}",
            display_path.display()
        ))
    })?;
    if !safe_attachment_output_metadata(&metadata) {
        return Err(PublicError::validation(format!(
            "output path {} must be a regular file and must not be a symbolic link or reparse point",
            display_path.display()
        )));
    }
    Ok(())
}

struct SiblingTemporaryFile<'a> {
    parent: &'a Dir,
    display_path: &'a Path,
    name: PathBuf,
    file: Option<fs::File>,
    armed: bool,
}

impl<'a> SiblingTemporaryFile<'a> {
    fn create(parent: &'a Dir, display_path: &'a Path) -> PublicResult<Self> {
        for _ in 0..8 {
            let name = PathBuf::from(format!(".sealtask-attachment-{}.tmp", Uuid::now_v7()));
            let mut options = OpenOptions::new();
            options
                .write(true)
                .create_new(true)
                .follow(FollowSymlinks::No);
            match parent.open_with(&name, &options) {
                Ok(file) => {
                    let file = file.into_std();
                    let temporary = Self {
                        parent,
                        display_path,
                        name,
                        file: Some(file),
                        armed: true,
                    };
                    let metadata = match temporary.file()?.metadata() {
                        Ok(metadata) => metadata,
                        Err(err) => {
                            let primary = PublicError::unexpected(format!(
                                "failed to inspect temporary output file for {}: {err}",
                                display_path.display()
                            ));
                            return Err(temporary.cleanup_after_error(primary));
                        }
                    };
                    if !safe_attachment_output_metadata(&metadata) {
                        let primary = PublicError::validation(format!(
                            "temporary output path for {} was not a regular file",
                            display_path.display()
                        ));
                        return Err(temporary.cleanup_after_error(primary));
                    }
                    return Ok(temporary);
                }
                Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {}
                Err(err) => {
                    return Err(PublicError::unexpected(format!(
                        "failed to create a temporary output file beside {}: {err}",
                        display_path.display()
                    )));
                }
            }
        }
        Err(PublicError::unexpected(format!(
            "failed to allocate a unique temporary output file beside {}",
            display_path.display()
        )))
    }

    fn name(&self) -> &Path {
        &self.name
    }

    fn file(&self) -> PublicResult<&fs::File> {
        self.file.as_ref().ok_or_else(|| {
            PublicError::unexpected("temporary attachment handle closed before publication")
        })
    }

    fn file_mut(&mut self) -> PublicResult<&mut fs::File> {
        self.file.as_mut().ok_or_else(|| {
            PublicError::unexpected("temporary attachment handle closed before publication")
        })
    }

    fn close(&mut self) {
        self.file.take();
    }

    fn disarm(&mut self) {
        self.armed = false;
    }

    fn cleanup_after_error(mut self, primary: PublicError) -> PublicError {
        self.close();
        match self.parent.remove_file(&self.name) {
            Ok(()) => {
                self.disarm();
                primary
            }
            Err(cleanup) => {
                attachment_output_cleanup_failure(primary, cleanup, self.display_path, &self.name)
            }
        }
    }

    fn remove_after_publish(mut self) -> PublicResult<()> {
        match self.parent.remove_file(&self.name) {
            Ok(()) => {
                self.disarm();
                Ok(())
            }
            Err(cleanup) => Err(PublicError::compensation_failed(
                "attachment output installation",
                format!(
                    "output file {} was installed successfully",
                    self.display_path.display()
                ),
                format!(
                    "failed to remove temporary sibling {}: {cleanup}",
                    self.name.display()
                ),
            )),
        }
    }
}

impl Drop for SiblingTemporaryFile<'_> {
    fn drop(&mut self) {
        self.close();
        if self.armed {
            let _ = self.parent.remove_file(&self.name);
        }
    }
}

fn attachment_output_cleanup_failure(
    primary: PublicError,
    cleanup: io::Error,
    display_path: &Path,
    temporary_name: &Path,
) -> PublicError {
    PublicError::compensation_failed(
        "attachment output",
        primary.to_string(),
        format!(
            "failed to remove temporary sibling {} for {}: {cleanup}",
            temporary_name.display(),
            display_path.display()
        ),
    )
}

fn validate_working_directory_relative_output_path(path: &Path) -> PublicResult<()> {
    if path.as_os_str().is_empty() {
        return Err(PublicError::validation("output path cannot be empty"));
    }

    let mut has_file_name = false;
    for component in path.components() {
        match component {
            Component::Normal(_) => has_file_name = true,
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(PublicError::validation(
                    "output path must be relative to the current working directory and cannot contain parent-directory traversal",
                ));
            }
        }
    }
    if !has_file_name {
        return Err(PublicError::validation(
            "output path must identify a file below the current working directory",
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn safe_attachment_output_metadata(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt as _;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
    metadata.is_file() && metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT == 0
}

#[cfg(not(windows))]
fn safe_attachment_output_metadata(metadata: &fs::Metadata) -> bool {
    metadata.is_file() && !metadata.file_type().is_symlink()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_attachment_file_names_are_cross_platform_safe() {
        assert_eq!(
            sanitize_attachment_file_name("dir\\report:<bad>?*\n.txt"),
            "report__bad____.txt"
        );
        assert_eq!(sanitize_attachment_file_name("NUL.txt"), "_NUL.txt");
        assert_eq!(sanitize_attachment_file_name("com9.LOG"), "_com9.LOG");
        assert_eq!(sanitize_attachment_file_name("con .txt"), "_con .txt");
        assert_eq!(sanitize_attachment_file_name("ordinary. "), "ordinary_");
        assert_eq!(sanitize_attachment_file_name("COM0.txt"), "COM0.txt");
    }

    #[test]
    fn attachment_output_preserves_create_new_and_safely_overwrites_regular_files() {
        let directory = tempfile::tempdir().expect("temp dir");
        let root =
            Dir::open_ambient_dir(directory.path(), ambient_authority()).expect("root capability");
        let output = Path::new("attachment.bin");

        write_attachment_file_in(&root, output, b"first", false).expect("create output");
        let error = write_attachment_file_in(&root, output, b"second", false)
            .expect_err("create_new must reject an existing file");
        assert!(error.to_string().contains("use --force"));
        assert_eq!(
            fs::read(directory.path().join(output)).expect("unchanged output"),
            b"first"
        );

        write_attachment_file_in(&root, output, b"x", true).expect("force regular output");
        assert_eq!(
            fs::read(directory.path().join(output)).expect("overwritten output"),
            b"x"
        );
        assert_eq!(
            fs::read_dir(directory.path()).expect("directory").count(),
            1,
            "successful replacement must not retain a temporary sibling"
        );
    }

    #[test]
    fn attachment_output_force_preserves_original_when_temporary_write_fails() {
        let directory = tempfile::tempdir().expect("temp dir");
        fs::write(directory.path().join("attachment.bin"), b"original").expect("original");
        let root =
            Dir::open_ambient_dir(directory.path(), ambient_authority()).expect("root capability");

        let error = replace_attachment_file(
            &root,
            Path::new("attachment.bin"),
            Path::new("attachment.bin"),
            |file| {
                file.write_all(b"partial")?;
                Err(io::Error::other("injected write failure"))
            },
        )
        .expect_err("temporary write failure");

        assert!(matches!(error, PublicError::Unexpected(_)));
        assert_eq!(
            fs::read(directory.path().join("attachment.bin")).expect("original remains"),
            b"original"
        );
        assert_eq!(
            fs::read_dir(directory.path()).expect("directory").count(),
            1,
            "failed replacement must clean up its temporary sibling"
        );
    }

    #[test]
    fn attachment_output_no_force_leaves_no_partial_destination_when_write_fails() {
        let directory = tempfile::tempdir().expect("temp dir");
        let root =
            Dir::open_ambient_dir(directory.path(), ambient_authority()).expect("root capability");
        let output = Path::new("attachment.bin");

        let error = install_attachment_file(
            &root,
            output,
            output,
            false,
            |file| {
                file.write_all(b"partial")?;
                Err(io::Error::other("injected write failure"))
            },
            fs::File::sync_all,
        )
        .expect_err("temporary write failure");

        assert!(matches!(error, PublicError::Unexpected(_)));
        assert!(
            !directory.path().join(output).exists(),
            "create-new mode must publish only a complete file"
        );
        assert_eq!(
            fs::read_dir(directory.path()).expect("directory").count(),
            0,
            "failed create-new output must clean its temporary sibling"
        );
    }

    #[test]
    fn attachment_output_sync_failure_preserves_targets_and_cleans_temporary_files() {
        for force in [false, true] {
            let directory = tempfile::tempdir().expect("temp dir");
            let root = Dir::open_ambient_dir(directory.path(), ambient_authority())
                .expect("root capability");
            let output = Path::new("attachment.bin");
            if force {
                fs::write(directory.path().join(output), b"original").expect("original");
            }

            let error = install_attachment_file(
                &root,
                output,
                output,
                force,
                |file| file.write_all(b"replacement"),
                |_| Err(io::Error::other("injected sync failure")),
            )
            .expect_err("temporary sync failure");
            assert!(matches!(error, PublicError::Unexpected(_)));

            if force {
                assert_eq!(
                    fs::read(directory.path().join(output)).expect("original remains"),
                    b"original"
                );
            } else {
                assert!(!directory.path().join(output).exists());
            }
            assert_eq!(
                fs::read_dir(directory.path()).expect("directory").count(),
                usize::from(force),
                "sync failure must not retain a temporary sibling"
            );
        }
    }

    #[test]
    fn attachment_output_cleanup_failure_has_compensation_error_shape() {
        let primary = PublicError::unexpected("injected primary failure");
        let error = attachment_output_cleanup_failure(
            primary,
            io::Error::other("injected cleanup failure"),
            Path::new("attachment.bin"),
            Path::new(".sealtask-attachment-test.tmp"),
        );

        match error {
            PublicError::CompensationFailed {
                operation,
                primary,
                cleanup,
            } => {
                assert_eq!(operation, "attachment output");
                assert!(primary.contains("injected primary failure"));
                assert!(cleanup.contains("injected cleanup failure"));
                assert!(cleanup.contains(".sealtask-attachment-test.tmp"));
            }
            other => panic!("expected compensation failure, got {other:?}"),
        }
    }

    #[test]
    fn attachment_output_rejects_non_regular_force_target() {
        let directory = tempfile::tempdir().expect("temp dir");
        let root =
            Dir::open_ambient_dir(directory.path(), ambient_authority()).expect("root capability");
        let error = write_attachment_file_in(&root, Path::new("."), b"data", true)
            .expect_err("directory must not be overwritten");
        assert!(matches!(error, PublicError::Validation(_)));
    }

    #[cfg(unix)]
    #[test]
    fn attachment_output_force_rejects_symbolic_link_without_touching_target() {
        use std::os::unix::fs::symlink;

        let directory = tempfile::tempdir().expect("temp dir");
        let target = directory.path().join("target.txt");
        fs::write(&target, b"target remains").expect("target");
        let link = directory.path().join("output.txt");
        symlink(&target, &link).expect("symlink");
        let root =
            Dir::open_ambient_dir(directory.path(), ambient_authority()).expect("root capability");

        let error = write_attachment_file_in(&root, Path::new("output.txt"), b"replacement", true)
            .expect_err("symlink must be rejected");
        assert!(matches!(error, PublicError::Validation(_)));
        assert_eq!(
            fs::read(target).expect("target unchanged"),
            b"target remains"
        );
    }

    #[cfg(unix)]
    #[test]
    fn attachment_output_rejects_intermediate_symlink_escape_without_touching_target() {
        use std::os::unix::fs::symlink;

        let directory = tempfile::tempdir().expect("temp dir");
        let outside = tempfile::tempdir().expect("outside temp dir");
        let target = outside.path().join("target.txt");
        fs::write(&target, b"outside remains").expect("outside target");
        symlink(outside.path(), directory.path().join("escape")).expect("directory symlink");
        let root =
            Dir::open_ambient_dir(directory.path(), ambient_authority()).expect("root capability");

        let error =
            write_attachment_file_in(&root, Path::new("escape/target.txt"), b"replacement", true)
                .expect_err("intermediate symlink escape must be rejected");
        assert!(matches!(error, PublicError::Validation(_)));
        assert_eq!(
            fs::read(target).expect("outside target unchanged"),
            b"outside remains"
        );
    }

    #[test]
    fn attachment_output_rejects_absolute_and_parent_relative_paths() {
        let directory = tempfile::tempdir().expect("temp dir");
        let root =
            Dir::open_ambient_dir(directory.path(), ambient_authority()).expect("root capability");

        assert!(write_attachment_file_in(&root, directory.path(), b"data", false).is_err());
        assert!(
            write_attachment_file_in(&root, Path::new("../outside.txt"), b"data", false).is_err()
        );
    }

    #[cfg(windows)]
    #[test]
    fn attachment_output_rejects_intermediate_directory_reparse_point() {
        use std::os::windows::fs::symlink_dir;

        let directory = tempfile::tempdir().expect("temp dir");
        let outside = tempfile::tempdir().expect("outside temp dir");
        let target = outside.path().join("target.txt");
        fs::write(&target, b"outside remains").expect("outside target");
        if let Err(error) = symlink_dir(outside.path(), directory.path().join("escape")) {
            if error.kind() == io::ErrorKind::PermissionDenied {
                return;
            }
            panic!("directory symlink: {error}");
        }
        let root =
            Dir::open_ambient_dir(directory.path(), ambient_authority()).expect("root capability");

        let error =
            write_attachment_file_in(&root, Path::new("escape/target.txt"), b"replacement", true)
                .expect_err("intermediate reparse point escape must be rejected");
        assert!(matches!(error, PublicError::Validation(_)));
        assert_eq!(
            fs::read(target).expect("outside target unchanged"),
            b"outside remains"
        );
    }
}
