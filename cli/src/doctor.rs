use std::fs::{self, File};
use std::future::Future;
use std::path::{Path, PathBuf};

use crate::operator_config::OperatorSettingsStore;
use crate::output::{CliResult, OutputFormat, print_json, terminal_line};
use crate::table::sanitize_cell;
use crate::terminal::{self, StyleRole, with_progress};
use chrono::{DateTime, Utc};
use reqwest::Url;
use sealtask_client_auth::{
    Credentials, PersistedDataKeyStatus, active_profile, config_dir, credentials_path,
    load_credentials, normalize_api_url,
};
use sealtask_client_core::PublicError;
use sealtask_client_runtime::RuntimeClient;
use serde::Serialize;
use uuid::Uuid;

const DIAGNOSTIC_SCHEMA_VERSION: u8 = 1;
const CONTEXT_FILE_NAME: &str = "context.json";
const MAX_CREDENTIALS_FILE_BYTES: u64 = 1024 * 1024;
const MAX_CONTEXT_FILE_BYTES: u64 = 64 * 1024;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DoctorOptions {
    pub(crate) offline: bool,
    pub(crate) strict: bool,
    pub(crate) include_keychain: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DiagnosticCheckStatus {
    Pass,
    Warning,
    Fail,
    Skipped,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DiagnosticOutcomeV1 {
    Passed,
    PassedWithWarnings,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DiagnosticCheckV1 {
    pub(crate) id: String,
    pub(crate) status: DiagnosticCheckStatus,
    pub(crate) summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) remediation: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DiagnosticSummaryV1 {
    pub(crate) passed: usize,
    pub(crate) warnings: usize,
    pub(crate) failed: usize,
    pub(crate) skipped: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DiagnosticEnvironmentV1 {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) profile: Option<String>,
    pub(crate) api_target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) config_directory: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) credentials_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) context_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) credential_user_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) authenticated_user_id: Option<Uuid>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DiagnosticReportV1 {
    pub(crate) schema_version: u8,
    pub(crate) generated_at: DateTime<Utc>,
    pub(crate) options: DoctorOptions,
    pub(crate) environment: DiagnosticEnvironmentV1,
    pub(crate) outcome: DiagnosticOutcomeV1,
    pub(crate) summary: DiagnosticSummaryV1,
    pub(crate) checks: Vec<DiagnosticCheckV1>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DoctorFailure {
    FailedChecks,
    StrictWarnings,
}

impl std::fmt::Display for DoctorFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FailedChecks => formatter.write_str("one or more diagnostic checks failed"),
            Self::StrictWarnings => {
                formatter.write_str("strict diagnostics treat warnings as failures")
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DoctorRunResult {
    report: DiagnosticReportV1,
    failure: Option<DoctorFailure>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct HealthProbeFailure {
    error_code: String,
}

impl DoctorRunResult {
    #[must_use]
    pub(crate) const fn report(&self) -> &DiagnosticReportV1 {
        &self.report
    }

    pub(crate) fn status(&self) -> Result<(), DoctorFailure> {
        self.failure.map_or(Ok(()), Err)
    }
}

pub(crate) fn print_doctor_report(
    report: &DiagnosticReportV1,
    format: OutputFormat,
) -> CliResult<()> {
    match format {
        OutputFormat::Json | OutputFormat::JsonPretty => {
            print_json(report, format, "serializing doctor report should succeed")
        }
        OutputFormat::Table => {
            println!(
                "{}",
                terminal::style_stdout("SealTask Doctor", StyleRole::Heading)
            );
            println!("{}", "=".repeat(60));
            println!(
                "Profile: {}",
                diagnostic_terminal_line(
                    report
                        .environment
                        .profile
                        .as_deref()
                        .unwrap_or("<unavailable>")
                )
            );
            println!(
                "API target: {}",
                diagnostic_terminal_line(&report.environment.api_target)
            );
            if let Some(path) = report.environment.config_directory.as_deref() {
                println!("Config: {}", diagnostic_terminal_line(path));
            }
            if let Some(path) = report.environment.credentials_path.as_deref() {
                println!("Credentials: {}", diagnostic_terminal_line(path));
            }
            if let Some(path) = report.environment.context_path.as_deref() {
                println!("Context: {}", diagnostic_terminal_line(path));
            }
            if let Some(user_id) = report.environment.credential_user_id {
                println!("Credential user: {user_id}");
            }
            if let Some(user_id) = report.environment.authenticated_user_id {
                println!("Authenticated user: {user_id}");
            }

            println!();
            println!("{:<4}  {:<35}  Summary", "STAT", "Check");
            println!("{}", "-".repeat(88));
            for check in &report.checks {
                let error_code = check
                    .error_code
                    .as_deref()
                    .map(|code| format!(" [{}]", diagnostic_terminal_line(code)))
                    .unwrap_or_default();
                println!(
                    "{:<4}  {:<35}  {}{}",
                    terminal::style_stdout(check.status.label(), check.status.style_role()),
                    diagnostic_terminal_line(&check.id),
                    diagnostic_terminal_line(&check.summary),
                    error_code
                );
                if let Some(remediation) = check.remediation.as_deref() {
                    println!("      Next: {}", diagnostic_terminal_line(remediation));
                }
            }

            println!();
            println!(
                "Checks: {} passed, {} warning(s), {} failed, {} skipped",
                report.summary.passed,
                report.summary.warnings,
                report.summary.failed,
                report.summary.skipped
            );
            println!(
                "Outcome: {}",
                terminal::style_stdout(report.outcome.label(), report.outcome.style_role())
            );
            Ok(())
        }
    }
}

impl DiagnosticCheckStatus {
    const fn label(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Warning => "WARN",
            Self::Fail => "FAIL",
            Self::Skipped => "SKIP",
        }
    }

    const fn style_role(self) -> StyleRole {
        match self {
            Self::Pass => StyleRole::Success,
            Self::Warning => StyleRole::Warning,
            Self::Fail => StyleRole::Error,
            Self::Skipped => StyleRole::Muted,
        }
    }
}

impl DiagnosticOutcomeV1 {
    const fn label(self) -> &'static str {
        match self {
            Self::Passed => "PASSED",
            Self::PassedWithWarnings => "PASSED WITH WARNINGS",
            Self::Failed => "FAILED",
        }
    }

    const fn style_role(self) -> StyleRole {
        match self {
            Self::Passed => StyleRole::Success,
            Self::PassedWithWarnings => StyleRole::Warning,
            Self::Failed => StyleRole::Error,
        }
    }
}

fn diagnostic_terminal_line(value: &str) -> String {
    terminal_line(&sanitize_cell(value))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FileReadiness {
    Missing,
    Readable,
    Unsafe,
}

/// Collects local and, unless `offline`, authenticated API diagnostics.
///
/// The report contains only stable classifications, safe paths, timestamps,
/// and user IDs. Arbitrary operating-system, keychain, daemon, and backend
/// error prose is deliberately replaced by stable error codes.
pub(crate) async fn run_doctor(
    runtime: &RuntimeClient,
    operator_config_root: &Path,
    options: DoctorOptions,
) -> DoctorRunResult {
    run_doctor_with_probes(
        runtime,
        operator_config_root,
        options,
        || probe_health(runtime),
        || async { runtime.get_me().await.map(|identity| identity.id) },
    )
    .await
}

async fn run_doctor_with_probes<H, HFut, I, IFut>(
    runtime: &RuntimeClient,
    operator_config_root: &Path,
    options: DoctorOptions,
    health_probe: H,
    identity_probe: I,
) -> DoctorRunResult
where
    H: FnOnce() -> HFut,
    HFut: Future<Output = Result<u16, HealthProbeFailure>>,
    I: FnOnce() -> IFut,
    IFut: Future<Output = Result<Uuid, PublicError>>,
{
    let mut checks = Vec::new();
    inspect_operator_settings(operator_config_root, &mut checks);
    let profile = resolve_profile(&mut checks);
    let config_directory = resolve_config_directory(&mut checks);
    let credentials_file = resolve_credentials_path(&mut checks);
    let context_file = config_directory
        .as_ref()
        .map(|directory| directory.join(CONTEXT_FILE_NAME));

    let config_readable = config_directory
        .as_deref()
        .is_some_and(|directory| inspect_config_directory(directory, &mut checks));
    let credentials_readiness = match credentials_file.as_deref() {
        Some(path) if config_readable => inspect_credentials_file(path, &mut checks),
        Some(_) => {
            checks.push(DiagnosticCheckV1::skipped(
                "credentials.file",
                "Credentials file inspection was skipped because the configuration directory is unavailable.",
            ));
            checks.push(DiagnosticCheckV1::skipped(
                "credentials.permissions",
                "Credentials permissions were not inspected.",
            ));
            FileReadiness::Unsafe
        }
        None => {
            checks.push(DiagnosticCheckV1::skipped(
                "credentials.file",
                "Credentials path resolution failed.",
            ));
            checks.push(DiagnosticCheckV1::skipped(
                "credentials.permissions",
                "Credentials permissions were not inspected.",
            ));
            FileReadiness::Unsafe
        }
    };
    match context_file.as_deref() {
        Some(path) if config_readable => inspect_context_file(path, &mut checks),
        Some(_) | None => {
            checks.push(DiagnosticCheckV1::skipped(
                "context.file",
                "Project context inspection was skipped because its path is unavailable.",
            ));
            checks.push(DiagnosticCheckV1::skipped(
                "context.permissions",
                "Project context permissions were not inspected.",
            ));
        }
    }

    let credentials = load_and_check_credentials(
        runtime,
        credentials_readiness,
        credentials_file.as_deref(),
        &mut checks,
    );
    let credential_user_id = credentials.as_ref().map(|credentials| credentials.user_id);
    let session_usable = check_session(credentials.as_ref(), &mut checks);

    check_unlock_daemon(runtime, credentials.as_ref(), &mut checks);
    check_keychain(
        runtime,
        credentials.as_ref(),
        options.include_keychain,
        &mut checks,
    );

    let authenticated_user_id = append_api_checks(
        &mut checks,
        options.offline,
        credential_user_id,
        session_usable,
        health_probe,
        identity_probe,
    )
    .await;
    let summary = summarize(&checks);
    let failure = diagnostic_failure(&summary, options.strict);
    let outcome = diagnostic_outcome(&summary, failure);

    DoctorRunResult {
        report: DiagnosticReportV1 {
            schema_version: DIAGNOSTIC_SCHEMA_VERSION,
            generated_at: Utc::now(),
            options,
            environment: DiagnosticEnvironmentV1 {
                profile,
                api_target: safe_api_target(runtime.api_url()),
                config_directory: config_directory.as_deref().map(display_path),
                credentials_path: credentials_file.as_deref().map(display_path),
                context_path: context_file.as_deref().map(display_path),
                credential_user_id,
                authenticated_user_id,
            },
            outcome,
            summary,
            checks,
        },
        failure,
    }
}

fn inspect_operator_settings(path: &Path, checks: &mut Vec<DiagnosticCheckV1>) {
    let result = OperatorSettingsStore::new(path).and_then(|store| store.load());
    match result {
        Ok(_) => checks.push(DiagnosticCheckV1::pass(
            "configuration.operator_settings",
            "Operator settings are valid or have not been initialized.",
        )),
        Err(_) => checks.push(
            DiagnosticCheckV1::fail(
                "configuration.operator_settings",
                "Operator settings could not be loaded safely.",
            )
            .with_error_code("operator_settings_invalid")
            .with_remediation(
                "Upgrade SealTask if a newer version wrote the file; otherwise move operator-settings.json aside and rerun 'sealtask profile use'.",
            ),
        ),
    }
}

impl DiagnosticCheckV1 {
    fn new(
        id: impl Into<String>,
        status: DiagnosticCheckStatus,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            status,
            summary: summary.into(),
            error_code: None,
            remediation: None,
        }
    }

    fn pass(id: impl Into<String>, summary: impl Into<String>) -> Self {
        Self::new(id, DiagnosticCheckStatus::Pass, summary)
    }

    fn warning(id: impl Into<String>, summary: impl Into<String>) -> Self {
        Self::new(id, DiagnosticCheckStatus::Warning, summary)
    }

    fn fail(id: impl Into<String>, summary: impl Into<String>) -> Self {
        Self::new(id, DiagnosticCheckStatus::Fail, summary)
    }

    fn skipped(id: impl Into<String>, summary: impl Into<String>) -> Self {
        Self::new(id, DiagnosticCheckStatus::Skipped, summary)
    }

    #[must_use]
    fn with_error_code(mut self, error_code: impl Into<String>) -> Self {
        self.error_code = Some(error_code.into());
        self
    }

    #[must_use]
    fn with_remediation(mut self, remediation: impl Into<String>) -> Self {
        self.remediation = Some(remediation.into());
        self
    }
}

fn resolve_profile(checks: &mut Vec<DiagnosticCheckV1>) -> Option<String> {
    match active_profile() {
        Ok(profile) => {
            checks.push(DiagnosticCheckV1::pass(
                "configuration.profile",
                "The active profile was resolved.",
            ));
            Some(profile)
        }
        Err(error) => {
            checks.push(
                DiagnosticCheckV1::fail(
                    "configuration.profile",
                    "The active profile could not be resolved.",
                )
                .with_error_code(error.code())
                .with_remediation("Use a valid --profile or SEALTASK_PROFILE value."),
            );
            None
        }
    }
}

fn resolve_config_directory(checks: &mut Vec<DiagnosticCheckV1>) -> Option<PathBuf> {
    match config_dir().and_then(absolute_path) {
        Ok(path) => {
            checks.push(DiagnosticCheckV1::pass(
                "configuration.directory_path",
                "The configuration directory path was resolved.",
            ));
            Some(path)
        }
        Err(error) => {
            checks.push(
                DiagnosticCheckV1::fail(
                    "configuration.directory_path",
                    "The configuration directory path could not be resolved.",
                )
                .with_error_code(error.code())
                .with_remediation("Set SEALTASK_CONFIG_DIR to an accessible directory."),
            );
            None
        }
    }
}

fn resolve_credentials_path(checks: &mut Vec<DiagnosticCheckV1>) -> Option<PathBuf> {
    match credentials_path().and_then(absolute_path) {
        Ok(path) => {
            checks.push(DiagnosticCheckV1::pass(
                "configuration.credentials_path",
                "The credentials file path was resolved.",
            ));
            Some(path)
        }
        Err(error) => {
            checks.push(
                DiagnosticCheckV1::fail(
                    "configuration.credentials_path",
                    "The credentials file path could not be resolved.",
                )
                .with_error_code(error.code()),
            );
            None
        }
    }
}

fn absolute_path(path: PathBuf) -> Result<PathBuf, PublicError> {
    if path.is_absolute() {
        return Ok(path);
    }
    std::env::current_dir()
        .map(|directory| directory.join(path))
        .map_err(|_| PublicError::unexpected("current directory is unavailable"))
}

fn inspect_config_directory(path: &Path, checks: &mut Vec<DiagnosticCheckV1>) -> bool {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            checks.push(
                DiagnosticCheckV1::warning(
                    "configuration.directory",
                    "The configuration directory does not exist yet.",
                )
                .with_remediation("Run 'sealtask auth login' to initialize this profile."),
            );
            checks.push(DiagnosticCheckV1::skipped(
                "configuration.directory_permissions",
                "Configuration directory permissions were not inspected.",
            ));
            return false;
        }
        Err(_) => {
            checks.push(
                DiagnosticCheckV1::fail(
                    "configuration.directory",
                    "The configuration directory metadata is unreadable.",
                )
                .with_error_code("io_error")
                .with_remediation("Make the configuration directory readable by this user."),
            );
            checks.push(DiagnosticCheckV1::skipped(
                "configuration.directory_permissions",
                "Configuration directory permissions were not inspected.",
            ));
            return false;
        }
    };

    if metadata.file_type().is_symlink() {
        checks.push(
            DiagnosticCheckV1::fail(
                "configuration.directory",
                "The configuration directory must not be a symbolic link.",
            )
            .with_error_code("unsafe_symlink")
            .with_remediation("Move the profile into a private, non-symlinked directory."),
        );
        checks.push(DiagnosticCheckV1::skipped(
            "configuration.directory_permissions",
            "Configuration directory permissions were not inspected.",
        ));
        return false;
    }
    if !metadata.is_dir() {
        checks.push(
            DiagnosticCheckV1::fail(
                "configuration.directory",
                "The configured path is not a directory.",
            )
            .with_error_code("not_a_directory"),
        );
        checks.push(DiagnosticCheckV1::skipped(
            "configuration.directory_permissions",
            "Configuration directory permissions were not inspected.",
        ));
        return false;
    }
    if fs::read_dir(path).is_err() {
        checks.push(
            DiagnosticCheckV1::fail(
                "configuration.directory",
                "The configuration directory is not readable.",
            )
            .with_error_code("io_error")
            .with_remediation("Grant the current user read and execute access."),
        );
        check_private_permissions(
            "configuration.directory_permissions",
            &metadata,
            "The configuration directory",
            checks,
        );
        return false;
    }

    checks.push(DiagnosticCheckV1::pass(
        "configuration.directory",
        "The configuration directory is readable.",
    ));
    check_private_permissions(
        "configuration.directory_permissions",
        &metadata,
        "The configuration directory",
        checks,
    );
    true
}

fn inspect_credentials_file(path: &Path, checks: &mut Vec<DiagnosticCheckV1>) -> FileReadiness {
    inspect_regular_file(
        path,
        "credentials.file",
        "credentials.permissions",
        "The credentials file",
        MAX_CREDENTIALS_FILE_BYTES,
        true,
        checks,
    )
}

fn inspect_context_file(path: &Path, checks: &mut Vec<DiagnosticCheckV1>) {
    let readiness = inspect_regular_file(
        path,
        "context.file",
        "context.permissions",
        "The project context file",
        MAX_CONTEXT_FILE_BYTES,
        false,
        checks,
    );
    if readiness != FileReadiness::Readable {
        return;
    }

    let valid_json = File::open(path)
        .ok()
        .and_then(|file| serde_json::from_reader::<_, serde_json::Value>(file).ok())
        .is_some_and(|value| {
            value.is_object()
                && value
                    .get("schemaVersion")
                    .and_then(serde_json::Value::as_u64)
                    .is_some()
        });
    if valid_json {
        checks.push(DiagnosticCheckV1::pass(
            "context.format",
            "The project context has a recognized JSON envelope.",
        ));
    } else {
        checks.push(
            DiagnosticCheckV1::fail(
                "context.format",
                "The project context is not valid diagnostic-readable JSON.",
            )
            .with_error_code("invalid_context")
            .with_remediation("Run 'sealtask projects clear', then select the project again."),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn inspect_regular_file(
    path: &Path,
    file_check_id: &str,
    permissions_check_id: &str,
    label: &str,
    maximum_bytes: u64,
    missing_is_warning: bool,
    checks: &mut Vec<DiagnosticCheckV1>,
) -> FileReadiness {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let check = if missing_is_warning {
                DiagnosticCheckV1::warning(
                    file_check_id,
                    format!("{label} does not exist for this profile."),
                )
                .with_remediation("Run 'sealtask auth login' for the active API target.")
            } else {
                DiagnosticCheckV1::pass(
                    file_check_id,
                    format!("{label} is absent; no current project is saved."),
                )
            };
            checks.push(check);
            checks.push(DiagnosticCheckV1::skipped(
                permissions_check_id,
                format!("{label} permissions were not inspected."),
            ));
            return FileReadiness::Missing;
        }
        Err(_) => {
            checks.push(
                DiagnosticCheckV1::fail(file_check_id, format!("{label} metadata is unreadable."))
                    .with_error_code("io_error"),
            );
            checks.push(DiagnosticCheckV1::skipped(
                permissions_check_id,
                format!("{label} permissions were not inspected."),
            ));
            return FileReadiness::Unsafe;
        }
    };

    if metadata.file_type().is_symlink() {
        checks.push(
            DiagnosticCheckV1::fail(
                file_check_id,
                format!("{label} must not be a symbolic link."),
            )
            .with_error_code("unsafe_symlink"),
        );
        checks.push(DiagnosticCheckV1::skipped(
            permissions_check_id,
            format!("{label} permissions were not inspected."),
        ));
        return FileReadiness::Unsafe;
    }
    if !metadata.is_file() {
        checks.push(
            DiagnosticCheckV1::fail(file_check_id, format!("{label} is not a regular file."))
                .with_error_code("not_a_regular_file"),
        );
        checks.push(DiagnosticCheckV1::skipped(
            permissions_check_id,
            format!("{label} permissions were not inspected."),
        ));
        return FileReadiness::Unsafe;
    }
    if metadata.len() > maximum_bytes {
        checks.push(
            DiagnosticCheckV1::fail(
                file_check_id,
                format!("{label} exceeds the diagnostic safety limit."),
            )
            .with_error_code("file_too_large"),
        );
        check_private_permissions(permissions_check_id, &metadata, label, checks);
        return FileReadiness::Unsafe;
    }
    if File::open(path).is_err() {
        checks.push(
            DiagnosticCheckV1::fail(file_check_id, format!("{label} is not readable."))
                .with_error_code("io_error"),
        );
        check_private_permissions(permissions_check_id, &metadata, label, checks);
        return FileReadiness::Unsafe;
    }

    checks.push(DiagnosticCheckV1::pass(
        file_check_id,
        format!("{label} is a readable regular file."),
    ));
    check_private_permissions(permissions_check_id, &metadata, label, checks);
    FileReadiness::Readable
}

#[cfg(unix)]
fn check_private_permissions(
    id: &str,
    metadata: &fs::Metadata,
    label: &str,
    checks: &mut Vec<DiagnosticCheckV1>,
) {
    use std::os::unix::fs::PermissionsExt;

    if metadata.permissions().mode() & 0o077 == 0 {
        checks.push(DiagnosticCheckV1::pass(
            id,
            format!("{label} is private to the current user."),
        ));
    } else {
        checks.push(
            DiagnosticCheckV1::warning(
                id,
                format!("{label} is accessible to group or other users."),
            )
            .with_error_code("permissions_too_broad")
            .with_remediation("Restrict the path to owner-only permissions."),
        );
    }
}

#[cfg(not(unix))]
fn check_private_permissions(
    id: &str,
    _metadata: &fs::Metadata,
    label: &str,
    checks: &mut Vec<DiagnosticCheckV1>,
) {
    checks.push(DiagnosticCheckV1::skipped(
        id,
        format!("{label} Unix permission bits are unavailable on this platform."),
    ));
}

fn load_and_check_credentials(
    runtime: &RuntimeClient,
    readiness: FileReadiness,
    credentials_file: Option<&Path>,
    checks: &mut Vec<DiagnosticCheckV1>,
) -> Option<Credentials> {
    match readiness {
        FileReadiness::Missing => {
            checks.push(
                DiagnosticCheckV1::warning(
                    "credentials.state",
                    "No credentials are stored for this profile.",
                )
                .with_remediation("Run 'sealtask auth login'."),
            );
            None
        }
        FileReadiness::Unsafe => {
            checks.push(DiagnosticCheckV1::skipped(
                "credentials.state",
                "Credential parsing was skipped because the file is unsafe or unreadable.",
            ));
            None
        }
        FileReadiness::Readable => match load_credentials() {
            Ok(Some(credentials)) => {
                if normalize_api_url(&credentials.api_url) == normalize_api_url(runtime.api_url()) {
                    checks.push(DiagnosticCheckV1::pass(
                        "credentials.state",
                        "Stored credentials match the active API target.",
                    ));
                    Some(credentials)
                } else {
                    checks.push(
                        DiagnosticCheckV1::warning(
                            "credentials.state",
                            "Stored credentials belong to a different API target.",
                        )
                        .with_error_code("api_target_mismatch")
                        .with_remediation("Switch profiles or sign in to the active API target."),
                    );
                    None
                }
            }
            Ok(None) => {
                let summary = if credentials_file.is_some_and(Path::exists) {
                    "The credentials file disappeared during inspection."
                } else {
                    "No credentials are stored for this profile."
                };
                checks.push(
                    DiagnosticCheckV1::warning("credentials.state", summary)
                        .with_error_code("credentials_changed")
                        .with_remediation("Retry the command or sign in again."),
                );
                None
            }
            Err(error) => {
                checks.push(
                    DiagnosticCheckV1::fail(
                        "credentials.state",
                        "The credentials file could not be parsed safely.",
                    )
                    .with_error_code(error.code())
                    .with_remediation("Run 'sealtask auth logout', then sign in again."),
                );
                None
            }
        },
    }
}

fn check_session(credentials: Option<&Credentials>, checks: &mut Vec<DiagnosticCheckV1>) -> bool {
    let Some(credentials) = credentials else {
        checks.push(DiagnosticCheckV1::skipped(
            "session.state",
            "Session state is unavailable without matching credentials.",
        ));
        return false;
    };
    if credentials.is_refresh_expired() {
        checks.push(
            DiagnosticCheckV1::fail(
                "session.state",
                "The stored authentication session has expired.",
            )
            .with_error_code("refresh_expired")
            .with_remediation("Run 'sealtask auth login'."),
        );
        false
    } else if credentials.is_access_expired() {
        checks.push(DiagnosticCheckV1::pass(
            "session.state",
            "The access credential is expired but the session can refresh automatically.",
        ));
        true
    } else {
        checks.push(DiagnosticCheckV1::pass(
            "session.state",
            "The stored authentication session is active.",
        ));
        true
    }
}

fn check_unlock_daemon(
    runtime: &RuntimeClient,
    credentials: Option<&Credentials>,
    checks: &mut Vec<DiagnosticCheckV1>,
) {
    if credentials.is_none() {
        checks.push(DiagnosticCheckV1::skipped(
            "unlock.daemon",
            "Unlock state is unavailable without matching credentials.",
        ));
        return;
    }
    match runtime.unlock_status() {
        Ok(status) if status.unlocked => checks.push(DiagnosticCheckV1::pass(
            "unlock.daemon",
            "The local unlock daemon has an active session for these credentials.",
        )),
        Ok(_) => checks.push(DiagnosticCheckV1::pass(
            "unlock.daemon",
            "The local unlock daemon is locked; this is valid when unlocking on demand.",
        )),
        Err(error) => checks.push(
            DiagnosticCheckV1::warning(
                "unlock.daemon",
                "The local unlock daemon status could not be determined.",
            )
            .with_error_code(error.code())
            .with_remediation("Retry 'sealtask auth unlock' if decrypted commands fail."),
        ),
    }
}

fn check_keychain(
    runtime: &RuntimeClient,
    credentials: Option<&Credentials>,
    include_keychain: bool,
    checks: &mut Vec<DiagnosticCheckV1>,
) {
    if !include_keychain {
        checks.push(DiagnosticCheckV1::skipped(
            "unlock.keychain",
            "Keychain inspection was not requested.",
        ));
        return;
    }
    if credentials.is_none() {
        checks.push(DiagnosticCheckV1::skipped(
            "unlock.keychain",
            "Keychain state is unavailable without matching credentials.",
        ));
        return;
    }

    match runtime.persisted_data_key_status() {
        Ok(Some(PersistedDataKeyStatus::Available)) => checks.push(DiagnosticCheckV1::pass(
            "unlock.keychain",
            "A saved unlock key is available.",
        )),
        Ok(Some(PersistedDataKeyStatus::Missing)) => checks.push(DiagnosticCheckV1::pass(
            "unlock.keychain",
            "No saved unlock key is configured.",
        )),
        Ok(Some(PersistedDataKeyStatus::Unavailable(_))) => checks.push(
            DiagnosticCheckV1::warning(
                "unlock.keychain",
                "The saved unlock key backend is unavailable.",
            )
            .with_error_code("keychain_unavailable")
            .with_remediation("Check the platform keychain, then retry."),
        ),
        Ok(None) => checks.push(DiagnosticCheckV1::skipped(
            "unlock.keychain",
            "No keychain target exists for the active API.",
        )),
        Err(error) => checks.push(
            DiagnosticCheckV1::warning(
                "unlock.keychain",
                "The saved unlock key state could not be determined.",
            )
            .with_error_code(error.code())
            .with_remediation("Check the platform keychain, then retry."),
        ),
    }
}

async fn probe_health(runtime: &RuntimeClient) -> Result<u16, HealthProbeFailure> {
    let client = runtime
        .control_plane_http_client()
        .map_err(|error| HealthProbeFailure {
            error_code: error.code().to_string(),
        })?;
    let endpoint = health_endpoint(runtime.api_url())?;
    client
        .get(endpoint)
        .send()
        .await
        .map(|response| response.status().as_u16())
        .map_err(|error| HealthProbeFailure {
            error_code: classify_reqwest_error(&error).to_string(),
        })
}

fn health_endpoint(api_url: &str) -> Result<Url, HealthProbeFailure> {
    let mut url = Url::parse(api_url).map_err(|_| HealthProbeFailure {
        error_code: "invalid_api_target".to_string(),
    })?;
    if url.cannot_be_a_base()
        || url.set_username("").is_err()
        || url.set_password(None).is_err()
        || url.host_str().is_none()
    {
        return Err(HealthProbeFailure {
            error_code: "invalid_api_target".to_string(),
        });
    }
    url.set_query(None);
    url.set_fragment(None);
    url.set_path("/health");
    Ok(url)
}

fn classify_reqwest_error(error: &reqwest::Error) -> &'static str {
    if error.is_timeout() {
        "transport_timeout"
    } else if error.is_connect() {
        "transport_connect"
    } else if error.is_request() {
        "transport_request"
    } else if error.is_body() {
        "transport_body"
    } else {
        "transport_other"
    }
}

async fn append_api_checks<H, HFut, I, IFut>(
    checks: &mut Vec<DiagnosticCheckV1>,
    offline: bool,
    credential_user_id: Option<Uuid>,
    session_usable: bool,
    health_probe: H,
    identity_probe: I,
) -> Option<Uuid>
where
    H: FnOnce() -> HFut,
    HFut: Future<Output = Result<u16, HealthProbeFailure>>,
    I: FnOnce() -> IFut,
    IFut: Future<Output = Result<Uuid, PublicError>>,
{
    if offline {
        checks.push(DiagnosticCheckV1::skipped(
            "api.reachability",
            "API reachability was skipped in offline mode.",
        ));
        checks.push(DiagnosticCheckV1::skipped(
            "api.health",
            "API health was skipped in offline mode.",
        ));
        checks.push(DiagnosticCheckV1::skipped(
            "api.identity",
            "Authenticated identity was skipped in offline mode.",
        ));
        return None;
    }

    match with_progress("Checking API health…", health_probe()).await {
        Ok(status) => {
            checks.push(DiagnosticCheckV1::pass(
                "api.reachability",
                "The API health endpoint is reachable.",
            ));
            if (200..300).contains(&status) {
                checks.push(DiagnosticCheckV1::pass(
                    "api.health",
                    "The API health endpoint reports healthy.",
                ));
            } else {
                checks.push(
                    DiagnosticCheckV1::fail(
                        "api.health",
                        "The API health endpoint returned an unhealthy status.",
                    )
                    .with_error_code(format!("http_status_{status}"))
                    .with_remediation("Check API and database health before retrying."),
                );
            }
        }
        Err(error) => {
            checks.push(
                DiagnosticCheckV1::fail(
                    "api.reachability",
                    "The API health endpoint could not be reached.",
                )
                .with_error_code(error.error_code)
                .with_remediation("Check network access, the API target, and any proxy."),
            );
            checks.push(DiagnosticCheckV1::skipped(
                "api.health",
                "API health is unavailable because the reachability probe failed.",
            ));
            checks.push(DiagnosticCheckV1::skipped(
                "api.identity",
                "Authenticated identity was skipped because the API is unreachable.",
            ));
            return None;
        }
    }

    let Some(credential_user_id) = credential_user_id else {
        checks.push(DiagnosticCheckV1::skipped(
            "api.identity",
            "Authenticated identity requires matching credentials.",
        ));
        return None;
    };
    if !session_usable {
        checks.push(DiagnosticCheckV1::skipped(
            "api.identity",
            "Authenticated identity was skipped because the session has expired.",
        ));
        return None;
    }

    match with_progress("Checking authenticated identity…", identity_probe()).await {
        Ok(authenticated_user_id) => {
            if authenticated_user_id == credential_user_id {
                checks.push(DiagnosticCheckV1::pass(
                    "api.identity",
                    "The authenticated API identity matches the stored credential identity.",
                ));
            } else {
                checks.push(
                    DiagnosticCheckV1::fail(
                        "api.identity",
                        "The authenticated API identity does not match the stored credential identity.",
                    )
                    .with_error_code("identity_mismatch")
                    .with_remediation("Sign out and authenticate this profile again."),
                );
            }
            Some(authenticated_user_id)
        }
        Err(error) => {
            checks.push(
                DiagnosticCheckV1::fail(
                    "api.identity",
                    "The authenticated identity request failed.",
                )
                .with_error_code(error.code())
                .with_remediation("Verify the API target and authenticate again."),
            );
            None
        }
    }
}

fn summarize(checks: &[DiagnosticCheckV1]) -> DiagnosticSummaryV1 {
    let mut summary = DiagnosticSummaryV1 {
        passed: 0,
        warnings: 0,
        failed: 0,
        skipped: 0,
    };
    for check in checks {
        match check.status {
            DiagnosticCheckStatus::Pass => summary.passed += 1,
            DiagnosticCheckStatus::Warning => summary.warnings += 1,
            DiagnosticCheckStatus::Fail => summary.failed += 1,
            DiagnosticCheckStatus::Skipped => summary.skipped += 1,
        }
    }
    summary
}

fn diagnostic_failure(summary: &DiagnosticSummaryV1, strict: bool) -> Option<DoctorFailure> {
    if summary.failed > 0 {
        Some(DoctorFailure::FailedChecks)
    } else if strict && summary.warnings > 0 {
        Some(DoctorFailure::StrictWarnings)
    } else {
        None
    }
}

fn diagnostic_outcome(
    summary: &DiagnosticSummaryV1,
    failure: Option<DoctorFailure>,
) -> DiagnosticOutcomeV1 {
    match (failure, summary.warnings) {
        (Some(_), _) => DiagnosticOutcomeV1::Failed,
        (None, 0) => DiagnosticOutcomeV1::Passed,
        (None, _) => DiagnosticOutcomeV1::PassedWithWarnings,
    }
}

fn safe_api_target(value: &str) -> String {
    let Ok(mut url) = Url::parse(value) else {
        return "<invalid>".to_string();
    };
    if url.cannot_be_a_base()
        || url.set_username("").is_err()
        || url.set_password(None).is_err()
        || url.host_str().is_none()
    {
        return "<invalid>".to_string();
    }
    url.set_query(None);
    url.set_fragment(None);
    url.set_path("");
    url.to_string().trim_end_matches('/').to_string()
}

fn display_path(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn strict_warnings_produce_a_failed_status_without_reclassifying_checks() {
        let checks = vec![
            DiagnosticCheckV1::pass("one", "passed"),
            DiagnosticCheckV1::warning("two", "warning"),
        ];
        let summary = summarize(&checks);
        let failure = diagnostic_failure(&summary, true);
        let result = DoctorRunResult {
            report: DiagnosticReportV1 {
                schema_version: DIAGNOSTIC_SCHEMA_VERSION,
                generated_at: Utc::now(),
                options: DoctorOptions {
                    offline: true,
                    strict: true,
                    include_keychain: false,
                },
                environment: empty_environment(),
                outcome: diagnostic_outcome(&summary, failure),
                summary,
                checks,
            },
            failure,
        };

        assert_eq!(result.status(), Err(DoctorFailure::StrictWarnings));
        assert_eq!(
            result.report().checks[1].status,
            DiagnosticCheckStatus::Warning
        );
        assert_eq!(result.report().outcome, DiagnosticOutcomeV1::Failed);
        assert_eq!(diagnostic_failure(&result.report().summary, false), None);
        assert_eq!(
            diagnostic_outcome(&result.report().summary, None),
            DiagnosticOutcomeV1::PassedWithWarnings
        );
    }

    #[tokio::test]
    async fn offline_api_checks_never_invoke_either_network_probe() {
        let health_called = AtomicBool::new(false);
        let identity_called = AtomicBool::new(false);
        let mut checks = Vec::new();

        let authenticated_user_id = append_api_checks(
            &mut checks,
            true,
            Some(Uuid::from_u128(1)),
            true,
            || async {
                health_called.store(true, Ordering::SeqCst);
                Ok::<_, HealthProbeFailure>(200)
            },
            || async {
                identity_called.store(true, Ordering::SeqCst);
                Ok::<_, PublicError>(Uuid::from_u128(1))
            },
        )
        .await;

        assert_eq!(authenticated_user_id, None);
        assert!(!health_called.load(Ordering::SeqCst));
        assert!(!identity_called.load(Ordering::SeqCst));
        assert!(
            checks
                .iter()
                .all(|check| check.status == DiagnosticCheckStatus::Skipped)
        );
    }

    #[tokio::test]
    async fn online_health_runs_without_credentials_but_identity_does_not() {
        let health_called = AtomicBool::new(false);
        let identity_called = AtomicBool::new(false);
        let mut checks = Vec::new();

        let authenticated_user_id = append_api_checks(
            &mut checks,
            false,
            None,
            false,
            || async {
                health_called.store(true, Ordering::SeqCst);
                Ok::<_, HealthProbeFailure>(200)
            },
            || async {
                identity_called.store(true, Ordering::SeqCst);
                Ok::<_, PublicError>(Uuid::from_u128(1))
            },
        )
        .await;

        assert_eq!(authenticated_user_id, None);
        assert!(health_called.load(Ordering::SeqCst));
        assert!(!identity_called.load(Ordering::SeqCst));
        assert_eq!(
            checks
                .iter()
                .map(|check| (&*check.id, check.status))
                .collect::<Vec<_>>(),
            [
                ("api.reachability", DiagnosticCheckStatus::Pass),
                ("api.health", DiagnosticCheckStatus::Pass),
                ("api.identity", DiagnosticCheckStatus::Skipped),
            ]
        );
    }

    #[test]
    fn health_endpoint_strips_url_credentials_query_and_fragment() {
        let endpoint =
            health_endpoint("https://operator:password@example.com/base?token=secret#ciphertext")
                .expect("health URL");

        assert_eq!(endpoint.as_str(), "https://example.com/health");
    }

    #[test]
    fn human_diagnostic_lines_strip_controls_and_bidi_overrides() {
        assert_eq!(
            diagnostic_terminal_line("safe\nforged\u{1b}[31m\u{202e}reversed\u{202c}"),
            "safe forged[31mreversed"
        );
    }

    #[test]
    fn serialized_report_contains_no_secret_bearing_fields() {
        let report = DiagnosticReportV1 {
            schema_version: DIAGNOSTIC_SCHEMA_VERSION,
            generated_at: Utc::now(),
            options: DoctorOptions::default(),
            environment: DiagnosticEnvironmentV1 {
                profile: Some("default".to_string()),
                api_target: safe_api_target(
                    "https://operator:password@example.com/api?token=secret#ciphertext",
                ),
                config_directory: Some("/tmp/sealtask".to_string()),
                credentials_path: Some("/tmp/sealtask/credentials.json".to_string()),
                context_path: Some("/tmp/sealtask/context.json".to_string()),
                credential_user_id: Some(Uuid::from_u128(1)),
                authenticated_user_id: Some(Uuid::from_u128(1)),
            },
            outcome: DiagnosticOutcomeV1::Passed,
            summary: DiagnosticSummaryV1 {
                passed: 1,
                warnings: 0,
                failed: 0,
                skipped: 0,
            },
            checks: vec![DiagnosticCheckV1::pass("safe", "No secret values.")],
        };

        let serialized = serde_json::to_string(&report).expect("serialize report");
        for forbidden in [
            "operator",
            "password",
            "token=secret",
            "ciphertext",
            "private@example.com",
        ] {
            assert!(!serialized.contains(forbidden));
        }
        assert!(serialized.contains("https://example.com"));
    }

    #[test]
    fn remote_backend_prose_is_replaced_with_a_stable_error_code() {
        let error = PublicError::validation(
            "backend prose containing token-secret and private@example.com",
        );
        let check = DiagnosticCheckV1::fail("api.identity", "Identity request failed.")
            .with_error_code(error.code());
        let serialized = serde_json::to_string(&check).expect("serialize check");

        assert!(serialized.contains("\"errorCode\":\"validation\""));
        assert!(!serialized.contains("token-secret"));
        assert!(!serialized.contains("private@example.com"));
    }

    fn empty_environment() -> DiagnosticEnvironmentV1 {
        DiagnosticEnvironmentV1 {
            profile: None,
            api_target: "https://example.com".to_string(),
            config_directory: None,
            credentials_path: None,
            context_path: None,
            credential_user_id: None,
            authenticated_user_id: None,
        }
    }
}
