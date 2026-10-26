//! Clap argument and subcommand definitions for the `living-docs` CLI.
//!
//! `--help` comes from the explicit `about`/`after_help` attributes in
//! `args/help.rs`, one short line plus a real `Examples:` block per verb,
//! and from the help on each argument here. The root `--help`'s verb listing
//! is grouped into Authoring (`new`, `set`, `supersede`, `index`, `fmt`),
//! Gate (`check`), Reading (`read`, `guide`), and Distribution (`install`,
//! `uninstall`, `completions`) by `help::root_command_template` — clap's
//! `next_help_heading` groups an individual subcommand's own args, not
//! sibling subcommands, so it cannot do this grouping itself. Off a TTY the
//! help and version print as JSON from `args/help_json.rs`.

use crate::output::ColorChoice;
use clap::error::ErrorKind;
use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
use clap_complete::Shell;

#[path = "args/help.rs"]
mod help;
#[path = "args/help_json.rs"]
mod help_json;
#[path = "split_reference.rs"]
pub(crate) mod split_reference;
#[path = "args/sub.rs"]
mod sub;
use std::path::PathBuf;
pub(crate) use sub::{GuideArgs, InstallCmd, ReadArgs, UninstallCmd};

/// Parses argv into a [`Cli`], first grafting the hand-grouped root command
/// listing (`help::root_command_template`) onto the derived [`Command`] —
/// clap's derive has no hook for a custom template, so this stands in for
/// [`Cli::parse`]. Help and version requests print JSON instead of clap's
/// text when the output mode resolves to JSON; every other clap error exits
/// as clap would.
pub(crate) fn parse() -> Cli {
    let command = Cli::command().help_template(help::root_command_template());
    let argv: Vec<std::ffi::OsString> = std::env::args_os().collect();
    match command.clone().try_get_matches_from(&argv) {
        Ok(matches) => Cli::from_arg_matches(&matches).unwrap_or_else(|err| err.exit()),
        Err(err) => exit_with(&err, &command, &argv),
    }
}

fn exit_with(err: &clap::Error, command: &clap::Command, argv: &[std::ffi::OsString]) -> ! {
    if help_json::wants_json(argv) {
        if let Some(document) = json_document(err.kind(), command, argv) {
            println!("{document}");
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
    about = "Deterministic layer of Living Docs authoring. Write ONLY the body below the closing ---. Frontmatter and indexes are CLI-owned: `living-docs set` / `supersede`.",
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
    #[command(about = help::NEW_ABOUT, long_about = None, after_help = help::with_exit_codes(help::NEW_EXAMPLES))]
    New {
        #[arg(value_name = "TYPE", help = help::doc_type_help())]
        doc_type: String,
        /// Title of the new record; it becomes the heading and the file slug.
        title: String,
        /// Seeds the frontmatter `description:` field with this sentence
        /// instead of the template's placeholder.
        #[arg(long)]
        description: Option<String>,
        /// Architecture views only: seeds the frontmatter `kind:` field from
        /// the C4/arc42 vocabulary (context, container, component, flow,
        /// sequence, state, data-model, deployment) that orders the
        /// generated architecture index.
        #[arg(long)]
        kind: Option<String>,
        #[arg(long, help = help::owner_help())]
        owner: Option<String>,
        /// Replaces the template's body slots with the text of this file, or
        /// of standard input for `-`; the CLI places it below the heading
        /// and never writes prose itself.
        #[arg(long, value_name = "PATH")]
        body: Option<String>,
    },
    /// `title` also rewrites the heading, renames the file to the new slug
    /// and repoints every in-bundle reference; it is refused on a record
    /// closed for good. `number` renames the record to a free number
    /// (`NNNN` or `next`), the repair for two records sharing a number.
    #[command(about = help::SET_ABOUT, long_about = None, after_help = help::with_exit_codes(help::SET_EXAMPLES))]
    Set {
        #[arg(help = help::reference_help())]
        reference: String,
        #[arg(help = help::set_key_help())]
        key: String,
        /// The new value; for the `section` key, the section heading.
        value: Option<String>,
        /// Reads the value, or for `section` the text under the heading, from
        /// this file or from standard input for `-`. `section` replaces the
        /// named body section or appends it when absent.
        #[arg(long, value_name = "PATH")]
        from: Option<String>,
    },
    #[command(about = help::SUPERSEDE_ABOUT, long_about = None, after_help = help::with_exit_codes(help::SUPERSEDE_EXAMPLES))]
    Supersede {
        #[arg(help = format!("the record being retired: {}", help::reference_help()))]
        old: String,
        #[arg(help = format!("the record that replaces it: {}", help::reference_help()))]
        new: String,
    },
    /// With no `doc_type`, regenerates every registered type's index.
    #[command(about = help::INDEX_ABOUT, long_about = None, after_help = help::with_exit_codes(help::INDEX_EXAMPLES))]
    Index {
        #[arg(value_name = "TYPE", help = help::doc_type_help())]
        doc_type: Option<String>,
    },
    #[command(about = help::FMT_ABOUT, long_about = None, after_help = help::with_exit_codes(help::FMT_EXAMPLES))]
    Fmt {
        /// A bundle root or a single record path; filesystem backend only.
        paths: Vec<PathBuf>,
        /// Report what is pending without writing anything; exits 1 when a
        /// record is not canonical.
        #[arg(long)]
        check: bool,
    },
    #[command(about = help::CHECK_ABOUT, long_about = help::check_long_about(), after_help = help::with_exit_codes(help::CHECK_EXAMPLES))]
    Check {
        /// The bundle root to validate (default docs) or, with
        /// `--mermaid-only`, the file(s) or directory(ies) to sweep for
        /// mermaid fences.
        paths: Vec<PathBuf>,
        /// Validate only mermaid fences over `paths`, skipping every other invariant.
        #[arg(long)]
        mermaid_only: bool,
        /// Promotes a missing `owner` on a doctype whose registry row
        /// requires it from a warning to an invariant violation.
        #[arg(long)]
        require_owner: bool,
        /// Report only the findings anchored to these records, so a bundle
        /// with legacy debt can gate a commit on what it touched. Every
        /// invariant still runs over the whole bundle.
        #[arg(long, num_args = 1.., value_name = "PATH")]
        changed_files: Vec<PathBuf>,
        /// Lists every advisory after the per-kind summary instead of the
        /// summary alone; the JSON output is unaffected.
        #[arg(long)]
        advisories: bool,
    },
    #[command(about = help::READ_ABOUT, long_about = None, after_help = help::with_exit_codes(help::READ_EXAMPLES))]
    Read(ReadArgs),
    #[command(about = help::GUIDE_ABOUT, long_about = None, after_help = help::with_exit_codes(help::GUIDE_EXAMPLES))]
    Guide(GuideArgs),
    #[command(about = help::INSTALL_ABOUT, long_about = None, after_help = help::with_exit_codes(help::INSTALL_EXAMPLES))]
    Install {
        #[command(subcommand)]
        action: InstallCmd,
    },
    /// There is no `uninstall skills`: no verb wrote a skills-directory
    /// pointer to undo.
    #[command(about = help::UNINSTALL_ABOUT, long_about = None, after_help = help::with_exit_codes(help::UNINSTALL_EXAMPLES))]
    Uninstall {
        #[command(subcommand)]
        action: UninstallCmd,
    },
    /// Generated by `clap_complete` straight from this command tree, so it
    /// never drifts from the verbs and flags above.
    #[command(about = help::COMPLETIONS_ABOUT, long_about = None, after_help = help::with_exit_codes(help::COMPLETIONS_EXAMPLES))]
    Completions {
        /// The shell to generate the completion script for.
        shell: Shell,
    },
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
