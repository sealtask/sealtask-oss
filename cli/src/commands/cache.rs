use crate::args::CacheCommand;
use crate::output::{CliResult, OutputFormat, print_json, print_simple_result, terminal_line};
use sealtask_client_runtime::{
    ReadCacheMode, ReadCacheStatus, ReadCacheVerification, RuntimeClient,
};
use serde::Serialize;

const CACHE_OUTPUT_SCHEMA_VERSION: u8 = 1;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CacheStatusResult {
    schema_version: u8,
    #[serde(flatten)]
    status: ReadCacheStatus,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CacheVerificationResult {
    output_schema_version: u8,
    verified: bool,
    #[serde(flatten)]
    verification: ReadCacheVerification,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CacheClearResult {
    schema_version: u8,
    cleared: bool,
}

pub(crate) async fn run_cache(
    runtime: &RuntimeClient,
    format: OutputFormat,
    command: CacheCommand,
) -> CliResult<()> {
    match command {
        CacheCommand::Status => print_status(format, runtime.read_cache_status()?),
        CacheCommand::Verify { password_stdin } => {
            let verification = runtime.verify_read_cache(password_stdin).await?;
            print_verification(format, verification)
        }
        CacheCommand::Clear => {
            let cleared = runtime.clear_read_cache()?;
            print_simple_result(
                format,
                &CacheClearResult {
                    schema_version: CACHE_OUTPUT_SCHEMA_VERSION,
                    cleared,
                },
                "serializing cache clear result should succeed",
                if cleared {
                    "Encrypted offline cache cleared."
                } else {
                    "No encrypted offline cache was present."
                },
            )
        }
    }
}

fn print_status(format: OutputFormat, status: ReadCacheStatus) -> CliResult<()> {
    let result = CacheStatusResult {
        schema_version: CACHE_OUTPUT_SCHEMA_VERSION,
        status,
    };
    match format {
        OutputFormat::Json | OutputFormat::JsonPretty | OutputFormat::Jsonl => {
            print_json(&result, format, "serializing cache status should succeed")
        }
        OutputFormat::Table => {
            println!("Encrypted offline cache");
            println!("Mode: {}", cache_mode(result.status.mode));
            println!(
                "Present: {}",
                if result.status.present { "yes" } else { "no" }
            );
            if let Some(ciphertext_bytes) = result.status.ciphertext_bytes {
                println!("Ciphertext size: {ciphertext_bytes} bytes");
            }
            if let Some(modified_at) = result.status.modified_at {
                println!("Modified: {}", terminal_line(&modified_at.to_rfc3339()));
            }
            Ok(())
        }
    }
}

fn print_verification(format: OutputFormat, verification: ReadCacheVerification) -> CliResult<()> {
    let result = CacheVerificationResult {
        output_schema_version: CACHE_OUTPUT_SCHEMA_VERSION,
        verified: true,
        verification,
    };
    match format {
        OutputFormat::Json | OutputFormat::JsonPretty | OutputFormat::Jsonl => print_json(
            &result,
            format,
            "serializing cache verification should succeed",
        ),
        OutputFormat::Table => {
            println!("Encrypted offline cache verified.");
            println!("Cache schema: {}", result.verification.schema_version);
            println!("Entries: {}", result.verification.entry_count);
            println!(
                "Captured: {} to {}",
                terminal_line(&result.verification.created_at.to_rfc3339()),
                terminal_line(&result.verification.updated_at.to_rfc3339())
            );
            println!(
                "Ciphertext size: {} bytes",
                result.verification.ciphertext_bytes
            );
            Ok(())
        }
    }
}

const fn cache_mode(mode: ReadCacheMode) -> &'static str {
    match mode {
        ReadCacheMode::Online => "online population",
        ReadCacheMode::Offline => "offline reads only",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn verification_json_has_distinct_output_and_cache_schema_fields() {
        let result = CacheVerificationResult {
            output_schema_version: CACHE_OUTPUT_SCHEMA_VERSION,
            verified: true,
            verification: ReadCacheVerification {
                schema_version: 7,
                entry_count: 3,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                ciphertext_bytes: 512,
            },
        };
        let encoded = serde_json::to_string(&result).expect("serialize cache verification");
        assert_eq!(encoded.matches("\"schemaVersion\"").count(), 1);
        assert_eq!(encoded.matches("\"outputSchemaVersion\"").count(), 1);
        let value: serde_json::Value = serde_json::from_str(&encoded).expect("parse JSON");
        assert_eq!(value["schemaVersion"], 7);
        assert_eq!(value["outputSchemaVersion"], 1);
    }
}
