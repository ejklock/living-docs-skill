//! Clap argument and subcommand definitions for the `living-docs` CLI.

use crate::config::{Backend, Engine};
use crate::skill_install::Harness;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "living-docs",
    version,
    about = "Deterministic layer of Living Docs authoring. Write ONLY the body below the closing ---. Frontmatter and indexes are CLI-owned: `living-docs status` / `supersede` / `index`."
)]
pub(crate) struct Cli {
    /// Root of the docs bundle. Overridable so tests can point at a temp tree.
    #[arg(long, global = true, default_value = "docs")]
    pub(crate) docs_dir: PathBuf,

    /// Which persistence backend `new`/`check`/`export`/`index`/`supersede`
    /// operate against: the local `.md` tree (`fs`, default) or the
    /// SQLite/ParadeDB read-model (`db`), scoped to a project derived from
    /// `--docs-dir` (ADR 0007, issue 0006 slices 0006-D2/0006-E). `index`'s
    /// output artifact (`index.md`) is always written to the filesystem
    /// regardless of this flag — only the records feeding it move through
    /// the active backend (ADR 0007: `index.md` is fs-only).
    #[arg(long, global = true, value_enum, default_value = "fs")]
    pub(crate) backend: Backend,

    /// Which database engine `db sync`/`search`, and any `--backend db`
    /// authoring command, connects to: ParadeDB via `$DATABASE_URL` (the
    /// default, ADR 0004) or the local embedded SQLite/FTS5 file (`sqlite`,
    /// opt-in, falling back to `.living-docs/index.db` when `$DATABASE_URL`
    /// is unset).
    #[arg(long, global = true, value_enum, default_value = "paradedb")]
    pub(crate) engine: Engine,

    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Subcommand)]
pub(crate) enum Command {
    New {
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
    },
    /// `new` plus deterministic pre-fill (issue 0008): frontmatter title,
    /// numbered title heading, a trail comment, and every judgment section
    /// collapsed to a marked empty `<!-- judgment: ... -->` slot the
    /// authoring model fills.
    Brief {
        doc_type: String,
        title: String,
        /// Git range (e.g. HEAD~3..HEAD) whose touched files are listed —
        /// verbatim from `git diff --name-only` — under the context slot.
        #[arg(long)]
        from_diff: Option<String>,
    },
    Index {
        doc_type: Option<String>,
        /// Restrict the rendered index to records whose effective visibility
        /// (frontmatter `visibility`, or `private` when absent — default-deny,
        /// ADR 0009) is in this comma-separated set. Omitted: every record
        /// renders, unchanged from today's dev view.
        #[arg(long, value_delimiter = ',')]
        visibility: Option<Vec<String>>,
    },
    /// `old` and `new` each accept a bare `NNNN` or a type-qualified
    /// `TYPE/NNNN` reference (e.g. `issue/0028`) — required when the same
    /// number exists in more than one doc-type directory, since a bare
    /// `NNNN` fails loudly on that collision instead of guessing (issue
    /// 0029/0025).
    Supersede {
        old: String,
        new: String,
    },
    /// Sets a record's `status:` frontmatter field directly — for the
    /// `Proposed`/`Accepted`/`Deprecated` lifecycle only. `Superseded` is
    /// rejected; use `supersede`, which also wires the
    /// `supersedes`/`superseded_by` links. `number` accepts a bare `NNNN`
    /// or a type-qualified `TYPE/NNNN` reference (e.g. `issue/0028`),
    /// required when the same number exists in more than one doc-type
    /// directory (issue 0029/0025).
    Status {
        number: String,
        new_status: String,
    },
    /// Sets a record's `description:` frontmatter field directly — the
    /// CLI-owned counterpart to hand-editing the placeholder, reusing the
    /// same record-resolution and frontmatter-mutation helpers `status`
    /// uses (issue 0021, part 2 of 2). Unlike `status`, no vocabulary
    /// constrains the sentence; any string is accepted. `number` accepts a
    /// bare `NNNN` or a type-qualified `TYPE/NNNN` reference (e.g.
    /// `issue/0028`), required when the same number exists in more than one
    /// doc-type directory (issue 0029/0025).
    Describe {
        number: String,
        description: String,
    },
    Next {
        doc_type: String,
    },
    /// Validate the mechanical Living Docs invariants on a docs bundle, matching
    /// `lint-docs.sh`'s `[BUNDLE_ROOT]` argument (default `docs`) rather than the
    /// global `--docs-dir`. With `--mermaid-only`, `paths` instead lists the
    /// file(s)/directory(ies) to sweep for ```mermaid``` fences (default:
    /// git-tracked `*.md`, fixtures dir excluded), matching `lint-mermaid.sh`.
    Check {
        paths: Vec<PathBuf>,
        /// Validate only ```mermaid``` fences over `paths`, skipping every other invariant.
        #[arg(long)]
        mermaid_only: bool,
    },
    /// Canonicalizes every concept record's frontmatter in place — the
    /// remediation verb for `check`'s canonical-frontmatter invariant (ADR
    /// 0019). Matches `check`'s own `[BUNDLE_ROOT]` argument rather than the
    /// global `--docs-dir`; fs-backend only, since db-mode is canonical by
    /// construction on export.
    Fmt {
        paths: Vec<PathBuf>,
    },
    /// Read-only adaptation advisor (ADR 0037): prints an ordered plan of
    /// RUN (mechanical), AUTHOR (judgment) or ADOPT (bootstrap) steps.
    Migrate {
        paths: Vec<PathBuf>,
    },
    /// Materializes every record the active `--backend` lists back into
    /// conformant `.md` files under `out_dir` — the lossless round-trip
    /// fitness function (ADR 0007, issue 0006 slice 0006-D2).
    Export {
        out_dir: PathBuf,
        /// Restrict the exported set to records whose effective visibility
        /// (frontmatter `visibility`, or `private` when absent —
        /// default-deny, ADR 0010) is in this comma-separated set. Omitted:
        /// every record exports, unchanged from today's behavior.
        #[arg(long, value_delimiter = ',')]
        visibility: Option<Vec<String>>,
    },
    /// Operate on the derived read-model — ParadeDB via `$DATABASE_URL` by
    /// default (ADR 0004), or the local embedded SQLite/FTS5 file with
    /// `--engine sqlite`.
    Db {
        #[command(subcommand)]
        cmd: DbCmd,
    },
    /// Fails closed when an exported bundle leaks a private doc, or a
    /// dangling link to a doc withheld from the bundle (ADR 0010 leak gate,
    /// part 1 — always inspects a materialized filesystem bundle, regardless
    /// of `--backend`).
    LeakGate {
        bundle: PathBuf,
        /// Additionally runs the Tier-3 PII detectors (ADR 0012) — the
        /// highest-false-positive class, so they stay opt-in rather than
        /// running by default.
        #[arg(long)]
        check_tier3: bool,
    },
    /// Full-text search the derived read-model, ranked best-match-first.
    Search {
        query: String,
        /// Narrow results to one project's slug. Omitted spans every
        /// project, labeling each hit by the project it belongs to (ADR
        /// 0005, issue 0005 slice 0005-C1).
        #[arg(long)]
        project: Option<String>,
    },
    /// Serves skill content embedded in the binary at compile time (ADR
    /// 0014): list embedded skills and their topics, print a skill's full
    /// `SKILL.md` body, or print one topic's detail. `skill install` (ADR
    /// 0028) places the corpus into a harness's skills directory instead.
    Skill {
        /// The skill to query, e.g. `living-docs`. Required unless `--list`.
        name: Option<String>,
        /// Print only this topic's detail instead of the full `SKILL.md`
        /// body; maps to a `rules/`/`templates/` basename.
        #[arg(long)]
        topic: Option<String>,
        /// List every embedded skill and its available topics instead of
        /// printing a single skill's content.
        #[arg(long)]
        list: bool,
        /// Emit minified single-line JSON instead of plain text, for
        /// consumption by other agents. Only changes the success-output
        /// shape; errors still print to stderr as plain text. Overrides TTY
        /// autodetection; mutually exclusive with `--plain`.
        #[arg(long)]
        json: bool,
        /// Force human-readable plain text, overriding TTY autodetection.
        /// Mutually exclusive with `--json`.
        #[arg(long, conflicts_with = "json")]
        plain: bool,
        #[command(subcommand)]
        action: Option<SkillCmd>,
    },
    /// Materializes the corpus hook scripts into a target project (ADR 0023).
    Hooks {
        #[command(subcommand)]
        cmd: HooksCmd,
    },
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
