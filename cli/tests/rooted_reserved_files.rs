//! An `index.md` is read and written through the store rooted at the bundle:
//! one that resolves outside the bundle is refused by every verb that rewrites
//! an index and reported once by `check`, one that links inside keeps working.

#![cfg(unix)]

mod common;

use common::{index_bundle, living_docs, run_new, temp_bundle, write};
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Output;

const RULE: &str = "a record must lie inside the bundle";
const OUTSIDE: &str = "outside bytes\n";
fn issue(number: &str, title: &str) -> String {
    format!(
        "---\ntype: Issue\ntitle: {title}\ndescription: A record\nowner: Someone\nstatus: Open\ntimestamp: 2026-01-01T00:00:00Z\n---\n\n# {number}. {title}\n\n### Scope\n\nOne thing.\n\n### Acceptance\n\n- It works.\n"
    )
}

struct Fixture {
    docs: common::Scratch,
    outside: PathBuf,
    index: PathBuf,
}

fn bundle_with_two_issues(label: &str) -> common::Scratch {
    let docs = temp_bundle("rooted-reserved", label);
    write(&docs, "issues/0001-first.md", &issue("0001", "first"));
    write(&docs, "issues/0002-second.md", &issue("0002", "second"));
    index_bundle(&docs);
    docs
}

fn relink(index: &Path, target: &Path) {
    fs::remove_file(index).unwrap();
    symlink(target, index).unwrap();
}

fn linked_outside(label: &str, relative_index: &str) -> Fixture {
    let docs = bundle_with_two_issues(label);
    let outside = docs.parent().unwrap().join("outside.md");
    fs::write(&outside, OUTSIDE).unwrap();
    let index = docs.join(relative_index);
    relink(&index, &outside);
    Fixture {
        docs,
        outside,
        index,
    }
}

fn docs_cmd(docs: &Path, args: &[&str]) -> Output {
    living_docs()
        .arg("--docs-dir")
        .arg(docs)
        .args(args)
        .output()
        .unwrap()
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

fn assert_refused(output: &Output, fixture: &Fixture) {
    assert_refused_with(output, fixture, 2);
}

/// A write verb has already written its record when the index is refused, so
/// it keeps its exit 1 and names the index.
fn assert_refused_after_record(output: &Output, fixture: &Fixture) {
    assert_refused_with(output, fixture, 1);
}

fn assert_refused_with(output: &Output, fixture: &Fixture, code: i32) {
    let text = stderr_of(output);
    assert_eq!(output.status.code(), Some(code), "{text}");
    assert!(
        text.contains(&fixture.index.display().to_string()),
        "{text}"
    );
    assert_eq!(fs::read_to_string(&fixture.outside).unwrap(), OUTSIDE);
}

#[test]
fn manage_index_refuses_a_type_index_linked_outside_and_keeps_the_target() {
    let fixture = linked_outside("index-type", "issues/index.md");

    let output = docs_cmd(&fixture.docs, &["manage", "index", "--plain"]);

    assert_refused(&output, &fixture);
}

#[test]
fn write_new_refuses_a_type_index_linked_outside_and_keeps_the_target() {
    let fixture = linked_outside("new-type", "issues/index.md");

    let output = run_new(&fixture.docs, "issue", "Third");

    assert_refused_after_record(&output, &fixture);
    assert!(fixture.docs.join("issues/0003-third.md").is_file());
}

#[test]
fn write_set_refuses_a_type_index_linked_outside_and_keeps_the_target() {
    let fixture = linked_outside("set-type", "issues/index.md");

    let args = ["write", "set", "issues/0001", "status", "closed"];
    let output = docs_cmd(&fixture.docs, &args);

    assert_refused_after_record(&output, &fixture);
}

#[test]
fn write_supersede_refuses_a_type_index_linked_outside_and_keeps_the_target() {
    let fixture = linked_outside("supersede-type", "issues/index.md");

    let args = ["write", "supersede", "issues/0001", "issues/0002"];
    let output = docs_cmd(&fixture.docs, &args);

    assert_refused_after_record(&output, &fixture);
}

#[test]
fn manage_index_refuses_a_root_index_linked_outside_and_keeps_the_target() {
    let fixture = linked_outside("index-root", "index.md");

    let output = docs_cmd(&fixture.docs, &["manage", "index", "--plain"]);

    assert_refused(&output, &fixture);
}

#[test]
fn manage_index_refuses_a_dangling_type_index_link_and_creates_nothing_outside() {
    let docs = bundle_with_two_issues("dangling");
    let missing = docs.parent().unwrap().join("missing.md");
    let index = docs.join("issues/index.md");
    relink(&index, &missing);

    let output = docs_cmd(&docs, &["manage", "index", "--plain"]);

    let text = stderr_of(&output);
    assert_eq!(output.status.code(), Some(2), "{text}");
    assert!(text.contains(&index.display().to_string()), "{text}");
    assert!(!missing.exists());
}

#[test]
fn manage_index_regenerates_through_a_type_index_linked_inside_the_bundle() {
    let docs = bundle_with_two_issues("inside");
    let target = docs.join("issues-index-target.md");
    fs::write(&target, "stale\n").unwrap();
    relink(&docs.join("issues/index.md"), &target);

    let output = docs_cmd(&docs, &["manage", "index", "--plain"]);

    assert!(output.status.success(), "{}", stderr_of(&output));
    let regenerated = fs::read_to_string(&target).unwrap();
    assert!(regenerated.contains("0002-second.md"), "{regenerated}");
}

#[test]
fn write_new_creates_the_missing_type_index_and_root_index_in_a_fresh_bundle() {
    let docs = temp_bundle("rooted-reserved", "fresh");

    let output = run_new(&docs, "issue", "First");

    assert!(output.status.success(), "{}", stderr_of(&output));
    let type_index = fs::read_to_string(docs.join("issues/index.md")).unwrap();
    assert!(type_index.contains("0001-first.md"), "{type_index}");
    let root = fs::read_to_string(docs.join("index.md")).unwrap();
    assert!(root.contains("(issues/)"), "{root}");
}

fn check_json(docs: &Path) -> serde_json::Value {
    let output = living_docs()
        .args(["check", docs.to_str().unwrap(), "--json"])
        .output()
        .unwrap();
    serde_json::from_slice(&output.stdout).expect("check --json prints JSON")
}

#[test]
fn check_reports_a_type_index_linked_outside_as_exactly_one_violation() {
    let fixture = linked_outside("check-outside", "issues/index.md");

    let report = check_json(&fixture.docs);

    let violations = report["violations"].as_array().unwrap();
    let index = fixture.index.display().to_string();
    assert_eq!(violations.len(), 1, "{report}");
    assert!(violations[0].to_string().contains(&index), "{report}");
    assert!(violations[0].to_string().contains(RULE), "{report}");
}

#[test]
fn check_accepts_a_type_index_linked_inside_the_bundle() {
    let docs = bundle_with_two_issues("check-inside");
    let index = docs.join("issues/index.md");
    let target = docs.join("issues-index-target.md");
    fs::copy(&index, &target).unwrap();
    relink(&index, &target);

    let report = check_json(&docs);

    let named = index.display().to_string();
    let on_index: Vec<String> = report["violations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.to_string())
        .filter(|v| v.contains(&named))
        .collect();
    assert!(on_index.is_empty(), "{on_index:?}");
    let on_record = report["violations"].to_string();
    assert!(!on_record.contains("issues/000"), "{report}");
}

#[test]
fn check_reports_a_root_index_linked_outside_as_exactly_one_violation() {
    let fixture = linked_outside("check-root-outside", "index.md");

    let report = check_json(&fixture.docs);

    let violations = report["violations"].as_array().unwrap();
    assert_eq!(violations.len(), 1, "{report}");
    assert_eq!(
        violations[0]["file"],
        fixture.index.display().to_string(),
        "{report}"
    );
    assert!(violations[0].to_string().contains(RULE), "{report}");
}

#[test]
fn write_new_keeps_a_root_index_linked_outside_unchanged_and_names_it() {
    let fixture = linked_outside("new-root", "index.md");

    let output = run_new(&fixture.docs, "issue", "Third");

    assert_refused_after_record(&output, &fixture);
    assert!(fixture.docs.join("issues/0003-third.md").is_file());
}
