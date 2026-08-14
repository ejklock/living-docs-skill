//! Second-level subcommand enums for `seal`, `hooks`, `skill`, and `db`.

use crate::skill_install::Harness;
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand)]
pub(crate) enum SealCmd {
    /// Generates a fresh per-clone key and seals the current bundle as the
    /// trusted baseline — run after clone, merge, or adoption.
    Init,
}

#[derive(Subcommand)]
pub(crate) enum HooksCmd {
    /// Writes the two corpus hook scripts into `<dir>/.living-docs/hooks/`
    /// at mode 0755, materializes the pre-commit doc-gate to
    /// `<dir>/.githooks/pre-commit` (pointing `core.hooksPath` at it), and
    /// wires the Claude Code hooks into `<dir>/.claude/settings.json`,
    /// idempotently — re-running replaces the living-docs entries by
    /// identity rather than appending. The generated commands pin the
    /// resolved `--docs-dir` bundle as a `LIVING_DOCS_BUNDLE=` prefix.
    /// `--dry-run` reports the same plan without writing anything.
    Install {
        /// Target project root; defaults to the current directory.
        #[arg(long)]
        dir: Option<PathBuf>,
        /// Report the plan without writing any file or directory.
        #[arg(long)]
        dry_run: bool,
    },
    /// Removes the artifacts `install` wrote — the two `.living-docs/hooks/`
    /// scripts, `.githooks/pre-commit`, and the living-docs entries in
    /// `<dir>/.claude/settings.json` — leaving unrelated entries and
    /// `core.hooksPath` untouched. A clean no-op when nothing was installed.
    /// `--dry-run` reports the same removal plan without deleting anything.
    Uninstall {
        /// Target project root; defaults to the current directory.
        #[arg(long)]
        dir: Option<PathBuf>,
        /// Report the plan without removing any file.
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
pub(crate) enum SkillCmd {
    /// Places the three skill directories from the embedded corpus into a
    /// harness's skills directory (ADR 0028) — no working tree involved.
    /// `--project` scopes the destination to the current project instead of
    /// the harness's global, `$HOME`-rooted directory; `--dir` overrides the
    /// destination outright.
    Install {
        #[arg(long, value_enum, default_value = "claude")]
        harness: Harness,
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
}

#[derive(Subcommand)]
pub(crate) enum DbCmd {
    /// Rebuild the read-model from every doc `--docs-dir` lists, scoped to
    /// one named project (ADR 0005, issue 0005 slice 0005-B).
    Sync {
        /// The project slug to sync into. Defaults to a slug derived from
        /// `--docs-dir`: its own directory name, or its parent directory's
        /// name when the final component is literally `docs` — so every
        /// repo's `<repo>/docs` bundle gets a project unique to that repo
        /// instead of every repo colliding on the literal word `docs`.
        #[arg(long)]
        project: Option<String>,
    },
}
