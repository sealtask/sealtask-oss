use crate::output::{CliResult, print_pretty_json};
use serde_json::json;
use worklist_client_auth::UnlockMode;
use worklist_client_crypto::CryptoCapability;
use worklist_client_runtime::RuntimeClient;

pub(crate) fn run_info(runtime: &RuntimeClient) -> CliResult<()> {
    let payload = json!({
        "apiBaseUrl": runtime.api_url(),
        "commandName": "worklist",
        "automationProfile": "agent_task_management",
        "jsonContractVersion": 1,
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
    print_pretty_json(&payload, "serializing CLI metadata should succeed")
}
