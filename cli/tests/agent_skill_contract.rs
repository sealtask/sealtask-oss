use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn skill_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../skills/sealtask")
}

fn read_skill_file(relative_path: &str) -> String {
    let path = skill_directory().join(relative_path);
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

fn cli(home: &TempDir) -> Command {
    let mut command = Command::cargo_bin("sealtask").expect("sealtask binary");
    command
        .env("HOME", home.path())
        .env("SEALTASK_CONFIG_DIR", home.path().join("config"))
        .env_remove("SEALTASK_PAGER")
        .env_remove("PAGER")
        .env_remove("NO_COLOR")
        .env_remove("CLICOLOR")
        .env_remove("CLICOLOR_FORCE");
    command
}

fn command_schema(home: &TempDir, path: &[&str]) -> Value {
    let output = cli(home)
        .args(["--json", "--non-interactive", "schema"])
        .args(path)
        .output()
        .expect("run command schema");
    assert!(
        output.status.success(),
        "schema failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("JSON command schema")
}

#[test]
fn first_party_agent_skill_has_valid_minimal_metadata_and_live_references() {
    let skill_path = skill_directory().join("SKILL.md");
    let skill = read_skill_file("SKILL.md");
    let mut sections = skill.splitn(3, "---");
    assert_eq!(sections.next(), Some(""), "frontmatter must be first");
    let frontmatter = sections.next().expect("frontmatter");
    let body = sections.next().expect("skill body");

    let fields = frontmatter
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.split_once(':')
                .unwrap_or_else(|| panic!("invalid frontmatter line: {line}"))
        })
        .collect::<Vec<_>>();
    assert_eq!(
        fields.iter().map(|(key, _)| *key).collect::<Vec<_>>(),
        ["name", "description"]
    );
    let name = fields[0].1.trim();
    let description = fields[1].1.trim();
    assert_eq!(name, "sealtask");
    assert_eq!(
        skill_path
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str()),
        Some(name)
    );
    assert!(name.chars().all(|character| character.is_ascii_lowercase()
        || character.is_ascii_digit()
        || character == '-'));
    assert!(!description.is_empty());
    assert!(description.len() <= 1024);
    assert!(body.contains("# SealTask"));
    assert!(!skill.contains("[TODO"));

    for reference in [
        "references/automation-contract.md",
        "references/recovery.md",
    ] {
        assert!(
            skill.contains(&format!("]({reference})")),
            "SKILL.md must link {reference}"
        );
        assert!(
            skill_directory().join(reference).is_file(),
            "missing {reference}"
        );
    }

    let openai = read_skill_file("agents/openai.yaml");
    for expected in [
        "display_name: \"SealTask\"",
        "short_description:",
        "default_prompt:",
        "$sealtask",
    ] {
        assert!(openai.contains(expected), "missing {expected}");
    }
}

#[test]
fn agent_skill_machine_workflow_matches_the_live_cli_contract() {
    let home = TempDir::new().expect("temporary home");
    let skill = read_skill_file("SKILL.md");
    let automation = read_skill_file("references/automation-contract.md");
    for command in [
        "sealtask --json --non-interactive info",
        "sealtask --json --non-interactive auth status",
        "sealtask --json --non-interactive projects current",
        "sealtask --json --non-interactive pick project id:<project-id> --scope local",
    ] {
        assert!(
            skill.contains(command) || automation.contains(command),
            "skill package must document {command}"
        );
    }
    assert!(skill.contains("workspace root"));
    assert!(automation.contains("git rev-parse --show-toplevel"));

    let info = cli(&home)
        .args(["--json", "--non-interactive", "info"])
        .output()
        .expect("run info");
    assert!(
        info.status.success(),
        "info failed: {}",
        String::from_utf8_lossy(&info.stderr)
    );
    let info: Value = serde_json::from_slice(&info.stdout).expect("JSON info");
    assert_eq!(info["jsonContractVersion"], 2);
    assert_eq!(
        info["picker"]["explicitProjectActivation"]["nonInteractiveSupported"],
        true
    );

    let pick_project = command_schema(&home, &["pick", "project"]);
    let arguments = pick_project["arguments"]
        .as_array()
        .expect("pick-project arguments");
    assert!(arguments.iter().any(|argument| argument["id"] == "project"));
    let scope = arguments
        .iter()
        .find(|argument| argument["id"] == "scope")
        .expect("scope argument");
    assert_eq!(
        scope["possibleValues"],
        serde_json::json!(["local", "global"])
    );

    assert_eq!(command_schema(&home, &["auth", "status"])["name"], "status");
    assert_eq!(
        command_schema(&home, &["projects", "current"])["name"],
        "current"
    );
}
