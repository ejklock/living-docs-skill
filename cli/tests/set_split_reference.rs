//! A record reference split into a type word and a number (a type word, then a number, before the key)
//! stays a usage error, and the error teaches the one-token form.

use std::process::{Command, Output};

mod common;

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_living-docs"))
        .arg("--plain")
        .args(args)
        .output()
        .expect("failed to run living-docs")
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

fn hint_command(out: &Output) -> String {
    hint_line(out)
        .trim_start_matches("hint: write the reference as one token: living-docs ")
        .to_string()
}

fn any_record_contains(dir: &std::path::Path, needle: &str) -> bool {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| std::fs::read_to_string(entry.unwrap().path()).unwrap())
        .any(|text| text.contains(needle))
}

/// A temp root and its `my docs` bundle (a path with a space) holding one ADR.
fn bundle_with_first_decision(label: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let root = common::temp_bundle("split", label)
        .parent()
        .unwrap()
        .to_path_buf();
    let docs = root.join("my docs");
    assert!(common::run_new(&docs, "adr", "First Decision")
        .status
        .success());
    (root, docs)
}

#[test]
fn a_split_adr_reference_exits_2_and_shows_the_one_token_command() {
    let out = run(&["set", "adr", "0001", "status", "Accepted"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(
        stderr_of(&out).contains("set adr/0001 status Accepted"),
        "{}",
        stderr_of(&out)
    );
}

#[test]
fn a_split_issue_reference_keeps_the_number_as_given() {
    let out = run(&["set", "issue", "12", "owner", "x"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(
        stderr_of(&out).contains("set issue/12 owner x"),
        "{}",
        stderr_of(&out)
    );
}

#[test]
fn the_hint_keeps_a_docs_dir_flag_placed_before_set() {
    let out = run(&[
        "--docs-dir",
        "somewhere",
        "set",
        "adr",
        "0001",
        "status",
        "Accepted",
    ]);
    assert_eq!(out.status.code(), Some(2));
    assert!(
        stderr_of(&out).contains("--docs-dir somewhere set adr/0001 status Accepted"),
        "{}",
        stderr_of(&out)
    );
}

#[test]
fn an_unknown_type_token_gets_no_hint() {
    let out = run(&["set", "glossary", "0001", "status", "Accepted"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(!stderr_of(&out).contains("hint"), "{}", stderr_of(&out));
}

#[test]
fn a_non_number_second_token_gets_no_hint() {
    let out = run(&["set", "adr", "first", "status", "Accepted"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(!stderr_of(&out).contains("hint"), "{}", stderr_of(&out));
}

#[test]
fn another_subcommand_with_an_extra_argument_gets_no_hint() {
    let out = run(&["supersede", "adr", "0001", "adr/0002", "extra"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(!stderr_of(&out).contains("hint"), "{}", stderr_of(&out));
}

fn hint_line(out: &Output) -> String {
    stderr_of(out)
        .lines()
        .find(|line| line.starts_with("hint:"))
        .unwrap_or_default()
        .to_string()
}

#[test]
fn clap_error_text_precedes_the_hint() {
    let out = run(&["set", "adr", "0001", "status", "Accepted"]);
    let text = stderr_of(&out);
    let error = text.find("unexpected argument 'Accepted' found");
    let hint = text.find("hint:");
    assert!(error.is_some() && hint.is_some() && error < hint, "{text}");
}

#[test]
fn a_value_with_a_space_is_single_quoted_and_plain_words_stay_bare() {
    let out = run(&["set", "issue", "0070", "owner", "Jane Doe"]);
    assert!(
        stderr_of(&out).contains("set issue/0070 owner 'Jane Doe'"),
        "{}",
        stderr_of(&out)
    );
}

#[test]
fn a_quote_in_a_value_is_escaped_for_a_posix_shell() {
    let out = run(&["set", "issue", "0070", "owner", "it's"]);
    assert!(
        stderr_of(&out).contains(r"set issue/0070 owner 'it'\''s'"),
        "{}",
        stderr_of(&out)
    );
}

#[test]
fn the_printed_hint_runs_in_a_shell_with_the_same_effect_as_the_one_token_form() {
    let parent = common::temp_bundle("split", "shell")
        .parent()
        .unwrap()
        .to_path_buf();
    let docs = parent.join("my docs");
    assert!(common::run_new(&docs, "adr", "First Decision")
        .status
        .success());
    let split = run(&[
        "--docs-dir",
        docs.to_str().unwrap(),
        "set",
        "adr",
        "0001",
        "title",
        "It's Two Words",
    ]);
    let script = format!("\"$LD_BIN\" {}", hint_command(&split));
    let ran = Command::new("sh")
        .args(["-c", &script])
        .env("LD_BIN", env!("CARGO_BIN_EXE_living-docs"))
        .output()
        .expect("failed to run sh");
    assert_eq!(ran.status.code(), Some(0), "{}", stderr_of(&ran));
    let retitled = any_record_contains(&docs.join("adr"), "It's Two Words");
    assert!(retitled, "the shell-run hint did not retitle the record");
}

#[test]
fn help_requested_after_a_split_reference_prints_no_hint() {
    let out = run(&["set", "adr", "0001", "--help"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(!stderr_of(&out).contains("hint"), "{}", stderr_of(&out));
    assert!(!String::from_utf8_lossy(&out.stdout).contains("hint:"));
}

#[test]
fn a_double_dash_before_the_verb_gets_no_hint() {
    let out = run(&["--", "set", "adr", "0001", "status", "Accepted"]);
    assert!(!stderr_of(&out).contains("hint"), "{}", stderr_of(&out));
}

#[test]
fn a_split_reference_without_a_value_keeps_the_runtime_message_and_adds_the_hint() {
    let out = run(&["set", "adr", "0001", "status"]);
    let text = stderr_of(&out);
    assert_eq!(out.status.code(), Some(2));
    assert!(
        text.contains("'adr' is not a valid record number"),
        "{text}"
    );
    assert!(text.contains("set adr/0001 status"), "{text}");
}

#[test]
fn a_directory_name_token_with_a_from_conflict_gets_the_hint_with_from_kept() {
    let out = run(&["set", "issues", "0070", "description", "--from", "f"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(
        stderr_of(&out).contains("set issues/0070 description --from f"),
        "{}",
        stderr_of(&out)
    );
}

#[test]
fn a_runtime_hint_keeps_a_docs_dir_with_a_space_quoted() {
    let out = run(&["--docs-dir", "my docs", "set", "adr", "0001", "status"]);
    assert!(
        stderr_of(&out).contains("--docs-dir 'my docs' set adr/0001 status"),
        "{}",
        stderr_of(&out)
    );
}

#[test]
fn a_bare_type_with_a_non_number_key_gets_no_hint() {
    let out = run(&["set", "adr", "title", "x"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(!stderr_of(&out).contains("hint"), "{}", stderr_of(&out));
}

#[test]
fn an_unknown_reference_word_at_runtime_gets_no_hint() {
    let out = run(&["set", "nope", "0001", "status"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(!stderr_of(&out).contains("hint"), "{}", stderr_of(&out));
}

#[test]
fn a_successful_set_prints_no_hint() {
    let docs = common::temp_bundle("split", "success");
    assert!(common::run_new(&docs, "adr", "First Decision")
        .status
        .success());
    let out = run(&[
        "--docs-dir",
        docs.to_str().unwrap(),
        "set",
        "adr/0001",
        "status",
        "Accepted",
    ]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_of(&out));
    assert!(!stderr_of(&out).contains("hint"), "{}", stderr_of(&out));
}

#[test]
fn the_runtime_hint_run_by_a_shell_matches_the_one_token_form() {
    let (root, docs) = bundle_with_first_decision("runtime-shell");
    let source = root.join("text.md");
    std::fs::write(&source, "Replaced description.").unwrap();
    let split = run(&[
        "--docs-dir",
        docs.to_str().unwrap(),
        "set",
        "adr",
        "0001",
        "description",
        "--from",
        source.to_str().unwrap(),
    ]);
    assert_eq!(split.status.code(), Some(2));
    let command = hint_command(&split);
    let script = root.join("hint.sh");
    std::fs::write(&script, format!("\"$LD_BIN\" {command}\n")).unwrap();
    let ran = Command::new("sh")
        .arg(&script)
        .env("LD_BIN", env!("CARGO_BIN_EXE_living-docs"))
        .output()
        .expect("failed to run sh");
    assert_eq!(ran.status.code(), Some(0), "{}", stderr_of(&ran));
    let written = any_record_contains(&docs.join("adr"), "Replaced description.");
    assert!(written, "the shell-run hint did not write the description");
}
