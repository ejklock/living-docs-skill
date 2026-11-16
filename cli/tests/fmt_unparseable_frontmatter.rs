//! A record whose frontmatter block exists but does not parse as a YAML
//! mapping is refused by `fmt` and `set` with its bytes untouched, and
//! `check` reports it as one violation instead of a record missing its keys.

use std::fs;
use std::path::Path;
use std::process::Output;

mod common;
use common::{index_bundle, living_docs, run_check, stdout_of, temp_bundle, write, Scratch};

const BROKEN: &str = "issues/0001-broken.md";
const DRIFTED: &str = "issues/0002-drifted.md";
const REFUSAL_MESSAGE: &str = "frontmatter does not parse as a YAML mapping";

fn issue_text(front: &str) -> String {
    format!("---\n{front}---\n\n## 0001. Broken\n\n### Scope\n\ns.\n")
}

fn broken_record() -> String {
    issue_text("type: Issue\ntitle: Broken\nstatus: open\naliases: [a\n")
}

fn drifted_record() -> String {
    "---\nstatus: open\ntitle: Drifted\ntype: Issue\ndescription: d.\nowner: o\ntimestamp: 2026-01-01T00:00:00Z\n---\n\n## 0002. Drifted\n\n### Scope\n\ns.\n".to_string()
}

fn canonical_record() -> String {
    "---\ntype: Issue\ntitle: Drifted\ndescription: d.\nowner: o\nstatus: open\ntimestamp: 2026-01-01T00:00:00Z\n---\n\n## 0002. Drifted\n\n### Scope\n\ns.\n".to_string()
}

fn bundle_with(label: &str, broken: &str, other: &str) -> Scratch {
    let bundle = temp_bundle("fmt-unparseable", label);
    write(&bundle, BROKEN, broken);
    write(&bundle, DRIFTED, other);
    bundle
}

fn fmt(bundle: &Path) -> Output {
    living_docs()
        .args(["manage", "fmt", bundle.to_str().unwrap(), "--plain"])
        .output()
        .unwrap()
}

fn read(bundle: &Path, rel: &str) -> String {
    fs::read_to_string(bundle.join(rel)).unwrap()
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn fmt_refuses_an_unclosed_flow_list_and_still_writes_the_other_record() {
    let bundle = bundle_with("unclosed", &broken_record(), &drifted_record());
    let out = fmt(&bundle);
    assert_eq!(out.status.code(), Some(2), "{}", stderr_of(&out));
    assert!(
        stderr_of(&out).contains("0001-broken.md"),
        "{}",
        stderr_of(&out)
    );
    assert_eq!(read(&bundle, BROKEN), broken_record());
    assert_ne!(read(&bundle, DRIFTED), drifted_record());
}

#[test]
fn fmt_refuses_a_top_level_scalar_block() {
    let text = issue_text("just text\n");
    let bundle = bundle_with("scalar", &text, &drifted_record());
    let out = fmt(&bundle);
    assert_eq!(out.status.code(), Some(2), "{}", stderr_of(&out));
    assert!(
        stderr_of(&out).contains("0001-broken.md"),
        "{}",
        stderr_of(&out)
    );
    assert_eq!(read(&bundle, BROKEN), text);
}

#[test]
fn fmt_leaves_a_file_with_no_frontmatter_block_untouched() {
    let prose = "# Notes\n\nno frontmatter here.\n";
    let bundle = temp_bundle("fmt-unparseable", "prose");
    write(&bundle, "notes.md", prose);
    let out = fmt(&bundle);
    assert!(out.status.success(), "{}", stderr_of(&out));
    assert_eq!(read(&bundle, "notes.md"), prose);
}

#[test]
fn fmt_does_not_refuse_a_blank_frontmatter_block() {
    let blank = "---\n\n---\n\n## 0001. Blank\n\n### Scope\n\ns.\n";
    let bundle = bundle_with("blank", blank, &canonical_record());
    let out = fmt(&bundle);
    assert!(out.status.success(), "{}", stderr_of(&out));
    assert!(
        !stderr_of(&out).contains(REFUSAL_MESSAGE),
        "{}",
        stderr_of(&out)
    );
}

#[test]
fn fmt_exits_zero_on_a_clean_bundle() {
    let bundle = temp_bundle("fmt-unparseable", "clean");
    write(&bundle, DRIFTED, &drifted_record());
    assert!(fmt(&bundle).status.success());
    assert!(fmt(&bundle).status.success());
}

fn set(bundle: &Path, args: &[&str]) -> Output {
    living_docs()
        .args(["--docs-dir", bundle.to_str().unwrap(), "write", "set"])
        .args(args)
        .arg("--plain")
        .output()
        .unwrap()
}

fn assert_refused_unchanged(bundle: &Path, out: &Output) {
    assert_eq!(out.status.code(), Some(2), "{}", stderr_of(out));
    assert!(
        stderr_of(out).contains("0001-broken.md"),
        "{}",
        stderr_of(out)
    );
    assert!(
        stderr_of(out).contains(REFUSAL_MESSAGE),
        "{}",
        stderr_of(out)
    );
    assert_eq!(read(bundle, BROKEN), broken_record());
}

#[test]
fn set_refuses_a_status_change_on_an_unparseable_record() {
    let bundle = bundle_with("set-status", &broken_record(), &drifted_record());
    let out = set(&bundle, &["issue/0001", "status", "closed"]);
    assert_refused_unchanged(&bundle, &out);
}

#[test]
fn set_refuses_a_section_replacement_on_an_unparseable_record() {
    let bundle = bundle_with("set-section", &broken_record(), &drifted_record());
    write(&bundle, "v.txt", "new text\n");
    let file = bundle.join("v.txt");
    let args = [
        "issue/0001",
        "section",
        "Scope",
        "--from",
        file.to_str().unwrap(),
    ];
    let out = set(&bundle, &args);
    assert_refused_unchanged(&bundle, &out);
}

#[test]
fn set_still_changes_a_parseable_record() {
    let bundle = bundle_with("set-ok", &broken_record(), &drifted_record());
    let out = set(&bundle, &["issue/0002", "status", "closed"]);
    assert!(out.status.success(), "{}", stderr_of(&out));
    assert!(read(&bundle, DRIFTED).contains("status: closed"));
}

fn indexed_bundle(label: &str) -> Scratch {
    let bundle = bundle_with(label, &broken_record(), &canonical_record());
    index_bundle(&bundle);
    bundle
}

#[test]
fn check_reports_one_violation_for_an_unparseable_record() {
    let bundle = indexed_bundle("check-one");
    let out = run_check(&bundle);
    assert_eq!(out.status.code(), Some(1), "{}", stdout_of(&out));
    let shown = stdout_of(&out);
    let lines: Vec<&str> = shown
        .lines()
        .filter(|l| l.contains("0001-broken.md"))
        .collect();
    assert_eq!(lines.len(), 1, "{shown}");
    assert!(lines[0].contains("frontmatter does not parse"), "{shown}");
    for missing in ["'type'", "title", "status"] {
        assert!(!lines[0].contains(missing), "{shown}");
    }
}

#[test]
fn check_json_carries_the_same_single_violation() {
    let bundle = indexed_bundle("check-json");
    let out = living_docs()
        .args(["check", bundle.to_str().unwrap(), "--json"])
        .output()
        .unwrap();
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let broken: Vec<&serde_json::Value> = report["violations"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v["file"].as_str().unwrap().contains("0001-broken.md"))
        .collect();
    assert_eq!(broken.len(), 1, "{report}");
    let message = broken[0]["message"].as_str().unwrap();
    assert!(message.contains("frontmatter does not parse"), "{message}");
    assert_eq!(report["ok"], false);
}
