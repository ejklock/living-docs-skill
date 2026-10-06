//! Second-level subcommand enums for `install`, `uninstall`, and `guide`.

use super::help;
use crate::skill_install::Harness;
use clap::{Args, Subcommand};
use std::path::PathBuf;

/// Arguments for the `read` verb (ADR 0050, renamed by ADR 0060), factored
/// into their own `Args` struct so the top-level `Command` enum stays within
/// the file-size ratchet (issue 0028).
#[derive(Args)]
pub(crate) struct ReadArgs {
    /// Restrict the view to records whose title, description, or body
    /// contains this term (case-insensitive). Omitted: everything active.
    #[arg(long)]
    pub(crate) topic: Option<String>,
    /// Print each record's full body instead of a one-line index entry.
    #[arg(long)]
    pub(crate) full: bool,
    /// Print each record's body without its detail-tier sections (an issue's
    /// Plan and Outcome): what was agreed, not the working material.
    #[arg(long, conflicts_with = "full")]
    pub(crate) contract: bool,
}

/// Arguments for the `guide` verb (ADR 0060, renamed from `skill`).
///
/// `topic_or_skill` is a single positional that resolves two ways: when it
/// names an embedded skill (e.g. `okf-knowledge-format`) it selects that
/// skill's corpus with no topic, printing the full `SKILL.md` body; otherwise
/// it is treated as a topic under `--skill` (default `living-docs`). This
/// keeps `guide adr` reading as "the adr topic" while `guide
/// okf-knowledge-format` still reads as "that skill's overview" — see
/// `crate::commands::guide::resolve_target` for the exact precedence,
/// including the hidden `--topic` flag that keeps the retired `skill <name>
/// --topic <t>` alias shape working.
#[derive(Args)]
pub(crate) struct GuideArgs {
    pub(crate) topic_or_skill: Option<String>,
    /// The embedded skill corpus to read from; defaults to `living-docs`.
    #[arg(long)]
    pub(crate) skill: Option<String>,
    /// Retired `skill` alias shape: `skill <name> --topic <t>`. Hidden — use
    /// the `topic_or_skill` positional instead.
    #[arg(long, hide = true)]
    pub(crate) topic: Option<String>,
    /// List every embedded skill and its available topics instead of
    /// printing a single skill's content.
    #[arg(long)]
    pub(crate) list: bool,
}

#[derive(Subcommand)]
pub(crate) enum InstallCmd {
    /// Places the three skill directories from the embedded corpus into a
    /// harness's skills directory (ADR 0028) — no working tree involved.
    /// `--project` scopes the destination to the current project instead of
    /// the harness's global, `$HOME`-rooted directory; `--dir` overrides the
    /// destination outright.
    #[command(
        about = help::INSTALL_SKILLS_ABOUT,
        long_about = None,
        after_help = help::with_exit_codes(help::INSTALL_SKILLS_EXAMPLES)
    )]
    Skills {
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
    /// Removes the artifacts `install hooks` wrote — the
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
