use crate::args::{ConfigCommand, ProfileCommand};
use crate::operator_config::{
    OperatorSettingsStore, ResolvedOperatorConfig, ResolvedValue, ValueSource,
};
use crate::output::{CliResult, OutputFormat, print_json, print_simple_result, terminal_line};
use serde::Serialize;
use std::collections::BTreeSet;
use std::time::Duration;

const OPERATOR_OUTPUT_SCHEMA_VERSION: u8 = 1;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConfigValueV1<T> {
    value: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<ValueSource>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TimeoutValueV1 {
    milliseconds: u64,
    display: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<ValueSource>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ResolvedConfigV1 {
    schema_version: u8,
    resolved: bool,
    api_url: ConfigValueV1<String>,
    profile: ConfigValueV1<String>,
    config_directory: ConfigValueV1<String>,
    profile_config_directory: String,
    connect_timeout: TimeoutValueV1,
    read_timeout: TimeoutValueV1,
    request_timeout: TimeoutValueV1,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProfileListV1 {
    schema_version: u8,
    active_profile: String,
    active_source: ValueSource,
    default_profile: String,
    profiles: Vec<ProfileEntryV1>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProfileEntryV1 {
    name: String,
    active: bool,
    default: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProfileUseV1 {
    schema_version: u8,
    default_profile: String,
    changed: bool,
    effective_profile_for_this_command: String,
    effective_source_for_this_command: ValueSource,
    override_active: bool,
}

pub(crate) fn run_config(
    format: OutputFormat,
    config: &ResolvedOperatorConfig,
    command: ConfigCommand,
) -> CliResult<()> {
    match command {
        ConfigCommand::Show { resolved } => print_config(format, config, resolved),
    }
}

pub(crate) fn run_profile(
    format: OutputFormat,
    config: &ResolvedOperatorConfig,
    command: ProfileCommand,
) -> CliResult<()> {
    let store = OperatorSettingsStore::new(&config.config_dir.value)?;
    match command {
        ProfileCommand::List => list_profiles(format, config, &store),
        ProfileCommand::Use { name } => use_profile(format, config, &store, &name),
    }
}

fn print_config(
    format: OutputFormat,
    config: &ResolvedOperatorConfig,
    resolved: bool,
) -> CliResult<()> {
    let report = ResolvedConfigV1 {
        schema_version: OPERATOR_OUTPUT_SCHEMA_VERSION,
        resolved,
        api_url: string_value(&config.api_url, resolved),
        profile: string_value(&config.profile, resolved),
        config_directory: ConfigValueV1 {
            value: config.config_dir.value.display().to_string(),
            source: resolved.then_some(config.config_dir.source),
        },
        profile_config_directory: config.profile_config_dir().display().to_string(),
        connect_timeout: timeout_value(&config.connect_timeout, resolved),
        read_timeout: timeout_value(&config.read_timeout, resolved),
        request_timeout: timeout_value(&config.request_timeout, resolved),
    };

    match format {
        OutputFormat::Json | OutputFormat::JsonPretty | OutputFormat::Jsonl => print_json(
            &report,
            format,
            "serializing resolved operator configuration should succeed",
        ),
        OutputFormat::Table => {
            println!("SealTask configuration");
            println!("{}", "-".repeat(40));
            print_human_value("API URL", &report.api_url.value, report.api_url.source)?;
            print_human_value("Profile", &report.profile.value, report.profile.source)?;
            print_human_value(
                "Config directory",
                &report.config_directory.value,
                report.config_directory.source,
            )?;
            println!(
                "Profile directory: {}",
                terminal_line(&report.profile_config_directory)
            );
            print_human_timeout("Connect timeout", &report.connect_timeout)?;
            print_human_timeout("Read timeout", &report.read_timeout)?;
            print_human_timeout("Request timeout", &report.request_timeout)
        }
    }
}

fn list_profiles(
    format: OutputFormat,
    config: &ResolvedOperatorConfig,
    store: &OperatorSettingsStore,
) -> CliResult<()> {
    let default_profile = store.load()?.active_profile;
    let mut names = store.list_profiles()?.into_iter().collect::<BTreeSet<_>>();
    names.insert(config.profile.value.clone());
    names.insert(default_profile.clone());
    let profiles = names
        .into_iter()
        .map(|name| ProfileEntryV1 {
            active: name == config.profile.value,
            default: name == default_profile,
            name,
        })
        .collect::<Vec<_>>();
    let result = ProfileListV1 {
        schema_version: OPERATOR_OUTPUT_SCHEMA_VERSION,
        active_profile: config.profile.value.clone(),
        active_source: config.profile.source,
        default_profile,
        profiles,
    };

    match format {
        OutputFormat::Json | OutputFormat::JsonPretty | OutputFormat::Jsonl => {
            print_json(&result, format, "serializing profile list should succeed")
        }
        OutputFormat::Table => {
            println!(
                "Profiles (effective: {} from {}; default: {})",
                terminal_line(&result.active_profile),
                result.active_source,
                terminal_line(&result.default_profile)
            );
            for profile in &result.profiles {
                println!(
                    "{}{} {}",
                    if profile.active { "*" } else { " " },
                    if profile.default { "+" } else { " " },
                    terminal_line(&profile.name)
                );
            }
            println!("Markers: * effective, + persisted default");
            Ok(())
        }
    }
}

fn use_profile(
    format: OutputFormat,
    config: &ResolvedOperatorConfig,
    store: &OperatorSettingsStore,
    name: &str,
) -> CliResult<()> {
    let current_default = store.load()?.active_profile;
    store.set_active_profile(name)?;
    let selected = crate::operator_config::validate_profile_name(name)?;
    let override_active = matches!(
        config.profile.source,
        ValueSource::Cli | ValueSource::Environment
    );
    let result = ProfileUseV1 {
        schema_version: OPERATOR_OUTPUT_SCHEMA_VERSION,
        default_profile: selected.clone(),
        changed: current_default != selected,
        effective_profile_for_this_command: config.profile.value.clone(),
        effective_source_for_this_command: config.profile.source,
        override_active,
    };
    let message = if override_active {
        format!(
            "Default profile set to {}. The current {} override still selects {}; unset it to use the new default.",
            terminal_line(&selected),
            config.profile.source,
            terminal_line(&config.profile.value)
        )
    } else if result.changed {
        format!(
            "Default profile set to {}. It will be active on the next command.",
            terminal_line(&selected)
        )
    } else {
        format!("Default profile is already {}.", terminal_line(&selected))
    };

    print_simple_result(
        format,
        &result,
        "serializing profile selection should succeed",
        &message,
    )
}

fn string_value(value: &ResolvedValue<String>, resolved: bool) -> ConfigValueV1<String> {
    ConfigValueV1 {
        value: value.value.clone(),
        source: resolved.then_some(value.source),
    }
}

fn timeout_value(value: &ResolvedValue<Duration>, resolved: bool) -> TimeoutValueV1 {
    TimeoutValueV1 {
        milliseconds: duration_milliseconds(value.value),
        display: format_duration(value.value),
        source: resolved.then_some(value.source),
    }
}

fn duration_milliseconds(value: Duration) -> u64 {
    u64::try_from(value.as_millis()).unwrap_or(u64::MAX)
}

fn format_duration(value: Duration) -> String {
    if value.subsec_millis() == 0 && value.as_secs().is_multiple_of(3_600) {
        format!("{}h", value.as_secs() / 3_600)
    } else if value.subsec_millis() == 0 && value.as_secs().is_multiple_of(60) {
        format!("{}m", value.as_secs() / 60)
    } else if value.subsec_millis() == 0 {
        format!("{}s", value.as_secs())
    } else {
        format!("{}ms", value.as_millis())
    }
}

fn print_human_value(label: &str, value: &str, source: Option<ValueSource>) -> CliResult<()> {
    match source {
        Some(source) => println!("{label}: {} ({source})", terminal_line(value)),
        None => println!("{label}: {}", terminal_line(value)),
    }
    Ok(())
}

fn print_human_timeout(label: &str, value: &TimeoutValueV1) -> CliResult<()> {
    print_human_value(label, &value.display, value.source)
}
