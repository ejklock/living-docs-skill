//! `new`, `set` and `supersede` leave the index of every type they touched
//! current, so a CLI-authored bundle passes `check` with no `index` call.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

mod common;
use common::{living_docs, run_check, stdout_of, temp_bundle, write};

const ROOT_WITH_ISSUES: &str = "# Docs\n\n* [Issues](issues/index.md)\n";
const ROOT_WITH_ADRS: &str = "# Docs\n\n* [ADRs](adr/index.md)\n";
const ISSUE_BODY: &str =
    "\n## 0001. T\n\nSummary.\n\n### Scope\n\nAll of it.\n\n### Acceptance\n\n- done\n";
const ADR_BODY: &str =
    "\n# 0001. T\n\n## Context\n\nc.\n\n## Decision\n\nWe will d.\n\n## Consequences\n\nq.\n";

fn bundle(label: &str, root: &str) -> common::Scratch {
    let docs = temp_bundle("mutation-verbs-index", label);
    write(&docs, "index.md", root);
    docs
}

fn verb(docs: &Path, args: &[&str]) -> Output {
    living_docs()
        .arg("--docs-dir")
        .arg(docs)
        .args(args)
        .arg("--plain")
        .output()
        .expect("failed to run living-docs")
}

fn json_verb(docs: &Path, args: &[&str]) -> serde_json::Value {
    let output = living_docs()
        .arg("--docs-dir")
        .arg(docs)
        .args(args)
        .arg("--json")
        .output()
        .expect("failed to run living-docs");
    assert!(output.status.success(), "stderr: {}", stderr_of(&output));
    serde_json::from_str(stdout_of(&output).trim_end()).expect("json output")
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

fn created_path(output: &Output) -> PathBuf {
    assert!(output.status.success(), "stderr: {}", stderr_of(output));
    PathBuf::from(stdout_of(output).lines().next().unwrap())
}

/// Replaces the scaffold's description slot and body so the record passes `check`.
fn fill(record: &Path, body: &str) {
    let contents = fs::read_to_string(record).unwrap();
    let front_end = contents.match_indices("---\n").nth(1).unwrap().0 + 4;
    let front: String = contents[..front_end]
        .lines()
        .map(|line| {
            if line.starts_with("description:") {
                "description: d"
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(record, format!("{front}\n{body}")).unwrap();
}

fn new_filled(docs: &Path, doc_type: &str, title: &str, body: &str) -> PathBuf {
    let record = created_path(&verb(docs, &["new", doc_type, title]));
    fill(&record, body);
    record
}

fn index_stdout(docs: &Path, doc_type: &str) -> String {
    let output = verb(docs, &["index", doc_type]);
    assert!(output.status.success());
    stdout_of(&output)
}

fn assert_check_passes(docs: &Path) {
    let output = run_check(docs);
    assert!(output.status.success(), "check:\n{}", stdout_of(&output));
}

#[test]
fn new_issue_then_check_passes_and_the_index_lists_the_record() {
    let docs = bundle("new-issue", ROOT_WITH_ISSUES);
    new_filled(&docs, "issue", "T", ISSUE_BODY);
    assert_check_passes(&docs);
    let index = fs::read_to_string(docs.join("issues/index.md")).unwrap();
    assert!(index.contains("[0001 — T](0001-t.md)"), "got:\n{index}");
}

#[test]
fn new_adr_then_check_passes_and_the_index_lists_the_record() {
    let docs = bundle("new-adr", ROOT_WITH_ADRS);
    new_filled(&docs, "adr", "T", ADR_BODY);
    assert_check_passes(&docs);
    let index = fs::read_to_string(docs.join("adr/index.md")).unwrap();
    assert!(index.contains("[0001 — T](0001-t.md)"), "got:\n{index}");
}

#[test]
fn new_constitution_writes_no_index_and_succeeds() {
    let docs = bundle("new-constitution", "# Docs\n");
    let output = verb(&docs, &["new", "constitution", "T", "--owner", "Tester"]);
    assert!(output.status.success(), "stderr: {}", stderr_of(&output));
    assert!(docs.join("constitution.md").is_file());
    assert!(!stderr_of(&output).contains("indexed"));
    let type_dirs = fs::read_dir(&docs)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .count();
    assert_eq!(type_dirs, 0, "no type directory is created");
}

#[test]
fn new_second_issue_leaves_the_index_command_nothing_to_write() {
    let docs = bundle("new-second", ROOT_WITH_ISSUES);
    new_filled(&docs, "issue", "First", ISSUE_BODY);
    new_filled(&docs, "issue", "Second", ISSUE_BODY);
    assert_eq!(index_stdout(&docs, "issue"), "");
}

#[test]
fn set_status_updates_the_index_row_without_an_index_call() {
    let docs = bundle("set-status", ROOT_WITH_ISSUES);
    new_filled(&docs, "issue", "T", ISSUE_BODY);
    let output = verb(&docs, &["set", "0001", "status", "closed"]);
    assert!(output.status.success(), "stderr: {}", stderr_of(&output));
    let index = fs::read_to_string(docs.join("issues/index.md")).unwrap();
    assert!(index.contains("(0001-t.md) - closed"), "got:\n{index}");
    assert_eq!(index_stdout(&docs, "issue"), "");
}

#[test]
fn set_title_relinks_the_index_row_to_the_renamed_file() {
    let docs = bundle("set-title", ROOT_WITH_ISSUES);
    new_filled(&docs, "issue", "T", ISSUE_BODY);
    let output = verb(&docs, &["set", "0001", "title", "Renamed"]);
    assert!(output.status.success(), "stderr: {}", stderr_of(&output));
    let index = fs::read_to_string(docs.join("issues/index.md")).unwrap();
    assert!(index.contains("0001-renamed.md"), "got:\n{index}");
    assert!(!index.contains("0001-t.md"), "got:\n{index}");
    assert_eq!(index_stdout(&docs, "issue"), "");
}

#[test]
fn supersede_updates_the_adr_index_without_an_index_call() {
    let docs = bundle("supersede", ROOT_WITH_ADRS);
    new_filled(&docs, "adr", "Old", ADR_BODY);
    new_filled(&docs, "adr", "New", ADR_BODY);
    let output = verb(&docs, &["supersede", "0001", "0002"]);
    assert!(output.status.success(), "stderr: {}", stderr_of(&output));
    let index = fs::read_to_string(docs.join("adr/index.md")).unwrap();
    assert!(index.contains("[0002 — New](0002-new.md)"), "got:\n{index}");
    let old_row = index.lines().find(|l| l.contains("0001-old.md")).unwrap();
    assert!(old_row.contains("Superseded"), "got: {old_row}");
    assert!(old_row.contains("0002"), "got: {old_row}");
    assert_eq!(index_stdout(&docs, "adr"), "");
}

#[test]
fn output_new_text_prints_the_record_then_the_instruction_and_stderr_names_the_index() {
    let docs = bundle("output-new-text", ROOT_WITH_ISSUES);
    let output = verb(&docs, &["new", "issue", "T"]);
    let stdout = stdout_of(&output);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 2, "got:\n{stdout}");
    assert!(lines[0].ends_with("issues/0001-t.md"));
    assert!(lines[1].starts_with("Write ONLY the body"));
    let expected = format!(
        "living-docs new: indexed {}",
        docs.join("issues/index.md").display()
    );
    assert_eq!(stderr_of(&output).matches(&expected).count(), 1);
}

#[test]
fn output_quiet_silences_the_indexed_line() {
    let docs = bundle("output-quiet", ROOT_WITH_ISSUES);
    let output = living_docs()
        .arg("--docs-dir")
        .arg(&docs)
        .args(["--quiet", "--plain", "new", "issue", "T"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(!stderr_of(&output).contains("indexed"));
}

#[test]
fn output_set_and_supersede_report_each_written_index_once() {
    let docs = bundle("output-others", ROOT_WITH_ADRS);
    new_filled(&docs, "adr", "Old", ADR_BODY);
    new_filled(&docs, "adr", "New", ADR_BODY);
    let superseded = verb(&docs, &["supersede", "0001", "0002"]);
    let line = format!("indexed {}", docs.join("adr/index.md").display());
    assert_eq!(stderr_of(&superseded).matches(&line).count(), 1);
}

#[test]
fn output_json_lists_the_written_index_and_an_empty_list_when_nothing_changed() {
    let docs = bundle("output-json", ROOT_WITH_ADRS);
    let created = json_verb(&docs, &["new", "adr", "Old"]);
    let index = docs.join("adr/index.md");
    assert_eq!(
        created["indexed"],
        serde_json::json!([index.to_str().unwrap()])
    );

    let unchanged = json_verb(&docs, &["set", "0001", "status", "Proposed"]);
    assert_eq!(unchanged["indexed"], serde_json::json!([]));
}

#[cfg(unix)]
#[test]
fn output_unwritable_index_keeps_the_record_and_exits_one_with_a_repair_hint() {
    use std::os::unix::fs::PermissionsExt;

    let docs = bundle("output-readonly", ROOT_WITH_ISSUES);
    new_filled(&docs, "issue", "First", ISSUE_BODY);
    let index = docs.join("issues/index.md");
    fs::set_permissions(&index, fs::Permissions::from_mode(0o444)).unwrap();

    let output = verb(&docs, &["new", "issue", "Second"]);
    fs::set_permissions(&index, fs::Permissions::from_mode(0o644)).unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert!(docs.join("issues/0002-second.md").is_file());
    assert!(stdout_of(&output).contains("issues/0002-second.md"));
    let stderr = stderr_of(&output);
    assert!(
        stderr.contains("record written; index not updated"),
        "got:\n{stderr}"
    );
    assert!(stderr.contains("living-docs index issue"), "got:\n{stderr}");
}
