use std::process::Command;

mod common;
use common::Scratch;

fn living_docs() -> Command {
    Command::new(env!("CARGO_BIN_EXE_living-docs"))
}

fn temp_dir(label: &str) -> Scratch {
    Scratch::new(&format!("living-docs-cli-test-{label}"))
}

/// ADR 0019, AC ac-s4-3: the root `--help` about text carries the same
/// body-only instruction `new` prints after a created path.
#[test]
fn root_help_carries_the_body_only_instruction() {
    let output = living_docs()
        .args(["--plain", "--help"])
        .output()
        .expect("failed to run living-docs --help");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    assert!(stdout.contains("Write ONLY the body below the closing"));
    assert!(stdout.contains("living-docs set"));
    assert!(stdout.contains("supersede"));
    assert!(stdout.contains("index"));
}

#[test]
fn unknown_subcommand_exits_with_code_2() {
    let output = living_docs()
        .arg("bogus")
        .output()
        .expect("failed to run living-docs");

    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn missing_required_argument_exits_with_code_2() {
    let output = living_docs()
        .arg("new")
        .output()
        .expect("failed to run living-docs");

    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn check_on_missing_bundle_exits_with_code_2() {
    // `check` takes its own positional bundle argument (default `docs`), not the
    // global `--docs-dir` — see cli/tests/check_core.rs for its full behavior.
    let docs = temp_dir("check-missing-bundle");
    let missing = docs.join("nope");

    let output = living_docs()
        .args(["check", missing.to_str().unwrap()])
        .output()
        .expect("failed to run living-docs");

    assert_eq!(output.status.code(), Some(2));
}
