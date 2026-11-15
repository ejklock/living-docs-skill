//! Help text of the install and uninstall verbs and their nested subcommands.

pub(crate) const INSTALL_ABOUT: &str =
    "Places skills or hooks into a project; mermaid goes to the user data directory.";
pub(crate) const INSTALL_EXAMPLES: &[&str] = &[
    "living-docs manage install skills --harness claude",
    "living-docs manage install hooks",
    "living-docs manage install mermaid",
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
    "Writes the session-teaching hook, the decision-prompt Stop hook and the pre-commit doc-gate into a project.";
pub(crate) const INSTALL_HOOKS_EXAMPLES: &[&str] = &[
    "living-docs manage install hooks",
    "living-docs manage install hooks --dir ./my-project",
];

pub(crate) const INSTALL_MERMAID_ABOUT: &str =
    "Downloads the pinned mermaid.js with curl, verifies its SHA-256, and places it in the user data directory.";
pub(crate) const INSTALL_MERMAID_EXAMPLES: &[&str] = &[
    "living-docs manage install mermaid",
    "living-docs manage install mermaid --dry-run",
];

pub(crate) const UNINSTALL_HOOKS_ABOUT: &str = "Removes what `manage install hooks` placed.";
pub(crate) const UNINSTALL_HOOKS_EXAMPLES: &[&str] = &[
    "living-docs manage uninstall hooks",
    "living-docs manage uninstall hooks --dir ./my-project",
];
