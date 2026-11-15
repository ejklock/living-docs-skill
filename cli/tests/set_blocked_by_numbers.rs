mod common;

use common::{living_docs, run_new, temp_bundle};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

const TRACKER_ID: &str = "ext-9";

fn set(docs: &Path, args: &[&str]) -> Output {
    living_docs()
        .args(["--docs-dir", docs.to_str().unwrap(), "write", "set"])
        .args(args)
        .arg("--plain")
        .output()
        .unwrap()
}

fn issue_bundle(label: &str) -> (common::Scratch, PathBuf) {
    let docs = temp_bundle("set-blocked-by-numbers", label);
    assert!(run_new(&docs, "issue", "T").status.success());
    let path = docs.join("issues/0001-t.md");
    (docs, path)
}

fn lines_of(path: &Path, key: &str) -> Vec<String> {
    let prefix = format!("{key}:");
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|line| line.starts_with(&prefix))
        .map(str::to_string)
        .collect()
}

fn assert_refused_unchanged(docs: &Path, path: &Path, args: &[&str]) -> String {
    let before = fs::read_to_string(path).unwrap();
    let out = set(docs, args);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert_eq!(fs::read_to_string(path).unwrap(), before);
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn blocked_by_accepts_decimal_numbers_trimmed_and_keeps_leading_zeros() {
    let (docs, path) = issue_bundle("numbers");
    let out = set(&docs, &["0001", "blocked_by", "12, 0012"]);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(lines_of(&path, "blocked_by"), ["blocked_by: [12, 0012]"]);
    assert!(set(&docs, &["0001", "blocked_by", " 7 "]).status.success());
    assert_eq!(lines_of(&path, "blocked_by"), ["blocked_by: [7]"]);
    assert!(set(&docs, &["0001", "blocked_by", ""]).status.success());
    assert_eq!(lines_of(&path, "blocked_by"), ["blocked_by: []"]);
}

#[test]
fn blocked_by_refuses_a_tracker_identifier_naming_it_and_leaves_the_record() {
    let (docs, path) = issue_bundle("tracker-id");
    let value = format!("12, {TRACKER_ID}");
    let stderr = assert_refused_unchanged(&docs, &path, &["0001", "blocked_by", &value]);
    assert!(stderr.contains(TRACKER_ID), "got: {stderr}");
}

#[test]
fn blocked_by_refuses_signs_fractions_and_non_ascii_digits() {
    let (docs, path) = issue_bundle("non-decimal");
    for value in ["-3", "1.5", "+4", "\u{661}\u{662}"] {
        let list = format!("12, {value}");
        let stderr = assert_refused_unchanged(&docs, &path, &["0001", "blocked_by", &list]);
        assert!(stderr.contains(value), "{value}: {stderr}");
    }
}

#[test]
fn labels_still_accept_a_tracker_identifier() {
    let (docs, path) = issue_bundle("labels-tracker-id");
    let value = format!("a, {TRACKER_ID}");
    assert!(set(&docs, &["0001", "labels", &value]).status.success());
    let expected = format!("labels: [a, {TRACKER_ID}]");
    assert_eq!(lines_of(&path, "labels"), [expected]);
}
