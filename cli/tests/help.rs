//! `--help`/`completions` coverage (ADR 0060): the root help lists exactly
//! the ten verbs plus `completions`, grouped under the four documented
//! headings, with `effective`/`skill` absent (they are hidden aliases); and
//! `completions <shell>` prints a non-empty script.

use std::collections::BTreeSet;
use std::process::{Command, Output};

#[path = "../src/args.rs"]
#[allow(dead_code)]
mod args;
#[path = "../src/output.rs"]
#[allow(dead_code)]
mod output;
#[path = "../src/skill.rs"]
#[allow(dead_code)]
mod skill;
#[path = "../src/skill_install.rs"]
#[allow(dead_code)]
mod skill_install;

const NON_VERB_SUBCOMMANDS: [&str; 1] = ["help"];

fn living_docs() -> Command {
    Command::new(env!("CARGO_BIN_EXE_living-docs"))
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

const VERBS: [&str; 11] = [
    "new",
    "set",
    "supersede",
    "index",
    "fmt",
    "check",
    "read",
    "guide",
    "install",
    "uninstall",
    "completions",
];

fn lists_as_a_command(help: &str, name: &str) -> bool {
    help.lines()
        .any(|line| line.trim_start().starts_with(&format!("{name} ")) || line.trim() == name)
}

fn non_hidden_verb_names() -> BTreeSet<String> {
    use clap::CommandFactory;
    args::Cli::command()
        .get_subcommands()
        .filter(|sub| !sub.is_hide_set())
        .map(|sub| sub.get_name().to_string())
        .filter(|name| !NON_VERB_SUBCOMMANDS.contains(&name.as_str()))
        .collect()
}

#[test]
fn root_help_lists_exactly_the_ten_verbs_plus_completions() {
    let output = living_docs()
        .arg("--help")
        .output()
        .expect("failed to run living-docs --help");
    let help = stdout_of(&output);
    assert!(output.status.success(), "got:\n{help}");

    for verb in VERBS {
        assert!(lists_as_a_command(&help, verb), "missing {verb}:\n{help}");
    }
    assert!(
        !lists_as_a_command(&help, "effective"),
        "hidden alias must not be listed:\n{help}"
    );
    assert!(
        !lists_as_a_command(&help, "skill"),
        "hidden alias must not be listed:\n{help}"
    );
}

#[test]
fn command_tree_non_hidden_subcommands_equal_the_hand_written_verb_list() {
    let expected: BTreeSet<String> = VERBS.iter().map(|verb| verb.to_string()).collect();
    let actual = non_hidden_verb_names();
    assert_eq!(
        actual, expected,
        "a verb added to Command but missing from VERBS (or vice versa) must fail here"
    );
}

#[test]
fn every_non_hidden_verb_help_documents_exit_codes() {
    for verb in non_hidden_verb_names() {
        let output = living_docs()
            .args([verb.as_str(), "--help"])
            .output()
            .unwrap_or_else(|_| panic!("failed to run living-docs {verb} --help"));
        let help = stdout_of(&output);
        assert!(output.status.success(), "{verb} --help got:\n{help}");
        assert!(
            help.contains("Exit codes:"),
            "{verb} --help missing the exit-code table:\n{help}"
        );
    }
}

#[test]
fn root_help_contains_the_four_group_headings() {
    let output = living_docs()
        .arg("--help")
        .output()
        .expect("failed to run living-docs --help");
    let help = stdout_of(&output);

    for heading in ["Authoring:", "Gate:", "Reading:", "Distribution:"] {
        assert!(help.contains(heading), "missing {heading}:\n{help}");
    }
}

#[test]
fn root_help_documents_the_exit_codes() {
    let output = living_docs()
        .arg("--help")
        .output()
        .expect("failed to run living-docs --help");
    let help = stdout_of(&output);

    assert!(help.contains("Exit codes:"), "got:\n{help}");
    assert!(help.contains('0'), "got:\n{help}");
    assert!(help.contains('1'), "got:\n{help}");
    assert!(help.contains('2'), "got:\n{help}");
}

#[test]
fn verb_help_carries_an_examples_section() {
    let output = living_docs()
        .args(["new", "--help"])
        .output()
        .expect("failed to run living-docs new --help");
    let help = stdout_of(&output);

    assert!(output.status.success(), "got:\n{help}");
    assert!(help.contains("Examples:"), "got:\n{help}");
    assert!(help.contains("living-docs new"), "got:\n{help}");
}

#[test]
fn completions_bash_prints_a_non_empty_script() {
    let output = living_docs()
        .args(["completions", "bash"])
        .output()
        .expect("failed to run living-docs completions bash");
    let stdout = stdout_of(&output);

    assert!(output.status.success(), "got:\n{stdout}");
    assert!(!stdout.trim().is_empty());
    assert!(stdout.contains("living_docs") || stdout.contains("living-docs"));
}

#[test]
fn completions_supports_zsh_and_fish() {
    for shell in ["zsh", "fish"] {
        let output = living_docs()
            .args(["completions", shell])
            .output()
            .unwrap_or_else(|_| panic!("failed to run living-docs completions {shell}"));
        let stdout = stdout_of(&output);
        assert!(output.status.success(), "{shell} got:\n{stdout}");
        assert!(!stdout.trim().is_empty(), "{shell} produced no output");
    }
}

fn registered_type_list() -> String {
    living_docs_core::doc_type::DOC_TYPES
        .iter()
        .map(|spec| spec.token)
        .collect::<Vec<_>>()
        .join(", ")
}

#[test]
fn type_positional_help_lists_the_registered_tokens_in_registry_order() {
    for verb in ["new", "index"] {
        let output = living_docs()
            .args([verb, "--help"])
            .output()
            .unwrap_or_else(|_| panic!("failed to run living-docs {verb} --help"));
        let help = stdout_of(&output);
        assert!(output.status.success(), "{verb} got:\n{help}");
        let expected = format!("one of: {}", registered_type_list());
        assert!(
            help.contains(&expected),
            "{verb} lacks '{expected}':\n{help}"
        );
    }
}

#[test]
fn set_help_shows_a_record_path_example() {
    let output = living_docs()
        .args(["set", "--help"])
        .output()
        .expect("failed to run living-docs set --help");
    let help = stdout_of(&output);
    assert!(
        help.contains("living-docs set issues/0049-flaky-upload-test.md status closed"),
        "got:\n{help}"
    );
}

#[test]
fn completions_rejects_an_unknown_shell_with_exit_code_two() {
    let output = living_docs()
        .args(["completions", "no-such-shell"])
        .output()
        .expect("failed to run living-docs completions no-such-shell");
    assert_eq!(output.status.code(), Some(2));
}
