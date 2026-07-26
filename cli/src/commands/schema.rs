use crate::args::Cli;
use crate::output::{CliResult, OutputFormat, print_json};
use clap::{Command, CommandFactory};
use sealtask_client_core::PublicError;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CommandSchemaV1 {
    schema_version: u32,
    name: String,
    about: Option<String>,
    usage: String,
    arguments: Vec<ArgumentSchemaV1>,
    subcommands: Vec<CommandSchemaV1>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ArgumentSchemaV1 {
    id: String,
    long: Option<String>,
    short: Option<char>,
    help: Option<String>,
    required: bool,
    global: bool,
    value_names: Vec<String>,
    possible_values: Vec<String>,
}

pub(crate) fn run(format: OutputFormat, path: &[String]) -> CliResult<()> {
    let mut root = Cli::command();
    root.build();
    let canonical_path = canonical_command_path(&root, path)?;
    let selected = select_command(&mut root, path)?;
    let display_path = std::iter::once("sealtask")
        .chain(canonical_path.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join(" ");
    match format {
        OutputFormat::Table => {
            let mut display_command = selected.clone().bin_name(&display_path);
            print!("{}", display_command.render_long_help());
            Ok(())
        }
        OutputFormat::Json | OutputFormat::JsonPretty => {
            let schema = command_schema(selected, &display_path);
            print_json(&schema, format, "serializing command schema should succeed")
        }
    }
}

fn canonical_command_path(command: &Command, path: &[String]) -> Result<Vec<String>, PublicError> {
    let mut selected = command;
    let mut canonical_path = Vec::with_capacity(path.len());
    for segment in path {
        let available = visible_subcommand_names(selected);
        let Some(next) = selected.get_subcommands().find(|command| {
            !command.is_hide_set()
                && (command.get_name() == segment
                    || command.get_all_aliases().any(|alias| alias == segment))
        }) else {
            return Err(unknown_path_segment(segment, &available));
        };
        canonical_path.push(next.get_name().to_string());
        selected = next;
    }
    Ok(canonical_path)
}

fn select_command<'a>(
    command: &'a mut Command,
    path: &[String],
) -> Result<&'a mut Command, PublicError> {
    let mut selected = command;
    for segment in path {
        let available = visible_subcommand_names(selected);
        let canonical = selected
            .get_subcommands()
            .find(|command| {
                !command.is_hide_set()
                    && (command.get_name() == segment
                        || command.get_all_aliases().any(|alias| alias == segment))
            })
            .map(|command| command.get_name().to_string());
        let Some(canonical) = canonical else {
            return Err(unknown_path_segment(segment, &available));
        };
        selected = selected
            .find_subcommand_mut(&canonical)
            .ok_or_else(|| unknown_path_segment(segment, &available))?;
    }
    Ok(selected)
}

fn visible_subcommand_names(command: &Command) -> Vec<String> {
    command
        .get_subcommands()
        .filter(|command| !command.is_hide_set())
        .map(|command| command.get_name().to_string())
        .collect()
}

fn unknown_path_segment(segment: &str, available: &[String]) -> PublicError {
    PublicError::validation(format!(
        "unknown command path segment '{segment}'; available: {}",
        available.join(", ")
    ))
}

fn command_schema(command: &mut Command, display_path: &str) -> CommandSchemaV1 {
    let mut display_command = command.clone().bin_name(display_path);
    let usage = display_command.render_usage().to_string();
    let arguments = command
        .get_arguments()
        .filter(|argument| !argument.is_hide_set())
        .map(|argument| {
            let takes_values = argument.get_action().takes_values();
            ArgumentSchemaV1 {
                id: argument.get_id().to_string(),
                long: argument.get_long().map(str::to_owned),
                short: argument.get_short(),
                help: argument.get_help().map(ToString::to_string),
                required: argument.is_required_set(),
                global: argument.is_global_set(),
                value_names: if takes_values {
                    argument
                        .get_value_names()
                        .map(|names| names.iter().map(ToString::to_string).collect())
                        .unwrap_or_default()
                } else {
                    Vec::new()
                },
                possible_values: if takes_values {
                    argument
                        .get_value_parser()
                        .possible_values()
                        .map(|values| values.map(|value| value.get_name().to_string()).collect())
                        .unwrap_or_default()
                } else {
                    Vec::new()
                },
            }
        })
        .collect();
    let subcommands = command
        .get_subcommands_mut()
        .filter(|command| !command.is_hide_set())
        .map(|command| {
            let child_path = format!("{display_path} {}", command.get_name());
            command_schema(command, &child_path)
        })
        .collect();
    CommandSchemaV1 {
        schema_version: 1,
        name: command.get_name().to_string(),
        about: command.get_about().map(ToString::to_string),
        usage,
        arguments,
        subcommands,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn machine_schema_excludes_hidden_commands_and_arguments() {
        let mut root = Cli::command();
        root.build();
        let schema = command_schema(&mut root, "sealtask");

        assert!(
            schema
                .subcommands
                .iter()
                .all(|command| command.name != "inspect")
        );
        assert!(
            schema
                .arguments
                .iter()
                .all(|argument| argument.long.as_deref() != Some("serve-unlock-daemon"))
        );
        assert!(select_command(&mut root, &["inspect".to_string()]).is_err());

        let selected = select_command(&mut root, &["lists".to_string()]).expect("visible alias");
        assert_eq!(selected.get_name(), "projects");
    }

    #[test]
    fn nested_schema_uses_runnable_paths_and_presence_flags_have_no_values() {
        let mut root = Cli::command();
        root.build();
        let selected = select_command(&mut root, &["tasks".to_string(), "update".to_string()])
            .expect("tasks update");
        let schema = command_schema(selected, "sealtask tasks update");
        assert!(schema.usage.contains("sealtask tasks update"));

        let json = schema
            .arguments
            .iter()
            .find(|argument| argument.long.as_deref() == Some("json"))
            .expect("global json flag");
        assert!(json.value_names.is_empty());
        assert!(json.possible_values.is_empty());
    }
}
