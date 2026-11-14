//! Operation enums and argument structs for `read`, `manage install` and `manage uninstall`.

use super::help;
use crate::skill_install::Harness;
use clap::{Args, Subcommand};
use std::path::PathBuf;

/// Arguments for `write new`.
#[derive(Args)]
pub(crate) struct NewRecordArgs {
    #[arg(value_name = "TYPE", help = help::doc_type_help())]
    pub(crate) doc_type: String,
    /// Title of the new record; it becomes the heading and the file slug.
    pub(crate) title: String,
    /// Seeds the frontmatter `description:` field with this sentence
    /// instead of the template's placeholder.
    #[arg(long)]
    pub(crate) description: Option<String>,
    /// Architecture views only: seeds the frontmatter `kind:` field from
    /// the C4/arc42 vocabulary (context, container, component, flow,
    /// sequence, state, data-model, deployment) that orders the
    /// generated architecture index.
    #[arg(long)]
    pub(crate) kind: Option<String>,
    #[arg(long, help = help::owner_help())]
    pub(crate) owner: Option<String>,
    /// Replaces the template's body slots with the text of this file, or
    /// of standard input for `-`; the CLI places it below the heading
    /// and never writes prose itself.
    #[arg(long, value_name = "PATH")]
    pub(crate) body: Option<String>,
}

/// Arguments for the `check` verb.
#[derive(Args)]
pub(crate) struct CheckArgs {
    /// The bundle root to validate (default docs) or, with
    /// `--mermaid-only`, the file(s) or directory(ies) to sweep for
    /// mermaid fences.
    pub(crate) paths: Vec<PathBuf>,
    /// Validate only mermaid fences over `paths`, skipping every other invariant.
    #[arg(long)]
    pub(crate) mermaid_only: bool,
    /// Promotes a missing `owner` on a doctype whose registry row
    /// requires it from a warning to an invariant violation.
    #[arg(long)]
    pub(crate) require_owner: bool,
    /// Report only the findings anchored to these records, so a bundle
    /// with legacy debt can gate a commit on what it touched. Every
    /// invariant still runs over the whole bundle.
    #[arg(long, num_args = 1.., value_name = "PATH")]
    pub(crate) changed_files: Vec<PathBuf>,
    /// Lists every advisory after the per-kind summary instead of the
    /// summary alone; the JSON output is unaffected.
    #[arg(long)]
    pub(crate) advisories: bool,
}

/// The `read` verb's operations: the in-force record view and the embedded
/// skill corpus.
#[derive(Subcommand)]
pub(crate) enum ReadCmd {
    #[command(about = help::RECORDS_ABOUT, long_about = None, after_help = help::with_exit_codes(help::RECORDS_EXAMPLES))]
    Records(ReadArgs),
    #[command(about = help::GUIDE_ABOUT, long_about = None, after_help = help::with_exit_codes(help::GUIDE_EXAMPLES))]
    Guide(GuideArgs),
}

/// Arguments for `read records`, factored into their own `Args` struct so the
/// top-level `Command` enum stays within the file-size ratchet.
#[derive(Args)]
pub(crate) struct ReadArgs {
    /// Restrict the view to records where every whitespace-separated term
    /// starts a word of the title, description, or body (case- and
    /// accent-insensitive, never mid-word), best match first. Omitted:
    /// everything active.
    #[arg(long)]
    pub(crate) topic: Option<String>,
    /// Print each record's full body instead of a one-line index entry.
    #[arg(long)]
    pub(crate) full: bool,
    /// Print each record's body without its detail-tier sections (an issue's
    /// Plan and Outcome): what was agreed, not the working material.
    #[arg(long, conflicts_with = "full")]
    pub(crate) contract: bool,
    /// Print only the live records whose body links the named record
    /// (`TYPE/NNNN`), directly, in the usual order. Run it before editing
    /// or superseding a record.
    #[arg(long, value_name = "TYPE/NNNN", conflicts_with_all = ["topic", "impact"])]
    pub(crate) links_to: Option<String>,
    /// Print every live record that reaches the named record (`TYPE/NNNN`)
    /// through a chain of links, each with its distance (1 is a direct
    /// link), nearest first.
    #[arg(long, value_name = "TYPE/NNNN", conflicts_with = "topic")]
    pub(crate) impact: Option<String>,
}

/// Arguments for `read guide`.
///
/// `topic_or_skill` is a single positional that resolves two ways: when it
/// names an embedded skill (e.g. `okf-knowledge-format`) it selects that
/// skill's corpus with no topic, printing the full `SKILL.md` body; otherwise
/// it is treated as a topic under `--skill` (default `living-docs`). This
/// keeps `read guide adr` reading as "the adr topic" while `read guide
/// okf-knowledge-format` still reads as "that skill's overview" — see
/// `crate::commands::guide::resolve_target` for the exact precedence.
#[derive(Args)]
pub(crate) struct GuideArgs {
    #[arg(help = help::guide_topic_help())]
    pub(crate) topic_or_skill: Option<String>,
    /// The embedded skill corpus to read from; defaults to `living-docs`.
    #[arg(long)]
    pub(crate) skill: Option<String>,
    /// List every embedded skill and its available topics instead of
    /// printing a single skill's content.
    #[arg(long)]
    pub(crate) list: bool,
}

#[derive(Subcommand)]
pub(crate) enum InstallCmd {
    /// Places the three skill directories from the embedded corpus into a
    /// harness's skills directory — no working tree involved. `--project`
    /// scopes the destination to the current project instead of the
    /// harness's global, `$HOME`-rooted directory; `--dir` overrides the
    /// destination outright.
    #[command(
        about = help::INSTALL_SKILLS_ABOUT,
        long_about = None,
        after_help = help::with_exit_codes(help::INSTALL_SKILLS_EXAMPLES)
    )]
    Skills {
        /// The harness whose skills directory receives the skills.
        #[arg(long, value_enum, default_value = "claude")]
        harness: Harness,
        /// Scope the destination to the current project's harness directory
        /// instead of the harness's global, `$HOME`-rooted one.
        #[arg(long)]
        project: bool,
        /// Destination root for the skill directories, overriding both
        /// `--harness` and `--project` outright. When given, `--harness`
        /// still parses but no longer affects where anything is placed.
        #[arg(long)]
        dir: Option<PathBuf>,
        /// Report the plan without writing any file.
        #[arg(long)]
        dry_run: bool,
    },
    /// Writes the session-teaching hook into `<dir>/.living-docs/hooks/`
    /// at mode 0755, materializes the pre-commit doc-gate to
    /// `<dir>/.githooks/pre-commit` (pointing `core.hooksPath` at it), and
    /// wires the `SessionStart` hook into `<dir>/.claude/settings.json`,
    /// idempotently — re-running replaces the living-docs entries by
    /// identity rather than appending. The generated commands pin the
    /// resolved `--docs-dir` bundle as a `LIVING_DOCS_BUNDLE=` prefix.
    /// `--dry-run` reports the same plan without writing anything.
    #[command(
        about = help::INSTALL_HOOKS_ABOUT,
        long_about = None,
        after_help = help::with_exit_codes(help::INSTALL_HOOKS_EXAMPLES)
    )]
    Hooks {
        /// Target project root; defaults to the current directory.
        #[arg(long)]
        dir: Option<PathBuf>,
        /// Report the plan without writing any file or directory.
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
pub(crate) enum UninstallCmd {
    /// Removes the artifacts `manage install hooks` wrote — the
    /// `.living-docs/hooks/` script, `.githooks/pre-commit`, and the
    /// living-docs entries in `<dir>/.claude/settings.json` — leaving
    /// unrelated entries and `core.hooksPath` untouched. A clean no-op when
    /// nothing was installed. `--dry-run` reports the same removal plan
    /// without deleting anything.
    #[command(
        about = help::UNINSTALL_HOOKS_ABOUT,
        long_about = None,
        after_help = help::with_exit_codes(help::UNINSTALL_HOOKS_EXAMPLES)
    )]
    Hooks {
        /// Target project root; defaults to the current directory.
        #[arg(long)]
        dir: Option<PathBuf>,
        /// Report the plan without removing any file.
        #[arg(long)]
        dry_run: bool,
    },
}
