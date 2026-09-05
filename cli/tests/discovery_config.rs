use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use std::time::Duration;
use tempfile::TempDir;

fn cli(home: &TempDir) -> Command {
    let mut command = Command::cargo_bin("sealtask").expect("CLI binary");
    command
        .env_clear()
        .env("HOME", home.path())
        .env("SEALTASK_CONFIG_DIR", home.path().join("config"))
        .env("SEALTASK_API_URL", "http://127.0.0.1:1")
        .current_dir(home.path())
        .timeout(Duration::from_secs(5));
    command
}

#[test]
fn schema_ignores_invalid_runtime_environment_without_creating_state() {
    let home = TempDir::new().expect("home");
    for format in ["table", "json", "json-pretty", "jsonl"] {
        for path in [vec![], vec!["tasks", "create"], vec!["lists", "get"]] {
            let baseline = cli(&home)
                .args(["--no-pager", "--format", format, "schema"])
                .args(&path)
                .output()
                .expect("baseline schema");
            assert!(baseline.status.success());
            for (name, value) in [
                ("SEALTASK_API_URL", "invalid"),
                ("SEALTASK_PROFILE", "../../invalid"),
                ("SEALTASK_CONNECT_TIMEOUT", "not a duration"),
                ("SEALTASK_READ_TIMEOUT", "not a duration"),
                ("SEALTASK_REQUEST_TIMEOUT", "not a duration"),
            ] {
                let output = cli(&home)
                    .env(name, value)
                    .args(["--no-pager", "--format", format, "schema"])
                    .args(&path)
                    .output()
                    .expect("schema with invalid environment");
                assert!(
                    output.status.success(),
                    "{name}: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                assert!(output.stderr.is_empty());
                assert_eq!(output.stdout, baseline.stdout, "{name}: {format} {path:?}");
            }
        }
    }
    assert_eq!(fs::read_dir(home.path()).expect("home entries").count(), 0);
}

#[test]
fn schema_ignores_corrupt_profile_state_and_never_contacts_the_api() {
    let home = TempDir::new().expect("home");
    let config = home.path().join("config");
    fs::create_dir(&config).expect("config fixture");
    let settings = config.join("operator-settings.json");
    let credentials = config.join("credentials.json");
    fs::write(&settings, "invalid settings").expect("settings fixture");
    fs::write(&credentials, "invalid credentials").expect("credentials fixture");
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("listener");
    listener.set_nonblocking(true).expect("nonblocking");
    let url = format!("http://{}", listener.local_addr().expect("address"));
    for flags in [
        vec![],
        vec!["--profile", "missing"],
        vec!["--profile", "../../invalid"],
        vec!["--offline"],
    ] {
        let output = cli(&home)
            .env("SEALTASK_API_URL", &url)
            .args(flags)
            .args(["--json", "schema", "tasks", "create"])
            .output()
            .expect("schema");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        let schema: Value = serde_json::from_slice(&output.stdout).expect("JSON schema");
        assert_eq!(schema["schemaVersion"], 1);
        assert_eq!(schema["name"], "create");
        assert_eq!(
            listener.accept().expect_err("no network").kind(),
            std::io::ErrorKind::WouldBlock
        );
    }
    assert_eq!(
        fs::read_to_string(settings).expect("settings"),
        "invalid settings"
    );
    assert_eq!(
        fs::read_to_string(credentials).expect("credentials"),
        "invalid credentials"
    );
    assert_eq!(fs::read_dir(&config).expect("config entries").count(), 2);
    assert_eq!(fs::read_dir(home.path()).expect("home entries").count(), 1);
}

#[test]
fn schema_keeps_path_and_presentation_validation_before_runtime_configuration() {
    let home = TempDir::new().expect("home");
    for (args, message) in [
        (
            vec!["--json", "schema", "tasks", "not-a-command"],
            "unknown command path segment",
        ),
        (
            vec!["--json", "--debug", "schema"],
            "cannot be combined with JSON",
        ),
        (
            vec!["--json", "--pager", "always", "schema"],
            "cannot be used with JSON",
        ),
    ] {
        let output = cli(&home)
            .env("SEALTASK_API_URL", "invalid")
            .args(args)
            .output()
            .expect("schema validation");
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let error: Value = serde_json::from_slice(&output.stderr).expect("JSON error");
        assert_eq!(error["error"]["code"], "validation");
        assert!(
            error["error"]["message"]
                .as_str()
                .expect("message")
                .contains(message)
        );
    }
    assert_eq!(fs::read_dir(home.path()).expect("home entries").count(), 0);
}

#[test]
fn schema_ignores_explicit_runtime_flags_even_without_a_usable_home_or_config_path() {
    let home = TempDir::new().expect("home");
    let blocked_parent = home.path().join("not-a-directory");
    fs::write(&blocked_parent, "fixture").expect("blocked config parent");
    let config_path = blocked_parent.join("config");
    for flags in [
        vec!["--api-url", "invalid"],
        vec!["--connect-timeout", "bogus"],
        vec!["--read-timeout", "bogus"],
        vec!["--request-timeout", "bogus"],
        vec!["--profile", "../../invalid"],
        vec!["--config-dir", config_path.to_str().expect("path")],
    ] {
        let output = cli(&home)
            .env_remove("HOME")
            .env("SEALTASK_CONFIG_DIR", &config_path)
            .args(flags)
            .args(["--json", "schema", "tasks", "create"])
            .output()
            .expect("schema");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        let schema: Value = serde_json::from_slice(&output.stdout).expect("schema JSON");
        assert_eq!(schema["name"], "create");
    }
    assert_eq!(
        fs::read_to_string(blocked_parent).expect("fixture preserved"),
        "fixture"
    );
    assert_eq!(fs::read_dir(home.path()).expect("home entries").count(), 1);
}

#[test]
fn schema_emits_no_runtime_telemetry_for_human_verbosity_flags() {
    let home = TempDir::new().expect("home");
    for flag in ["-v", "-vv", "--debug"] {
        let output = cli(&home)
            .env("SEALTASK_API_URL", "invalid")
            .args([flag, "schema", "tasks", "create"])
            .output()
            .expect("schema");
        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("Usage: sealtask tasks create"));
        assert!(output.stderr.is_empty());
    }
    assert_eq!(fs::read_dir(home.path()).expect("home entries").count(), 0);
}
