//! Clap argument and subcommand definitions for the `living-docs` CLI.
//!
//! Docblocks on each verb document its mechanics for a reader of this file;
//! they are no longer the help text (ADR 0060). `--help` comes from the
//! explicit `about`/`after_help` attributes in `args/help.rs`, one short
//! line plus a real `Examples:` block per verb. The root `--help`'s verb
//! listing is grouped into Authoring (`new`, `set`, `supersede`, `index`,
//! `fmt`), Gate (`check`), Reading (`read`, `guide`), and Distribution
//! (`install`, `uninstall`, `completions`) by `help::root_command_template`
//! — clap's `next_help_heading` groups an individual subcommand's own args,
//! not sibling subcommands, so it cannot do this grouping itself.

use crate::output::ColorChoice;
use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
use clap_complete::Shell;

#[path = "args/help.rs"]
mod help;
#[path = "args/sub.rs"]
mod sub;
use std::path::PathBuf;
pub(crate) use sub::{GuideArgs, InstallCmd, ReadArgs, UninstallCmd};

/// Parses argv into a [`Cli`], first grafting the hand-grouped root command
/// listing (`help::root_command_template`) onto the derived [`Command`] —
/// clap's derive has no hook for a custom template, so this stands in for
/// [`Cli::parse`].
pub(crate) fn parse() -> Cli {
    let command = Cli::command().help_template(help::root_command_template());
    let matches = command.get_matches();
    Cli::from_arg_matches(&matches).unwrap_or_else(|err| err.exit())
}

#[derive(Parser)]
#[command(
    name = "living-docs",
    version,
    about = "Deterministic layer of Living Docs authoring. Write ONLY the body below the closing ---. Frontmatter and indexes are CLI-owned: `living-docs set` / `supersede`.",
    after_help = help::EXIT_CODES
)]
pub(crate) struct Cli {
    /// Root of the docs bundle. Overridable so tests can point at a temp tree.
    #[arg(long, global = true, default_value = "docs")]
    pub(crate) docs_dir: PathBuf,

    /// Emit minified single-line JSON instead of plain text, for a data
    /// verb's success output. Overrides TTY autodetection; mutually
    /// exclusive with `--plain` (ADR 0060).
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
        title: String,
        /// Seeds the frontmatter `description:` field with this sentence
        /// instead of the template's placeholder (issue 0021).
        #[arg(long)]
        description: Option<String>,
        /// Architecture views only (ADR 0036): seeds the frontmatter
        /// `kind:` field from the C4/arc42 vocabulary (context, container,
        /// component, flow, sequence, state, data-model, deployment) that
        /// orders the generated architecture index.
        #[arg(long)]
        kind: Option<String>,
        /// Seeds the frontmatter `owner:` field with this value — a
        /// free-form name or email, inserted in canonical position
        /// immediately after `description:`. Never validated against an
        /// identity directory.
        #[arg(long)]
        owner: Option<String>,
        /// Replaces the template's body slots with the text of this file, or
        /// of standard input for `-`; the CLI places it below the heading
        /// and never writes prose itself.
        #[arg(long, value_name = "PATH")]
        body: Option<String>,
    },
    /// `reference` accepts a bare `NNNN`, a type-qualified `TYPE/NNNN`
    /// (e.g. `issue/0028`, the directory name works too), required on a
    /// cross-type number collision, or the path of a record inside the
    /// bundle. `status` is validated against the record's own type
    /// vocabulary (`Superseded` is reserved for `supersede`). `title` also
    /// rewrites the heading, renames the file to the new slug and repoints
    /// every in-bundle reference; it is refused on a record closed for good.
    #[command(about = help::SET_ABOUT, long_about = None, after_help = help::with_exit_codes(help::SET_EXAMPLES))]
    Set {
        reference: String,
        key: String,
        /// The new value; for the `section` key, the section heading.
        value: Option<String>,
        /// Reads the value, or for `section` the text under the heading, from
        /// this file or from standard input for `-`. `section` replaces the
        /// named body section or appends it when absent.
        #[arg(long, value_name = "PATH")]
        from: Option<String>,
    },
    /// `old`/`new` accept the same reference shape as `set`'s `reference`.
    #[command(about = help::SUPERSEDE_ABOUT, long_about = None, after_help = help::with_exit_codes(help::SUPERSEDE_EXAMPLES))]
    Supersede { old: String, new: String },
    /// With no `doc_type`, regenerates every registered type's index.
    #[command(about = help::INDEX_ABOUT, long_about = None, after_help = help::with_exit_codes(help::INDEX_EXAMPLES))]
    Index {
        #[arg(value_name = "TYPE", help = help::doc_type_help())]
        doc_type: Option<String>,
    },
    /// `paths` accepts a bundle root or a single record path; fs-backend
    /// only. `--check` reports what's pending without writing anything.
    #[command(about = help::FMT_ABOUT, long_about = None, after_help = help::with_exit_codes(help::FMT_EXAMPLES))]
    Fmt {
        paths: Vec<PathBuf>,
        #[arg(long)]
        check: bool,
    },
    /// `paths` accepts a bundle root (default `docs`) or, with
    /// `--mermaid-only`, the file(s)/directory(ies) to sweep for
    /// ```mermaid``` fences instead.
    #[command(about = help::CHECK_ABOUT, long_about = None, after_help = help::with_exit_codes(help::CHECK_EXAMPLES))]
    Check {
        paths: Vec<PathBuf>,
        /// Validate only ```mermaid``` fences over `paths`, skipping every other invariant.
        #[arg(long)]
        mermaid_only: bool,
        /// Promotes a missing `owner` on a doctype whose registry row
        /// requires it (ADR) from a warning to an invariant violation.
        #[arg(long)]
        require_owner: bool,
        /// Report only the findings anchored to these records, so a bundle
        /// with legacy debt can gate a commit on what it touched (ADR 0062).
        /// Every invariant still runs over the whole bundle; CI, not this
        /// mode, is what proves the corpus.
        #[arg(long, num_args = 1.., value_name = "PATH")]
        changed_files: Vec<PathBuf>,
        /// Lists every advisory after the per-kind summary instead of the
        /// summary alone; the JSON output is unaffected.
        #[arg(long)]
        advisories: bool,
    },
    /// Renamed from `effective` by ADR 0060; `effective` survives as a
    /// hidden alias for one release.
    #[command(alias = "effective", about = help::READ_ABOUT, long_about = None, after_help = help::with_exit_codes(help::READ_EXAMPLES))]
    Read(ReadArgs),
    /// Renamed from `skill` by ADR 0060; `skill` survives as a hidden alias
    /// for one release, keeping its old `skill <name> --topic <t>` shape
    /// (see `crate::commands::guide::resolve_target`).
    #[command(alias = "skill", about = help::GUIDE_ABOUT, long_about = None, after_help = help::with_exit_codes(help::GUIDE_EXAMPLES))]
    Guide(GuideArgs),
    /// Folds the retired `skill install` verb (ADR 0060).
    #[command(about = help::INSTALL_ABOUT, long_about = None, after_help = help::with_exit_codes(help::INSTALL_EXAMPLES))]
    Install {
        #[command(subcommand)]
        action: InstallCmd,
    },
    /// Folds the retired `hooks uninstall` verb (ADR 0060). There is no
    /// `uninstall skills` — no verb wrote a skills-directory pointer to undo.
    #[command(about = help::UNINSTALL_ABOUT, long_about = None, after_help = help::with_exit_codes(help::UNINSTALL_EXAMPLES))]
    Uninstall {
        #[command(subcommand)]
        action: UninstallCmd,
    },
    /// Generated by `clap_complete` straight from this command tree, so it
    /// never drifts from the verbs/flags above (ADR 0060).
    #[command(about = help::COMPLETIONS_ABOUT, long_about = None, after_help = help::with_exit_codes(help::COMPLETIONS_EXAMPLES))]
    Completions { shell: Shell },
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
