mod common;

use common::{living_docs, run_check, run_new, stdout_of, temp_bundle};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

fn set(docs: &Path, args: &[&str]) -> Output {
    living_docs()
        .args(["--docs-dir", docs.to_str().unwrap(), "write", "set"])
        .args(args)
        .arg("--plain")
        .output()
        .unwrap()
}

fn issue_bundle(label: &str) -> (common::Scratch, PathBuf) {
    let docs = temp_bundle("set-labels-numeric", label);
    for _ in 0..3 {
        assert!(run_new(&docs, "issue", "T").status.success());
    }
    let path = docs.join("issues/0001-t.md");
    (docs, path)
}

fn line_of(path: &Path, key: &str) -> String {
    let prefix = format!("{key}:");
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .find(|line| line.starts_with(&prefix))
        .unwrap()
        .to_string()
}

fn fmt_check(docs: &Path) -> Output {
    living_docs()
        .args(["manage", "fmt", docs.to_str().unwrap(), "--check"])
        .output()
        .unwrap()
}

#[test]
fn a_numeric_label_is_written_quoted_and_stays_clean() {
    let (docs, path) = issue_bundle("single");
    let before = stdout_of(&run_check(&docs));
    let out = set(&docs, &["0001", "labels", "12"]);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(line_of(&path, "labels"), "labels: [\"12\"]");
    assert_eq!(stdout_of(&run_check(&docs)), before);
    assert!(fmt_check(&docs).status.success());
}

#[test]
fn only_the_integer_items_of_a_label_list_are_quoted() {
    let (docs, path) = issue_bundle("mixed");
    let out = set(&docs, &["0001", "labels", "x, 12, 0005"]);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(line_of(&path, "labels"), "labels: [x, \"12\", 0005]");
    assert!(fmt_check(&docs).status.success());
}

#[test]
fn a_signed_or_oversized_numeric_label_is_quoted() {
    let (docs, path) = issue_bundle("signed");
    assert!(set(&docs, &["0001", "labels", "-3"]).status.success());
    assert_eq!(line_of(&path, "labels"), "labels: [\"-3\"]");
    let big = "18446744073709551616";
    assert!(set(&docs, &["0001", "labels", big]).status.success());
    assert_eq!(line_of(&path, "labels"), format!("labels: [\"{big}\"]"));
    assert!(fmt_check(&docs).status.success());
}

#[test]
fn blocked_by_keeps_an_issue_number_plain() {
    let (docs, path) = issue_bundle("blocked");
    assert!(set(&docs, &["0001", "blocked_by", "12"]).status.success());
    assert_eq!(line_of(&path, "blocked_by"), "blocked_by: [12]");
    let out = set(&docs, &["0001", "blocked_by", "0002, ref ab-77"]);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(
        line_of(&path, "blocked_by"),
        "blocked_by: [0002, ref ab-77]"
    );
}

#[test]
fn a_hand_written_numeric_label_is_left_as_written() {
    let (docs, path) = issue_bundle("hand-written");
    assert!(set(&docs, &["0001", "labels", "a"]).status.success());
    let text = fs::read_to_string(&path).unwrap();
    let edited = text.replace("labels: [a]", "labels: [12]");
    assert_ne!(text, edited, "no labels line to edit");
    fs::write(&path, &edited).unwrap();
    let before = fmt_check(&docs).status.code();
    assert!(set(&docs, &["0001", "tracker", "7"]).status.success());
    assert_eq!(line_of(&path, "labels"), "labels: [12]");
    assert_eq!(fmt_check(&docs).status.code(), before);
}
