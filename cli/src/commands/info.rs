use crate::output::{CliResult, OutputFormat, print_json};
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
            println!("SealTask CLI contract version 2");
            println!("API: {}", runtime.api_url());
            println!("Profile: {}", active_profile()?);
            println!("Config: {}", config_dir()?.display());
            Ok(())
        }
        OutputFormat::Json | OutputFormat::JsonPretty => {
            print_json(&payload, format, "serializing CLI metadata should succeed")
        }
    }
}
