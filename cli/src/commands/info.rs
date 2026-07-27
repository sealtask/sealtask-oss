use crate::interruption::MUTATION_INTERRUPT_GRACE;
use crate::output::{CliResult, OutputFormat, print_json};
use crate::terminal::{self, StyleRole};
use sealtask_client_api::{MAX_API_RETRIES, MAX_API_RETRY_DELAY};
use sealtask_client_auth::{UnlockMode, active_profile, config_dir};
use sealtask_client_crypto::CryptoCapability;
use sealtask_client_runtime::RuntimeClient;
use serde_json::json;

pub(crate) fn run_info(runtime: &RuntimeClient, format: OutputFormat) -> CliResult<()> {
    let picker = json!({
        "command": "sealtask pick",
        "entities": ["project", "task"],
        "selectorFormat": "id:<32-lowercase-hex>",
        "interactiveOnly": false,
        "controllingTerminal": true,
        "controllingTerminalAppliesTo": "interactive-selection-only",
        "externalProcess": false,
        "dynamicNameCompletion": false,
        "interactiveSelection": {
            "projectCommand": "sealtask pick project",
            "projectSelectorOmitted": true,
            "taskCommand": "sealtask pick task",
            "controllingTerminal": true,
            "jsonOutputSupported": false,
            "nonInteractiveSupported": false,
        },
        "explicitProjectActivation": {
            "command": "sealtask pick project PROJECT",
            "automationSafe": true,
            "controllingTerminal": false,
            "nonInteractiveSupported": true,
            "structuredOutputSupported": true,
            "scopeFlag": "--scope",
            "scopes": ["local", "global"],
        },
    });
    let payload = json!({
        "apiBaseUrl": runtime.api_url(),
        "activeProfile": active_profile()?,
        "configDirectory": config_dir()?.display().to_string(),
        "commandName": "sealtask",
        "automationProfile": "agent_task_management",
        "jsonContractVersion": 2,
        "outputFormats": ["table", "json", "json-pretty", "jsonl"],
        "streaming": {
            "commands": ["tasks watch", "activity follow", "batch run"],
            "machineFormat": "jsonl",
            "recordSchemaVersion": 1,
            "finiteJsonRejected": true,
            "flushEachRecord": true,
            "pagerDisabled": true,
            "redirectedHumanOutput": "append-only",
            "interactiveHumanOutput": "bounded-live-region",
            "interruptionExitCode": 130,
        },
        "audit": {
            "command": "projects audit",
            "activityCommand": "activity follow",
            "maximumPageItems": 100,
            "encryptedPayloadExcluded": true,
        },
        "retries": {
            "configuredRetries": runtime.api_transport_options().retry_policy().max_retries(),
            "maximumRetries": MAX_API_RETRIES,
            "maximumDelaySeconds": MAX_API_RETRY_DELAY.as_secs(),
            "replaySafeRequestsOnly": true,
            "environment": "SEALTASK_RETRY",
        },
        "mutationInterruption": {
            "outsideRequest": "cancel_immediately",
            "inFlightFirstSignal": "wait_for_definitive_response",
            "inFlightSecondSignal": "force_stop",
            "graceSeconds": MUTATION_INTERRUPT_GRACE.as_secs(),
            "forcedExitCode": 130,
            "forcedOutcome": "ambiguous",
            "ambiguousOnlyWhileRequestInFlight": true,
            "credentialRefresh": {
                "firstSignal": "persist_replacement_then_cancel_resource_request",
                "forcedOutcome": "session_ambiguous",
                "reloginGuidance": true,
            },
        },
        "taskDryRun": {
            "commands": ["tasks create", "tasks update"],
            "planSchemaVersion": 1,
            "planType": "taskMutationPlan",
            "willMutate": false,
            "preparesEncryptedRequest": true,
        },
        "readCache": {
            "mode": "online_population_with_explicit_offline_reads",
            "onlineFallback": false,
            "offlineFlag": "--offline",
            "maximumPlaintextBytes": 64 * 1024 * 1024,
            "singleEncryptedFile": true,
            "plaintextSnapshotSidecar": false,
            "coordinationSidecar": "opaque_invalidation_generation",
            "binding": [
                "normalizedApiUrl",
                "userId",
                "profile",
                "dataKeyCiphertextSha256"
            ],
            "controls": ["cache status", "cache verify", "cache clear"],
            "offlineReads": [
                "projects list",
                "projects get",
                "projects sections list",
                "tasks list",
                "tasks get",
                "comments list",
                "notes list",
                "notes get",
                "pick project",
                "pick task",
                "browse"
            ],
            "mutationsRejectedBeforeNetwork": true,
            "snapshotAgeReported": true,
            "rollbackProtection": false,
            "secureDeletionGuaranteed": false,
        },
        "invocationCache": {
            "enabledForCliReads": true,
            "scope": "one_process_invocation",
            "persistentFallback": false,
        },
        "browse": {
            "command": "browse",
            "controllingTerminalOnly": true,
            "decryptedContentOnStdout": false,
            "decryptedContentOnStderr": cfg!(windows),
            "redirectedStandardStreamsReceiveContent": false,
            "windowsRequiresAttendedStdinAndStderr": cfg!(windows),
            "offlineSupported": true,
        },
        "batch": {
            "command": "batch run",
            "inputSchemaVersion": 1,
            "recordSchemaVersion": 1,
            "operations": ["task.create", "task.update"],
            "outputFormats": ["table", "jsonl"],
            "finiteJsonRejected": true,
            "flushEachRecord": true,
            "limits": {
                "maximumLineBytes": 4 * 1024 * 1024,
                "maximumInputBytes": 64 * 1024 * 1024,
                "maximumOperations": 10_000,
                "maximumJobs": 16,
            },
            "checkpoint": {
                "supported": cfg!(any(target_os = "linux", target_os = "macos")),
                "resumeSupported": cfg!(any(target_os = "linux", target_os = "macos")),
                "supportedPlatforms": ["linux", "macos"],
                "unsupportedPlatformsFailClosed": true,
                "canonicalInputSha256Bound": true,
                "operationIdsHashed": true,
                "plaintextExcluded": true,
            },
            "exitCodes": {
                "success": 0,
                "failure": 1,
                "partialFailure": 3,
                "checkpointConflict": 4,
                "interrupted": 130,
            },
        },
        "shellCompletions": ["bash", "zsh", "fish", "powershell"],
        "manualPages": true,
        "taskListing": {
            "columns": [
                "id", "title", "project", "project-id", "priority", "due", "status",
                "comments", "created", "updated",
            ],
            "sortFields": [
                "id", "title", "project", "priority", "due", "status", "created", "updated",
            ],
            "rawFields": ["id", "title", "url"],
            "rawFieldOutput": "newline-delimited",
            "webUrlEnvironment": "SEALTASK_WEB_URL",
            "webUrlDefault": "API origin",
            "crossProjectTableIncludesProject": true,
        },
        "idPrefixSelectors": {
            "minimumHexCharacters": 8,
            "commentFlags": ["--comment-id"],
            "attachmentFlags": ["--attachment-id"],
        },
        "canonicalFlags": {
            "projectListDetails": "--details",
        },
        "editorInput": {
            "commandPrecedence": ["SEALTASK_EDITOR", "VISUAL", "EDITOR", "platform-default"],
            "directProcess": true,
            "controllingTerminal": true,
            "documentFormat": "first-line-title-then-markdown",
            "temporaryPermissions": {
                "posixDirectoryMode": "0700",
                "posixFileMode": "0600",
                "posixModesEnforced": cfg!(unix),
            },
            "workflows": ["tasks create --edit", "tasks edit", "notes edit"],
        },
        "bodyFileInput": {
            "stdinPath": "-",
            "workflows": [
                "tasks create --body-file",
                "tasks update --body-file",
                "comments create --body-file",
            ],
        },
        "picker": picker,
        "terminalPolicies": {
            "color": ["auto", "always", "never"],
            "pager": ["auto", "always", "never"],
            "progress": ["auto", "always", "never"],
            "quietFlag": "--quiet",
            "pagerCommandEnvironment": ["SEALTASK_PAGER", "PAGER"],
            "noColorEnvironment": "NO_COLOR",
        },
        "nonInteractiveFlag": "--non-interactive",
        "authUnlockModes": [
            UnlockMode::SingleCommand.as_str(),
            UnlockMode::Daemon.as_str(),
        ],
        "cryptoCapabilities": [
            CryptoCapability::DataKeyUnwrap.as_str(),
            CryptoCapability::WorkListKeyDecrypt.as_str(),
            CryptoCapability::PayloadSeal.as_str(),
            CryptoCapability::PayloadProof.as_str(),
        ],
        "decryptedReadModel": true,
        "note": "This CLI is intended for agent-friendly task and comment workflows against SealTask.",
    });
    match format {
        OutputFormat::Table => {
            println!(
                "{}",
                terminal::style_stdout("SealTask CLI contract version 2", StyleRole::Heading)
            );
            println!("API: {}", runtime.api_url());
            println!("Profile: {}", active_profile()?);
            println!("Config: {}", config_dir()?.display());
            println!("Editor: SEALTASK_EDITOR > VISUAL > EDITOR");
            println!("Picker: sealtask pick project|task (interactive selection)");
            println!(
                "Project activation: sealtask pick project PROJECT --scope local|global (automation-safe)"
            );
            println!("Task lists: --columns, --sort, --field id|title|url");
            println!("Streams: tasks watch, activity follow (--format jsonl for automation)");
            println!("Audit: projects audit [PROJECT]");
            println!("Dry runs: tasks create|update --dry-run");
            println!("Batch: batch run --input PATH|- [--checkpoint PATH --resume]");
            println!("Offline reads: --offline (encrypted snapshots; never implicit fallback)");
            println!("Cache: cache status|verify|clear");
            println!("Private TTY browser: browse [--offline]");
            println!(
                "Retries: {} replay-safe retry attempt(s)",
                runtime.api_transport_options().retry_policy().max_retries()
            );
            println!("Project details: sealtask projects list --details");
            Ok(())
        }
        OutputFormat::Json | OutputFormat::JsonPretty | OutputFormat::Jsonl => {
            print_json(&payload, format, "serializing CLI metadata should succeed")
        }
    }
}
