use std::process::{Command, Stdio};

use serde_json::Value;
use tempfile::TempDir;

fn invoke(args: &[&str]) -> std::process::Output {
    let home = TempDir::new().expect("fixture");
    Command::new(env!("CARGO_BIN_EXE_sealtask"))
        .env_clear()
        .env("HOME", home.path())
        .env("SEALTASK_CONFIG_DIR", home.path().join("config"))
        .env("SEALTASK_API_URL", "http://127.0.0.1:1")
        .env("TERM", "dumb")
        .current_dir(home.path())
        .stdin(Stdio::null())
        .args(args)
        .output()
        .expect("CLI")
}

#[test]
fn test_should_preserve_machine_error_format_before_option_shaped_operands() {
    for format in ["json", "json-pretty", "jsonl"] {
        for operand in ["--format=table", "--format=json", "--json"] {
            for joined in [false, true] {
                let option = format!("--format={format}");
                let mut args = if joined {
                    vec![option.as_str()]
                } else {
                    vec!["--format", format]
                };
                args.extend(["tasks", "get", "--", operand, "EXTRA"]);
                let output = invoke(&args);
                assert_eq!(output.status.code(), Some(2));
                assert!(output.stdout.is_empty());
                assert!(!output.stderr.contains(&0x1b));
                let error: Value = serde_json::from_slice(&output.stderr).expect("JSON error");
                assert_eq!(error["error"]["code"], "validation");
                assert!(
                    error["error"]["message"]
                        .as_str()
                        .expect("message")
                        .contains("EXTRA")
                );
                let stderr = String::from_utf8(output.stderr).expect("UTF-8");
                assert!(stderr.ends_with('\n'));
                assert_eq!(stderr.lines().count() > 1, format == "json-pretty");
            }
        }
    }
}

#[test]
fn test_should_keep_human_errors_when_operands_resemble_machine_flags() {
    for operand in ["--json", "--format=json", "--format=jsonl"] {
        for explicit_table in [false, true] {
            let mut args = if explicit_table {
                vec!["--format", "table"]
            } else {
                vec![]
            };
            args.extend(["tasks", "get", "--", operand, "EXTRA"]);
            let output = invoke(&args);
            assert_eq!(output.status.code(), Some(2));
            assert!(output.stdout.is_empty());
            let stderr = String::from_utf8(output.stderr).expect("UTF-8");
            assert!(stderr.starts_with("error: "), "{stderr}");
            assert!(stderr.contains("\n\nUsage:"));
            assert!(stderr.contains("EXTRA"));
        }
    }
}

#[test]
fn test_should_keep_json_alias_for_conflicting_or_invalid_real_options() {
    for args in [
        vec!["--format", "invalid", "--json"],
        vec!["--format=json-pretty", "--json"],
        vec!["--json", "tasks", "get", "--", "--format=table", "EXTRA"],
    ] {
        let output = invoke(&args);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        let error: Value = serde_json::from_slice(&output.stderr).expect("JSON error");
        assert_eq!(error["error"]["code"], "validation");
        assert_eq!(
            String::from_utf8(output.stderr)
                .expect("UTF-8")
                .lines()
                .count(),
            1
        );
    }
}
