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

/// The frontmatter keys `set` accepts.
const SET_KEYS: [&str; 5] = ["status", "description", "owner", "title", "section"];

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

/// Help for `set`'s `KEY`: the five keys and the status values per type.
pub(crate) fn set_key_help() -> String {
    format!(
        "one of: {}. `status` takes the record's own type vocabulary ({}); Superseded is set only by `supersede`. `section` takes the section heading as VALUE and its text from --from",
        SET_KEYS.join(", "),
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
        "a record, as NNNN (a bare number searched across every type; an ambiguous one is an error naming the candidates), TYPE/NNNN (TYPE is a type token, {}, or its directory name, {}; for example adr/0012 or issues/0049), or the path of a record inside the bundle, ending in .md",
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

/// Help for `guide`'s positional: how it resolves and every topic by skill.
pub(crate) fn guide_topic_help() -> String {
    let listing = crate::skill::list().unwrap_or_default();
    let by_skill = listing.lines().collect::<Vec<_>>().join("; ");
    format!(
        "a topic of the skill chosen by --skill (default living-docs), or the name of a skill to print its full SKILL.md. Topics by skill, as `living-docs guide --list` prints them: {by_skill}"
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
    "living-docs new adr \"Cache invalidation strategy\"",
    "living-docs new issue \"Flaky upload test\" --owner alice@example.com",
    "living-docs new issue \"Flaky upload test\" --body body.md",
    "cat body.md | living-docs new issue \"Flaky upload test\" --body -",
];

pub(crate) const SET_ABOUT: &str =
    "Sets one CLI-owned frontmatter field (status, description, owner, title) or replaces a body section.";
pub(crate) const SET_EXAMPLES: &[&str] = &[
    "living-docs set 0012 status Accepted",
    "living-docs set adr/0012 owner carol@example.com",
    "living-docs set issues/0049-flaky-upload-test.md status closed",
    "living-docs set adr/0012 title \"Cache invalidation is write-through\"",
    "living-docs set 0012 description --from description.txt",
    "living-docs set issue/0049 section Outcome --from outcome.md",
    "echo closed | living-docs set 0049 status --from -",
];

pub(crate) const SUPERSEDE_ABOUT: &str =
    "Retires an old record in favor of a new one, wiring both link directions.";
pub(crate) const SUPERSEDE_EXAMPLES: &[&str] = &["living-docs supersede 0011 0012"];

pub(crate) const INDEX_ABOUT: &str = "Rebuilds a doc type's index.md from the records on disk.";
pub(crate) const INDEX_EXAMPLES: &[&str] = &["living-docs index adr", "living-docs index"];

pub(crate) const FMT_ABOUT: &str =
    "Canonicalizes a record's frontmatter in place, or reports what's pending.";
pub(crate) const FMT_EXAMPLES: &[&str] = &["living-docs fmt docs", "living-docs fmt --check docs"];

pub(crate) const CHECK_ABOUT: &str = "Validates the bundle's mechanical invariants — the doc-gate.";
pub(crate) const CHECK_EXAMPLES: &[&str] = &[
    "living-docs check docs",
    "living-docs check --require-owner docs",
    "living-docs check --changed-files notes/0001-x.md -- docs",
];

pub(crate) const READ_ABOUT: &str =
    "Prints the agent-facing in-force view: active records, supersede chains collapsed.";
pub(crate) const READ_EXAMPLES: &[&str] = &[
    "living-docs read --topic caching",
    "living-docs read --full",
];

pub(crate) const GUIDE_ABOUT: &str =
    "Serves the embedded skill corpus: topics, templates, and full SKILL.md bodies.";
pub(crate) const GUIDE_EXAMPLES: &[&str] = &[
    "living-docs guide adr",
    "living-docs guide --list",
    "living-docs guide conformance --skill okf-knowledge-format",
];

pub(crate) const INSTALL_ABOUT: &str =
    "Places the embedded skill corpus or the enforcement hooks into a project.";
pub(crate) const INSTALL_EXAMPLES: &[&str] = &[
    "living-docs install skills --harness claude",
    "living-docs install hooks",
];

pub(crate) const UNINSTALL_ABOUT: &str = "Removes what `install hooks` placed.";
pub(crate) const UNINSTALL_EXAMPLES: &[&str] = &["living-docs uninstall hooks"];

pub(crate) const INSTALL_SKILLS_ABOUT: &str =
    "Places the embedded skill directories into a harness's skills directory.";
pub(crate) const INSTALL_SKILLS_EXAMPLES: &[&str] = &[
    "living-docs install skills --harness claude",
    "living-docs install skills --project",
];

pub(crate) const INSTALL_HOOKS_ABOUT: &str =
    "Writes the session-teaching hook and the pre-commit doc-gate into a project.";
pub(crate) const INSTALL_HOOKS_EXAMPLES: &[&str] = &[
    "living-docs install hooks",
    "living-docs install hooks --dir ./my-project",
];

pub(crate) const UNINSTALL_HOOKS_ABOUT: &str = "Removes what `install hooks` placed.";
pub(crate) const UNINSTALL_HOOKS_EXAMPLES: &[&str] = &[
    "living-docs uninstall hooks",
    "living-docs uninstall hooks --dir ./my-project",
];

pub(crate) const COMPLETIONS_ABOUT: &str = "Prints a shell completion script.";
pub(crate) const COMPLETIONS_EXAMPLES: &[&str] = &[
    "living-docs completions bash > /etc/bash_completion.d/living-docs",
    "living-docs completions zsh > \"${fpath[1]}/_living-docs\"",
];

/// The examples of the command addressed by `path` (the subcommand names
/// below the root; empty for the root, which has none).
pub(crate) fn examples_for(path: &[&str]) -> &'static [&'static str] {
    match path {
        ["new"] => NEW_EXAMPLES,
        ["set"] => SET_EXAMPLES,
        ["supersede"] => SUPERSEDE_EXAMPLES,
        ["index"] => INDEX_EXAMPLES,
        ["fmt"] => FMT_EXAMPLES,
        ["check"] => CHECK_EXAMPLES,
        ["read"] => READ_EXAMPLES,
        ["guide"] => GUIDE_EXAMPLES,
        ["install"] => INSTALL_EXAMPLES,
        ["install", "skills"] => INSTALL_SKILLS_EXAMPLES,
        ["install", "hooks"] => INSTALL_HOOKS_EXAMPLES,
        ["uninstall"] => UNINSTALL_EXAMPLES,
        ["uninstall", "hooks"] => UNINSTALL_HOOKS_EXAMPLES,
        ["completions"] => COMPLETIONS_EXAMPLES,
        _ => &[],
    }
}

const GROUP_AUTHORING: [(&str, &str); 5] = [
    ("new", NEW_ABOUT),
    ("set", SET_ABOUT),
    ("supersede", SUPERSEDE_ABOUT),
    ("index", INDEX_ABOUT),
    ("fmt", FMT_ABOUT),
];
const GROUP_GATE: [(&str, &str); 1] = [("check", CHECK_ABOUT)];
const GROUP_READING: [(&str, &str); 2] = [("read", READ_ABOUT), ("guide", GUIDE_ABOUT)];
const GROUP_DISTRIBUTION: [(&str, &str); 3] = [
    ("install", INSTALL_ABOUT),
    ("uninstall", UNINSTALL_ABOUT),
    ("completions", COMPLETIONS_ABOUT),
];

const GROUPS: [(&str, &[(&str, &str)]); 4] = [
    ("Authoring", &GROUP_AUTHORING),
    ("Gate", &GROUP_GATE),
    ("Reading", &GROUP_READING),
    ("Distribution", &GROUP_DISTRIBUTION),
];

/// The verbs under each root-help heading, in display order.
pub(crate) fn groups() -> Vec<(&'static str, Vec<&'static str>)> {
    GROUPS
        .iter()
        .map(|(heading, verbs)| (*heading, verbs.iter().map(|(name, _)| *name).collect()))
        .collect()
}

/// clap has no built-in way to group different subcommands under different
/// headings (`next_help_heading` groups args, not subcommands), so this
/// builds the root `--help`'s command listing by hand, and is wired in via
/// `Command::help_template` so `{options}`/`{after-help}` stay clap-generated
/// and in sync with the actual flags.
pub(crate) fn root_command_template() -> String {
    let commands: String = GROUPS
        .iter()
        .map(|(heading, verbs)| render_group(heading, verbs))
        .collect();
    format!(
        "{{about-with-newline}}\n{{usage-heading}} {{usage}}\n\n{commands}Options:\n{{options}}{{after-help}}"
    )
}

fn render_group(heading: &str, verbs: &[(&str, &str)]) -> String {
    let rows: String = verbs
        .iter()
        .map(|(name, about)| format!("  {name:<13}{about}\n"))
        .collect();
    format!("{heading}:\n{rows}\n")
}
