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
    let selected = select_command(&mut root, path)?;
    match format {
        OutputFormat::Table => {
            print!("{}", selected.render_long_help());
            Ok(())
        }
        OutputFormat::Json | OutputFormat::JsonPretty => {
            let schema = command_schema(selected);
            print_json(&schema, format, "serializing command schema should succeed")
        }
    }
}

fn select_command<'a>(
    command: &'a mut Command,
    path: &[String],
) -> Result<&'a mut Command, PublicError> {
    let mut selected = command;
    for segment in path {
        let available = selected
            .get_subcommands()
            .filter(|command| !command.is_hide_set())
            .map(|command| command.get_name().to_string())
            .collect::<Vec<_>>();
        let is_public = selected
            .get_subcommands()
            .any(|command| command.get_name() == segment && !command.is_hide_set());
        if !is_public {
            return Err(PublicError::validation(format!(
                "unknown command path segment '{segment}'; available: {}",
                available.join(", ")
            )));
        }
        selected = selected.find_subcommand_mut(segment).ok_or_else(|| {
            PublicError::validation(format!(
                "unknown command path segment '{segment}'; available: {}",
                available.join(", ")
            ))
        })?;
    }
    Ok(selected)
}

fn command_schema(command: &mut Command) -> CommandSchemaV1 {
    let usage = command.render_usage().to_string();
    let arguments = command
        .get_arguments()
        .filter(|argument| !argument.is_hide_set())
        .map(|argument| ArgumentSchemaV1 {
            id: argument.get_id().to_string(),
            long: argument.get_long().map(str::to_owned),
            short: argument.get_short(),
            help: argument.get_help().map(ToString::to_string),
            required: argument.is_required_set(),
            global: argument.is_global_set(),
            value_names: argument
                .get_value_names()
                .map(|names| names.iter().map(ToString::to_string).collect())
                .unwrap_or_default(),
            possible_values: argument
                .get_value_parser()
                .possible_values()
                .map(|values| values.map(|value| value.get_name().to_string()).collect())
                .unwrap_or_default(),
        })
        .collect();
    let subcommands = command
        .get_subcommands_mut()
        .filter(|command| !command.is_hide_set())
        .map(command_schema)
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
        let schema = command_schema(&mut root);

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
    }
}
