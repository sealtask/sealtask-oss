use crate::output::{CliResult, OutputFormat, print_json};
use crate::terminal::{self, StyleRole};
use sealtask_client_auth::{UnlockMode, active_profile, config_dir};
use sealtask_client_crypto::CryptoCapability;
use sealtask_client_runtime::RuntimeClient;
use serde_json::json;

pub(crate) fn run_info(runtime: &RuntimeClient, format: OutputFormat) -> CliResult<()> {
    let payload = json!({
        "apiBaseUrl": runtime.api_url(),
        "activeProfile": active_profile()?,
        "configDirectory": config_dir()?.display().to_string(),
        "commandName": "sealtask",
        "automationProfile": "agent_task_management",
        "jsonContractVersion": 2,
        "outputFormats": ["table", "json", "json-pretty"],
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
        "picker": {
            "command": "sealtask pick",
            "entities": ["project", "task"],
            "selectorFormat": "id:<32-lowercase-hex>",
            "interactiveOnly": true,
            "controllingTerminal": true,
            "externalProcess": false,
            "dynamicNameCompletion": false,
        },
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
            println!("Picker: sealtask pick project|task");
            println!("Task lists: --columns, --sort, --field id|title|url");
            println!("Project details: sealtask projects list --details");
            Ok(())
        }
        OutputFormat::Json | OutputFormat::JsonPretty => {
            print_json(&payload, format, "serializing CLI metadata should succeed")
        }
    }
}
