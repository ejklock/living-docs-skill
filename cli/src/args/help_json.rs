//! The JSON form of `--help` and `--version`, derived from the same clap
//! command tree the text help renders so the two cannot drift. Only the
//! examples and exit codes come from the hand tables in `help.rs`.

use super::help;
use crate::output::{self, OutputMode};
use clap::{Arg, Command};
use serde::Serialize;
use std::ffi::OsString;
use std::io::IsTerminal;

#[derive(Serialize)]
struct ArgDoc {
    name: String,
    required: bool,
    help: String,
    possible_values: Vec<String>,
}

#[derive(Serialize)]
struct OptionDoc {
    long: String,
    value: Option<String>,
    required: bool,
    help: String,
    default: Option<String>,
    possible_values: Vec<String>,
    global: bool,
}

#[derive(Serialize)]
struct ExitCodeDoc {
    code: u8,
    meaning: &'static str,
}

#[derive(Serialize)]
struct CommandDoc {
    name: String,
    about: String,
    usage: String,
    args: Vec<ArgDoc>,
    options: Vec<OptionDoc>,
    subcommands: Vec<CommandDoc>,
    examples: &'static [&'static str],
    exit_codes: Vec<ExitCodeDoc>,
}

#[derive(Serialize)]
struct VerbDoc {
    #[serde(flatten)]
    command: CommandDoc,
    global_options: Vec<OptionDoc>,
}

#[derive(Serialize)]
struct GroupDoc {
    name: &'static str,
    commands: Vec<&'static str>,
}

#[derive(Serialize)]
struct RootDoc {
    name: String,
    version: String,
    about: String,
    usage: String,
    groups: Vec<GroupDoc>,
    options: Vec<OptionDoc>,
    commands: Vec<CommandDoc>,
    exit_codes: Vec<ExitCodeDoc>,
}

#[derive(Serialize)]
struct VersionDoc {
    name: String,
    version: String,
}

fn exit_codes() -> Vec<ExitCodeDoc> {
    help::EXIT_CODES
        .iter()
        .map(|&(code, meaning)| ExitCodeDoc { code, meaning })
        .collect()
}

fn usage_of(command: &Command) -> String {
    let rendered = command.clone().render_usage().to_string();
    rendered
        .strip_prefix("Usage: ")
        .unwrap_or(&rendered)
        .to_owned()
}

fn about_of(command: &Command) -> String {
    command
        .get_long_about()
        .or_else(|| command.get_about())
        .map(ToString::to_string)
        .unwrap_or_default()
}

fn possible_values_of(arg: &Arg) -> Vec<String> {
    arg.get_possible_values()
        .iter()
        .filter(|value| !value.is_hide_set())
        .map(|value| value.get_name().to_owned())
        .collect()
}

fn help_of(arg: &Arg) -> String {
    arg.get_long_help()
        .or_else(|| arg.get_help())
        .map(ToString::to_string)
        .unwrap_or_default()
}

fn arg_doc(arg: &Arg) -> ArgDoc {
    let name = arg.get_value_names().and_then(|names| names.first());
    ArgDoc {
        name: name.map_or_else(|| arg.get_id().as_str().to_uppercase(), |n| n.to_string()),
        required: arg.is_required_set(),
        help: help_of(arg),
        possible_values: possible_values_of(arg),
    }
}

fn option_doc(arg: &Arg) -> Option<OptionDoc> {
    let long = arg.get_long()?;
    let takes_value = arg.get_action().takes_values();
    let value = arg.get_value_names().and_then(|names| names.first());
    let default = arg.get_default_values().first();
    Some(OptionDoc {
        long: long.to_owned(),
        value: value.filter(|_| takes_value).map(ToString::to_string),
        required: arg.is_required_set(),
        help: help_of(arg),
        default: default
            .filter(|_| takes_value)
            .map(|v| v.to_string_lossy().into_owned()),
        possible_values: possible_values_of(arg),
        global: arg.is_global_set(),
    })
}

fn is_builtin(arg: &Arg) -> bool {
    matches!(arg.get_id().as_str(), "help" | "version")
}

fn options_of(command: &Command, global: bool) -> Vec<OptionDoc> {
    command
        .get_arguments()
        .filter(|arg| !arg.is_positional() && !arg.is_hide_set() && !is_builtin(arg))
        .filter(|arg| arg.is_global_set() == global)
        .filter_map(option_doc)
        .collect()
}

fn command_doc(command: &Command, path: &[&str]) -> CommandDoc {
    let name = command.get_name();
    let own: Vec<&str> = path.iter().copied().chain([name]).collect();
    let subcommands = command
        .get_subcommands()
        .filter(|sub| !sub.is_hide_set() && sub.get_name() != "help")
        .map(|sub| command_doc(sub, &own))
        .collect();
    CommandDoc {
        name: name.to_owned(),
        about: about_of(command),
        usage: usage_of(command),
        args: command
            .get_positionals()
            .filter(|arg| !arg.is_hide_set())
            .map(arg_doc)
            .collect(),
        options: options_of(command, false),
        subcommands,
        examples: help::examples_for(&own[1..]),
        exit_codes: exit_codes(),
    }
}

fn root_doc(root: &Command) -> RootDoc {
    let commands = root
        .get_subcommands()
        .filter(|sub| !sub.is_hide_set() && sub.get_name() != "help")
        .map(|sub| command_doc(sub, &[root.get_name()]))
        .collect();
    let groups = help::groups()
        .into_iter()
        .map(|(name, commands)| GroupDoc { name, commands })
        .collect();
    RootDoc {
        name: root.get_name().to_owned(),
        version: root.get_version().unwrap_or_default().to_owned(),
        about: about_of(root),
        usage: usage_of(root),
        groups,
        options: options_of(root, true),
        commands,
        exit_codes: exit_codes(),
    }
}

/// The JSON document for the help of the command at `path` (empty for the
/// root): the root document carries every verb in full, a verb's document
/// carries that verb plus the global options.
pub(crate) fn render(root: &Command, path: &[&str]) -> String {
    let mut built = root.clone();
    built.build();
    let mut node = &built;
    let mut names = vec![built.get_name()];
    for step in path {
        let Some(next) = node.find_subcommand(step) else {
            break;
        };
        names.push(next.get_name());
        node = next;
    }
    if names.len() == 1 {
        return output::to_json(&root_doc(&built));
    }
    output::to_json(&VerbDoc {
        command: command_doc(node, &names[..names.len() - 1]),
        global_options: options_of(&built, true),
    })
}

/// The JSON document `--version` prints.
pub(crate) fn render_version(root: &Command) -> String {
    output::to_json(&VersionDoc {
        name: root.get_name().to_owned(),
        version: root.get_version().unwrap_or_default().to_owned(),
    })
}

/// Whether the help or version of this invocation prints JSON. The help path
/// runs before clap has parsed the globals, so the same mode decision every
/// data verb makes reads the flags straight from argv.
pub(crate) fn wants_json(argv: &[OsString]) -> bool {
    let has = |flag: &str| argv.iter().any(|arg| arg == flag);
    let color_always = argv
        .windows(2)
        .any(|pair| pair[0] == "--color" && pair[1] == "always")
        || has("--color=always");
    OutputMode::resolve(
        has("--json"),
        has("--plain"),
        color_always,
        std::io::stdout().is_terminal(),
    )
    .is_json()
}

/// The subcommand names an invocation addressed, in order, found by walking
/// argv through the command tree (`help set` and `set --help` both give
/// `["set"]`). Values of the global options that take one are skipped.
pub(crate) fn command_path(root: &Command, argv: &[OsString]) -> Vec<String> {
    let mut node = root;
    let mut path = Vec::new();
    let mut skip_value = false;
    for token in argv.iter().skip(1).filter_map(|arg| arg.to_str()) {
        if std::mem::take(&mut skip_value) || token.starts_with('-') {
            skip_value = matches!(token, "--docs-dir" | "--color");
            continue;
        }
        if let Some(next) = node.find_subcommand(token) {
            path.push(next.get_name().to_owned());
            node = next;
        }
    }
    path
}
