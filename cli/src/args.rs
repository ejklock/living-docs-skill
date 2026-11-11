//! Clap argument and subcommand definitions for the `living-docs` CLI.
//!
//! `--help` comes from the explicit `about`/`after_help` attributes in
//! `args/help.rs`, one short line plus a real `Examples:` block per verb,
//! and from the help on each argument here. The root `--help` lists the four
//! verbs (`write`, `read`, `manage`, `check`) in clap's own command listing.
//! Off a TTY the help and version print as JSON from `args/help_json.rs`.

use crate::output::{outln, ColorChoice};
use clap::error::ErrorKind;
use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};

#[path = "args/help.rs"]
mod help;
#[path = "args/help_json.rs"]
mod help_json;
#[path = "args/manage.rs"]
mod manage;
#[path = "split_reference.rs"]
pub(crate) mod split_reference;
#[path = "args/sub.rs"]
mod sub;
#[path = "args/write.rs"]
mod write;
pub(crate) use manage::ManageCmd;
use std::path::PathBuf;
pub(crate) use sub::{
    CheckArgs, GuideArgs, InstallCmd, NewRecordArgs, ReadArgs, ReadCmd, UninstallCmd,
};
pub(crate) use write::WriteCmd;

/// Parses argv into a [`Cli`] like [`Cli::parse`], except that help and
/// version requests print JSON instead of clap's text when the output mode
/// resolves to JSON; every other clap error exits as clap would.
pub(crate) fn parse() -> Cli {
    let command = Cli::command();
    let argv: Vec<std::ffi::OsString> = std::env::args_os().collect();
    match command.clone().try_get_matches_from(&argv) {
        Ok(matches) => Cli::from_arg_matches(&matches).unwrap_or_else(|err| err.exit()),
        Err(err) => exit_with(&err, &command, &argv),
    }
}

fn exit_with(err: &clap::Error, command: &clap::Command, argv: &[std::ffi::OsString]) -> ! {
    if help_json::wants_json(argv) {
        if let Some(document) = json_document(err.kind(), command, argv) {
            outln!("{document}");
            std::process::exit(err.exit_code());
        }
    }
    let hint = (err.kind() == ErrorKind::UnknownArgument)
        .then(|| split_reference::hint(argv))
        .flatten();
    if let Some(hint) = hint {
        let _ = err.print();
        eprintln!("{hint}");
        std::process::exit(err.exit_code());
    }
    err.exit()
}

fn json_document(
    kind: ErrorKind,
    command: &clap::Command,
    argv: &[std::ffi::OsString],
) -> Option<String> {
    match kind {
        ErrorKind::DisplayVersion => Some(help_json::render_version(command)),
        ErrorKind::DisplayHelp | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => {
            let path = help_json::command_path(command, argv);
            let steps: Vec<&str> = path.iter().map(String::as_str).collect();
            Some(help_json::render(command, &steps))
        }
        _ => None,
    }
}

#[derive(Parser)]
#[command(
    name = "living-docs",
    version,
    about = "Deterministic layer of Living Docs authoring. Write ONLY the body below the closing ---. Frontmatter and indexes are CLI-owned: `living-docs write set` / `write supersede`.",
    after_help = help::exit_codes_text()
)]
pub(crate) struct Cli {
    /// Root of the docs bundle, default docs under the current directory.
    #[arg(long, global = true, default_value = "docs")]
    pub(crate) docs_dir: PathBuf,

    /// Emit minified single-line JSON instead of plain text, for a data
    /// verb's success output and for --help and --version. Overrides TTY
    /// autodetection; mutually exclusive with `--plain`.
    #[arg(long, global = true)]
    pub(crate) json: bool,
    /// Force human-readable plain text, overriding TTY autodetection.
    /// Mutually exclusive with `--json`.
    #[arg(long, global = true, conflicts_with = "json")]
    pub(crate) plain: bool,
    /// Whether text output carries ANSI color: `auto` (default) colors a
    /// TTY and honors `NO_COLOR`, `always`/`never` override both.
    #[arg(long, global = true, value_enum, default_value = "auto")]
    pub(crate) color: ColorChoice,
    /// Silences informational stderr lines (warnings, progress notes).
    /// Errors always print regardless.
    #[arg(long, global = true)]
    pub(crate) quiet: bool,

    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Subcommand)]
pub(crate) enum Command {
    #[command(
        about = help::WRITE_ABOUT,
        long_about = None,
        subcommand_required = true,
        arg_required_else_help = true,
        after_help = help::with_exit_codes(help::WRITE_EXAMPLES)
    )]
    Write {
        #[command(subcommand)]
        operation: WriteCmd,
    },
    #[command(
        about = help::READ_ABOUT,
        long_about = None,
        subcommand_required = true,
        arg_required_else_help = true,
        after_help = help::with_exit_codes(help::READ_EXAMPLES)
    )]
    Read {
        #[command(subcommand)]
        operation: ReadCmd,
    },
    #[command(
        about = help::MANAGE_ABOUT,
        long_about = None,
        subcommand_required = true,
        arg_required_else_help = true,
        after_help = help::with_exit_codes(help::MANAGE_EXAMPLES)
    )]
    Manage {
        #[command(subcommand)]
        operation: ManageCmd,
    },
    #[command(about = help::CHECK_ABOUT, long_about = help::check_long_about(), after_help = help::with_exit_codes(help::CHECK_EXAMPLES))]
    Check(CheckArgs),
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    use living_docs_core::commands;

    #[test]
    fn root_help_about_carries_the_same_body_only_instruction_new_prints() {
        let about = Cli::command()
            .get_about()
            .expect("the root command carries an about string")
            .to_string();
        assert!(
            about.contains(commands::new::BODY_ONLY_INSTRUCTION),
            "got: {about}"
        );
    }
}
