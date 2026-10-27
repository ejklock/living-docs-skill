//! JSON-shape coverage for the authoring verbs (ADR 0060): `index`, `fmt`,
//! `new`, `set`, `supersede`. Split out of `cli/tests/output_contract.rs` to
//! stay under the file-size cap.

use serde::Deserialize;
use std::path::Path;
use std::process::{Command, Output};

mod common;
use common::{stdout_of, temp_bundle, write};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_living-docs"))
        .args(args)
        .output()
        .unwrap_or_else(|_| panic!("failed to run living-docs {args:?}"))
}

#[derive(Deserialize)]
struct FilesJson {
    files: Vec<String>,
}

#[test]
fn index_json_matches_the_documented_shape() {
    let docs = temp_bundle("output-contract", "index");
    let docs: &Path = &docs;
    run(&[
        "--docs-dir",
        docs.to_str().unwrap(),
        "new",
        "adr",
        "First Decision",
        "--plain",
    ]);
    std::fs::remove_file(docs.join("adr/index.md")).unwrap();

    let output = run(&[
        "--docs-dir",
        docs.to_str().unwrap(),
        "index",
        "adr",
        "--json",
    ]);
    let stdout = stdout_of(&output);
    assert!(output.status.success(), "got:\n{stdout}");
    let parsed: FilesJson = serde_json::from_str(stdout.trim_end())
        .unwrap_or_else(|e| panic!("index --json emits valid JSON: {e}\ngot:\n{stdout}"));
    assert_eq!(parsed.files.len(), 1);
    assert!(
        parsed.files[0].ends_with("adr/index.md"),
        "got: {:?}",
        parsed.files
    );
    assert_eq!(stdout.trim_end().lines().count(), 1);
}

#[derive(Deserialize)]
struct PendingJson {
    pending: Vec<String>,
}

#[test]
fn fmt_json_matches_the_documented_shape() {
    let docs = temp_bundle("output-contract", "fmt");
    let docs: &Path = &docs;
    write(
        docs,
        "adr/0001-doc.md",
        "---\ntitle: Doc\ntype: ADR\ndescription: d\n---\n# Doc\n\nBody.\n",
    );

    let check_output = run(&["fmt", docs.to_str().unwrap(), "--check", "--json"]);
    let check_stdout = stdout_of(&check_output);
    let pending: PendingJson = serde_json::from_str(check_stdout.trim_end()).unwrap_or_else(|e| {
        panic!("fmt --check --json emits valid JSON: {e}\ngot:\n{check_stdout}")
    });
    assert_eq!(pending.pending.len(), 1);

    let write_output = run(&["fmt", docs.to_str().unwrap(), "--json"]);
    let write_stdout = stdout_of(&write_output);
    let files: FilesJson = serde_json::from_str(write_stdout.trim_end())
        .unwrap_or_else(|e| panic!("fmt --json emits valid JSON: {e}\ngot:\n{write_stdout}"));
    assert_eq!(files.files.len(), 1);
    assert_eq!(write_stdout.trim_end().lines().count(), 1);
}

fn assert_names_the_adr_index(indexed: &[String]) {
    assert_eq!(indexed.len(), 1);
    assert!(indexed[0].ends_with("adr/index.md"), "got: {indexed:?}");
}

#[derive(Deserialize)]
struct NewJson {
    path: String,
    instruction: String,
    indexed: Vec<String>,
}

#[test]
fn new_json_matches_the_documented_shape() {
    let docs = temp_bundle("output-contract", "new");
    let output = run(&[
        "--docs-dir",
        docs.to_str().unwrap(),
        "new",
        "adr",
        "A New Record",
        "--json",
    ]);
    let stdout = stdout_of(&output);
    assert!(output.status.success(), "got:\n{stdout}");
    let parsed: NewJson = serde_json::from_str(stdout.trim_end())
        .unwrap_or_else(|e| panic!("new --json emits valid JSON: {e}\ngot:\n{stdout}"));
    assert!(
        parsed.path.ends_with("adr/0001-a-new-record.md"),
        "got: {}",
        parsed.path
    );
    assert!(
        parsed
            .instruction
            .contains("Write ONLY the body below the closing"),
        "got: {}",
        parsed.instruction
    );
    assert_names_the_adr_index(&parsed.indexed);
    assert_eq!(stdout.trim_end().lines().count(), 1);
}

#[derive(Deserialize)]
struct SetJson {
    path: String,
    key: String,
    value: String,
    indexed: Vec<String>,
}

#[test]
fn set_json_matches_the_documented_shape() {
    let docs = temp_bundle("output-contract", "set");
    run(&[
        "--docs-dir",
        docs.to_str().unwrap(),
        "new",
        "adr",
        "Set Target",
        "--plain",
    ]);

    let output = run(&[
        "--docs-dir",
        docs.to_str().unwrap(),
        "set",
        "0001",
        "status",
        "Accepted",
        "--json",
    ]);
    let stdout = stdout_of(&output);
    assert!(output.status.success(), "got:\n{stdout}");
    let parsed: SetJson = serde_json::from_str(stdout.trim_end())
        .unwrap_or_else(|e| panic!("set --json emits valid JSON: {e}\ngot:\n{stdout}"));
    assert_eq!(parsed.key, "status");
    assert_eq!(parsed.value, "Accepted");
    assert!(
        parsed.path.ends_with("0001-set-target.md"),
        "got: {}",
        parsed.path
    );
    assert_names_the_adr_index(&parsed.indexed);
}

#[test]
fn set_section_json_reports_the_key_and_the_heading() {
    let docs = temp_bundle("output-contract", "set-section");
    let dir = docs.to_str().unwrap();
    new_adr(dir, "Section Target");
    let from = docs.parent().unwrap().join("text.md");
    std::fs::write(&from, "Some text.\n").unwrap();
    let output = run(&[
        "--docs-dir",
        dir,
        "set",
        "0001",
        "section",
        "Notes",
        "--from",
        from.to_str().unwrap(),
        "--json",
    ]);
    let stdout = stdout_of(&output);
    assert!(output.status.success(), "got:\n{stdout}");
    let parsed: SetJson = serde_json::from_str(stdout.trim_end())
        .unwrap_or_else(|e| panic!("set section --json emits valid JSON: {e}\ngot:\n{stdout}"));
    assert_eq!(parsed.key, "section");
    assert_eq!(parsed.value, "Notes");
    assert!(parsed.indexed.is_empty(), "got: {:?}", parsed.indexed);
}

#[derive(Deserialize)]
struct SupersedeJson {
    old: String,
    new: String,
    indexed: Vec<String>,
}

fn new_adr(docs: &str, title: &str) {
    run(&["--docs-dir", docs, "new", "adr", title, "--plain"]);
}

#[test]
fn supersede_json_matches_the_documented_shape() {
    let docs = temp_bundle("output-contract", "supersede");
    let docs = docs.to_str().unwrap();
    new_adr(docs, "Old Decision");
    new_adr(docs, "New Decision");

    let output = run(&["--docs-dir", docs, "supersede", "0001", "0002", "--json"]);
    let stdout = stdout_of(&output);
    assert!(output.status.success(), "got:\n{stdout}");
    let parsed: SupersedeJson = serde_json::from_str(stdout.trim_end())
        .unwrap_or_else(|e| panic!("supersede --json emits valid JSON: {e}\ngot:\n{stdout}"));
    assert!(
        parsed.old.ends_with("0001-old-decision.md"),
        "got: {}",
        parsed.old
    );
    assert!(
        parsed.new.ends_with("0002-new-decision.md"),
        "got: {}",
        parsed.new
    );
    assert_names_the_adr_index(&parsed.indexed);
}
