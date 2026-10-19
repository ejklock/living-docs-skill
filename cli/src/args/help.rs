//! Help text for every verb (ADR 0060): a one-line `about` plus a real
//! `Examples:` block, kept as `const`s so `args.rs`/`args/sub.rs` stay
//! attribute-only and readable. Docblocks document the code; these strings
//! are what `--help` actually prints.

pub(crate) const EXIT_CODES: &str = "Exit codes:\n  0  success\n  1  a gate or a verb's own check failed\n  2  invalid usage or a missing input";

/// Appends the [`EXIT_CODES`] table to a verb's `Examples:` block, so every
/// verb's `--help` ends with the same documented table as the root help
/// (ADR 0060).
pub(crate) fn with_exit_codes(examples: &str) -> String {
    format!("{examples}\n\n{EXIT_CODES}")
}

/// Renders the registered record types for a `TYPE` positional's help, read
/// from the registry at runtime so a new row appears without a help edit.
pub(crate) fn doc_type_help() -> String {
    let tokens = living_docs_core::doc_type::DOC_TYPES
        .iter()
        .map(|spec| spec.token)
        .collect::<Vec<_>>();
    format!("one of: {}", tokens.join(", "))
}

pub(crate) const NEW_ABOUT: &str =
    "Scaffolds a new record with CLI-owned numbering, frontmatter, and heading.";
pub(crate) const NEW_EXAMPLES: &str = "Examples:\n  living-docs new adr \"Cache invalidation strategy\"\n  living-docs new issue \"Flaky upload test\" --owner alice@example.com\n  living-docs new issue \"Flaky upload test\" --body body.md\n  cat body.md | living-docs new issue \"Flaky upload test\" --body -";

pub(crate) const SET_ABOUT: &str =
    "Sets one CLI-owned frontmatter field (status, description, owner, title) or replaces a body section.";
pub(crate) const SET_EXAMPLES: &str = "Examples:\n  living-docs set 0012 status Accepted\n  living-docs set adr/0012 owner carol@example.com\n  living-docs set issues/0049-flaky-upload-test.md status closed\n  living-docs set adr/0012 title \"Cache invalidation is write-through\"\n  living-docs set 0012 description --from description.txt\n  living-docs set issue/0049 section Outcome --from outcome.md\n  echo closed | living-docs set 0049 status --from -";

pub(crate) const SUPERSEDE_ABOUT: &str =
    "Retires an old record in favor of a new one, wiring both link directions.";
pub(crate) const SUPERSEDE_EXAMPLES: &str = "Examples:\n  living-docs supersede 0011 0012";

pub(crate) const INDEX_ABOUT: &str = "Rebuilds a doc type's index.md from the records on disk.";
pub(crate) const INDEX_EXAMPLES: &str = "Examples:\n  living-docs index adr\n  living-docs index";

pub(crate) const FMT_ABOUT: &str =
    "Canonicalizes a record's frontmatter in place, or reports what's pending.";
pub(crate) const FMT_EXAMPLES: &str =
    "Examples:\n  living-docs fmt docs\n  living-docs fmt --check docs";

pub(crate) const CHECK_ABOUT: &str = "Validates the bundle's mechanical invariants — the doc-gate.";
pub(crate) const CHECK_EXAMPLES: &str =
    "Examples:\n  living-docs check docs\n  living-docs check --require-owner docs\n  living-docs check --changed-files docs/adr/0001-x.md -- docs";

pub(crate) const READ_ABOUT: &str =
    "Prints the agent-facing in-force view: active records, supersede chains collapsed.";
pub(crate) const READ_EXAMPLES: &str =
    "Examples:\n  living-docs read --topic caching\n  living-docs read --full";

pub(crate) const GUIDE_ABOUT: &str =
    "Serves the embedded skill corpus: topics, templates, and full SKILL.md bodies.";
pub(crate) const GUIDE_EXAMPLES: &str = "Examples:\n  living-docs guide adr\n  living-docs guide --list\n  living-docs guide conformance --skill okf-knowledge-format";

pub(crate) const INSTALL_ABOUT: &str =
    "Places the embedded skill corpus or the enforcement hooks into a project.";
pub(crate) const INSTALL_EXAMPLES: &str =
    "Examples:\n  living-docs install skills --harness claude\n  living-docs install hooks";

pub(crate) const UNINSTALL_ABOUT: &str = "Removes what `install hooks` placed.";
pub(crate) const UNINSTALL_EXAMPLES: &str = "Examples:\n  living-docs uninstall hooks";

pub(crate) const INSTALL_SKILLS_ABOUT: &str =
    "Places the embedded skill directories into a harness's skills directory.";
pub(crate) const INSTALL_SKILLS_EXAMPLES: &str = "Examples:\n  living-docs install skills --harness claude\n  living-docs install skills --project";

pub(crate) const INSTALL_HOOKS_ABOUT: &str =
    "Writes the session-teaching hook and the pre-commit doc-gate into a project.";
pub(crate) const INSTALL_HOOKS_EXAMPLES: &str =
    "Examples:\n  living-docs install hooks\n  living-docs install hooks --dir ./my-project";

pub(crate) const UNINSTALL_HOOKS_ABOUT: &str = "Removes what `install hooks` placed.";
pub(crate) const UNINSTALL_HOOKS_EXAMPLES: &str =
    "Examples:\n  living-docs uninstall hooks\n  living-docs uninstall hooks --dir ./my-project";

pub(crate) const COMPLETIONS_ABOUT: &str = "Prints a shell completion script.";
pub(crate) const COMPLETIONS_EXAMPLES: &str = "Examples:\n  living-docs completions bash > /etc/bash_completion.d/living-docs\n  living-docs completions zsh > \"${fpath[1]}/_living-docs\"";

/// clap has no built-in way to group different subcommands under different
/// headings (`next_help_heading` groups args, not subcommands — see
/// `Command::subcommand_help_heading`'s docs). This builds the root
/// `--help`'s command listing by hand, grouped as Authoring, Gate, Reading,
/// and Distribution (ADR 0060), and is wired in via `Command::help_template`
/// so `{options}`/`{after-help}` stay clap-generated and in sync with the
/// actual flags.
pub(crate) fn root_command_template() -> String {
    let groups = [
        ("Authoring", GROUP_AUTHORING.as_slice()),
        ("Gate", GROUP_GATE.as_slice()),
        ("Reading", GROUP_READING.as_slice()),
        ("Distribution", GROUP_DISTRIBUTION.as_slice()),
    ];
    let commands: String = groups
        .into_iter()
        .map(|(heading, verbs)| render_group(heading, verbs))
        .collect();
    format!(
        "{{about-with-newline}}\n{{usage-heading}} {{usage}}\n\n{commands}Options:\n{{options}}{{after-help}}"
    )
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

fn render_group(heading: &str, verbs: &[(&str, &str)]) -> String {
    let rows: String = verbs
        .iter()
        .map(|(name, about)| format!("  {name:<13}{about}\n"))
        .collect();
    format!("{heading}:\n{rows}\n")
}
