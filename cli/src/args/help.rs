//! Help text for every verb: a one-line `about`, the example invocations and
//! the exit-code table as structured data, and the vocabularies rendered from
//! the registries. The text help and the JSON help both read these, so the
//! two renderings cannot disagree.

use living_docs_core::check::kinds;
use living_docs_core::doc_type::{Identity, DOC_TYPES};

/// The exit-code table every verb's help ends with.
pub(crate) const EXIT_CODES: [(u8, &str); 3] = [
    (0, "success"),
    (1, "a gate or a verb's own check failed"),
    (2, "invalid usage or a missing input"),
];

/// The frontmatter keys `write set` accepts.
const SET_KEYS: &str =
    "status, description, owner, tracker, labels, blocked_by, title, number, section";

/// The `Exit codes:` block of the text help.
pub(crate) fn exit_codes_text() -> String {
    let rows: String = EXIT_CODES
        .iter()
        .map(|(code, meaning)| format!("\n  {code}  {meaning}"))
        .collect();
    format!("Exit codes:{rows}")
}

/// The `Examples:` block of the text help.
pub(crate) fn examples_text(examples: &[&str]) -> String {
    let rows: String = examples
        .iter()
        .map(|example| format!("\n  {example}"))
        .collect();
    format!("Examples:{rows}")
}

/// A verb's `Examples:` block followed by the exit-code table, so every
/// verb's `--help` ends the same way the root help does.
pub(crate) fn with_exit_codes(examples: &[&str]) -> String {
    format!("{}\n\n{}", examples_text(examples), exit_codes_text())
}

/// Renders the registered record types for a `TYPE` positional's help, read
/// from the registry at runtime so a new row appears without a help edit.
pub(crate) fn doc_type_help() -> String {
    let tokens = DOC_TYPES.iter().map(|spec| spec.token).collect::<Vec<_>>();
    format!("one of: {}", tokens.join(", "))
}

/// The status vocabulary of every type that has one, as `Type: status, status`.
fn status_vocabularies() -> String {
    DOC_TYPES
        .iter()
        .filter(|spec| !spec.status_vocabulary.is_empty())
        .map(|spec| {
            format!(
                "{}: {}",
                spec.frontmatter,
                spec.status_vocabulary.join(", ")
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// Help for `write set`'s `KEY`: the keys and the status values per type.
pub(crate) fn set_key_help() -> String {
    format!(
        "one of: {}. `status` takes the record's own type vocabulary ({}); Superseded is set only by `write supersede`. `number` takes NNNN or `next` and renumbers the record with its heading, links and index; `section` takes the section heading as VALUE and its text from --from",
        SET_KEYS,
        status_vocabularies()
    )
}

fn directory_names() -> String {
    DOC_TYPES
        .iter()
        .filter_map(|spec| match spec.identity {
            Identity::Numbered { dir } | Identity::Named { dir } => Some(dir),
            Identity::Singleton { .. } => None,
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Help for a positional that names a record: the three accepted forms.
pub(crate) fn reference_help() -> String {
    let tokens = DOC_TYPES.iter().map(|spec| spec.token).collect::<Vec<_>>();
    format!(
        "a record, as NNNN (a bare number searched across every type; an ambiguous one is an error naming the candidates), TYPE/NNNN (TYPE is a type token, {}, or its directory name, {}; for example adr/0012 or issues/0049), or the path of a record inside the bundle, ending in .md. A NNNN or TYPE/NNNN that matches two records is an error naming the candidates; name the one you mean by its path",
        tokens.join(", "),
        directory_names()
    )
}

/// Help for `new --owner`, naming the types that get a seeded owner.
pub(crate) fn owner_help() -> String {
    let seeded = DOC_TYPES
        .iter()
        .filter(|spec| spec.requires_owner)
        .map(|spec| spec.token)
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "Seeds the frontmatter `owner:` field with this value, a free-form name or email that is never validated. When omitted it defaults to git config user.name, then user.email, on types that require an owner ({seeded})"
    )
}

/// Help for `read guide`'s positional: how it resolves and every topic by skill.
pub(crate) fn guide_topic_help() -> String {
    let listing = crate::skill::list().unwrap_or_default();
    let by_skill = listing.lines().collect::<Vec<_>>().join("; ");
    format!(
        "a topic of the skill chosen by --skill (default living-docs), or the name of a skill to print its full SKILL.md. Topics by skill, as `living-docs read guide --list` prints them: {by_skill}"
    )
}

/// The `check` description: what it gates and the advisory kinds it reports.
pub(crate) fn check_long_about() -> String {
    let rows: String = kinds::ALL
        .iter()
        .map(|kind| format!("\n  {}  {}", kind.name, kind.meaning))
        .collect();
    format!(
        "{CHECK_ABOUT}\n\nViolations exit 1; advisories never change the exit code. Each advisory carries one of these kinds:{rows}"
    )
}

pub(crate) const NEW_ABOUT: &str =
    "Scaffolds a new record with CLI-owned numbering, frontmatter, and heading.";
pub(crate) const NEW_EXAMPLES: &[&str] = &[
    "living-docs write new adr \"Cache invalidation strategy\"",
    "living-docs write new issue \"Flaky upload test\" --owner alice@example.com",
    "living-docs write new issue \"Flaky upload test\" --body body.md",
    "cat body.md | living-docs write new issue \"Flaky upload test\" --body -",
];

pub(crate) const SET_ABOUT: &str =
    "Sets one CLI-owned frontmatter field (status, description, owner, title, number; tracker, labels, blocked_by on an issue) or replaces a body section.";
pub(crate) const SET_EXAMPLES: &[&str] = &[
    "living-docs write set 0012 status Accepted",
    "living-docs write set adr/0012 owner carol@example.com",
    "living-docs write set issues/0049-flaky-upload-test.md status closed",
    "living-docs write set adr/0012 title \"Cache invalidation is write-through\"",
    "living-docs write set issues/0055-a-title.md number next",
    "living-docs write set 0012 description --from description.txt",
    "living-docs write set issue/0049 section Outcome --from outcome.md",
    "echo closed | living-docs write set 0049 status --from -",
];

pub(crate) const SUPERSEDE_ABOUT: &str =
    "Retires an old record in favor of a new one, wiring both link directions.";
pub(crate) const SUPERSEDE_EXAMPLES: &[&str] = &["living-docs write supersede 0011 0012"];

pub(crate) const INDEX_ABOUT: &str = "Rebuilds a doc type's index.md from the records on disk.";
pub(crate) const INDEX_EXAMPLES: &[&str] =
    &["living-docs manage index adr", "living-docs manage index"];

pub(crate) const FMT_ABOUT: &str =
    "Canonicalizes a record's frontmatter, seeds a missing timestamp and required owner, aligns the title heading, or reports what's pending.";
pub(crate) const FMT_EXAMPLES: &[&str] = &[
    "living-docs manage fmt docs",
    "living-docs manage fmt --check docs",
];

pub(crate) const CHECK_ABOUT: &str = "Validates the bundle's mechanical invariants — the doc-gate.";
pub(crate) const CHECK_EXAMPLES: &[&str] = &[
    "living-docs check docs",
    "living-docs check --require-owner docs",
    "living-docs check --changed-files notes/0001-x.md -- docs",
];

pub(crate) const WRITE_ABOUT: &str = "Creates and changes records: new, set, supersede.";
pub(crate) const WRITE_EXAMPLES: &[&str] = &[
    "living-docs write new adr \"Cache invalidation strategy\"",
    "living-docs write set 0012 status Accepted",
    "living-docs write supersede 0011 0012",
];

pub(crate) const READ_ABOUT: &str =
    "Reads the bundle and the embedded skill corpus: records, guide.";
pub(crate) const READ_EXAMPLES: &[&str] = &[
    "living-docs read records --topic caching",
    "living-docs read guide adr",
];

pub(crate) const MANAGE_ABOUT: &str =
    "Maintains the bundle and its tooling: index, fmt, install, uninstall, completions.";
pub(crate) const MANAGE_EXAMPLES: &[&str] = &[
    "living-docs manage index adr",
    "living-docs manage fmt docs",
    "living-docs manage install hooks",
];

pub(crate) const RECORDS_ABOUT: &str =
    "Prints the agent-facing in-force view: active records, supersede chains collapsed.";
pub(crate) const RECORDS_EXAMPLES: &[&str] = &[
    "living-docs read records --topic caching",
    "living-docs read records --full",
    "living-docs read records --links-to adr/0012",
    "living-docs read records --impact adr/0012 --json",
];

pub(crate) const GUIDE_ABOUT: &str =
    "Serves the embedded skill corpus: topics, templates, and full SKILL.md bodies.";
pub(crate) const GUIDE_EXAMPLES: &[&str] = &[
    "living-docs read guide adr",
    "living-docs read guide --list",
    "living-docs read guide conformance --skill okf-knowledge-format",
];

pub(crate) const INSTALL_ABOUT: &str =
    "Places the embedded skill corpus or the enforcement hooks into a project.";
pub(crate) const INSTALL_EXAMPLES: &[&str] = &[
    "living-docs manage install skills --harness claude",
    "living-docs manage install hooks",
];

pub(crate) const UNINSTALL_ABOUT: &str = "Removes what `manage install hooks` placed.";
pub(crate) const UNINSTALL_EXAMPLES: &[&str] = &["living-docs manage uninstall hooks"];

pub(crate) const INSTALL_SKILLS_ABOUT: &str =
    "Places the embedded skill directories into a harness's skills directory.";
pub(crate) const INSTALL_SKILLS_EXAMPLES: &[&str] = &[
    "living-docs manage install skills --harness claude",
    "living-docs manage install skills --project",
];

pub(crate) const INSTALL_HOOKS_ABOUT: &str =
    "Writes the session-teaching hook and the pre-commit doc-gate into a project.";
pub(crate) const INSTALL_HOOKS_EXAMPLES: &[&str] = &[
    "living-docs manage install hooks",
    "living-docs manage install hooks --dir ./my-project",
];

pub(crate) const UNINSTALL_HOOKS_ABOUT: &str = "Removes what `manage install hooks` placed.";
pub(crate) const UNINSTALL_HOOKS_EXAMPLES: &[&str] = &[
    "living-docs manage uninstall hooks",
    "living-docs manage uninstall hooks --dir ./my-project",
];

pub(crate) const COMPLETIONS_ABOUT: &str = "Prints a shell completion script.";
pub(crate) const COMPLETIONS_EXAMPLES: &[&str] = &[
    "living-docs manage completions bash > /etc/bash_completion.d/living-docs",
    "living-docs manage completions zsh > \"${fpath[1]}/_living-docs\"",
];

pub(crate) const MCP_ABOUT: &str =
    "Serves the bundle to an MCP client over stdio as the write, read, manage and check tools.";
pub(crate) const MCP_EXAMPLES: &[&str] = &[
    "living-docs mcp --docs-dir docs",
    "claude mcp add living-docs -- living-docs mcp --docs-dir docs",
];

const EXAMPLES: [(&[&str], &[&str]); 18] = [
    (&["write"], WRITE_EXAMPLES),
    (&["write", "new"], NEW_EXAMPLES),
    (&["write", "set"], SET_EXAMPLES),
    (&["write", "supersede"], SUPERSEDE_EXAMPLES),
    (&["check"], CHECK_EXAMPLES),
    (&["mcp"], MCP_EXAMPLES),
    (&["read"], READ_EXAMPLES),
    (&["read", "records"], RECORDS_EXAMPLES),
    (&["read", "guide"], GUIDE_EXAMPLES),
    (&["manage"], MANAGE_EXAMPLES),
    (&["manage", "index"], INDEX_EXAMPLES),
    (&["manage", "fmt"], FMT_EXAMPLES),
    (&["manage", "install"], INSTALL_EXAMPLES),
    (&["manage", "install", "skills"], INSTALL_SKILLS_EXAMPLES),
    (&["manage", "install", "hooks"], INSTALL_HOOKS_EXAMPLES),
    (&["manage", "uninstall"], UNINSTALL_EXAMPLES),
    (&["manage", "uninstall", "hooks"], UNINSTALL_HOOKS_EXAMPLES),
    (&["manage", "completions"], COMPLETIONS_EXAMPLES),
];

/// The examples of the command addressed by `path` (the subcommand names
/// below the root; empty for the root, which has none).
pub(crate) fn examples_for(path: &[&str]) -> &'static [&'static str] {
    EXAMPLES
        .iter()
        .find(|(key, _)| *key == path)
        .map_or(&[], |(_, examples)| *examples)
}
