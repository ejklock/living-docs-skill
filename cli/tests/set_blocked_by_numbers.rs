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
fn blocked_by_accepts_text_items_beside_numbers_and_reads_back_both() {
    let (docs, path) = issue_bundle("text-items");
    let out = set(&docs, &["0001", "blocked_by", "ref ab-77, 12"]);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(
        lines_of(&path, "blocked_by"),
        ["blocked_by: [ref ab-77, 12]"]
    );
}

#[test]
fn blocked_by_accepts_a_tracker_identifier_a_sign_and_non_ascii_digits() {
    let (docs, path) = issue_bundle("non-decimal");
    for value in [TRACKER_ID, "-3", "\u{661}\u{662}"] {
        let list = format!("12, {value}");
        let out = set(&docs, &["0001", "blocked_by", &list]);
        assert!(out.status.success(), "{value}: {out:?}");
        let lines = lines_of(&path, "blocked_by");
        assert!(
            lines.len() == 1 && lines[0].contains(value),
            "{value}: {lines:?}"
        );
    }
}

#[test]
fn blocked_by_still_refuses_brackets_and_leaves_the_record() {
    let (docs, path) = issue_bundle("brackets");
    let stderr = assert_refused_unchanged(&docs, &path, &["0001", "blocked_by", "12, [x]"]);
    assert!(stderr.contains("brackets"), "got: {stderr}");
}

#[test]
fn labels_still_accept_a_tracker_identifier() {
    let (docs, path) = issue_bundle("labels-tracker-id");
    let value = format!("a, {TRACKER_ID}");
    assert!(set(&docs, &["0001", "labels", &value]).status.success());
    let expected = format!("labels: [a, {TRACKER_ID}]");
    assert_eq!(lines_of(&path, "labels"), [expected]);
}

#[test]
fn blocked_by_refuses_an_item_the_record_would_read_back_as_another_value_naming_it() {
    let (docs, path) = issue_bundle("rewritten");
    for value in ["+4", "1.5", "true"] {
        let list = format!("12, {value}");
        let stderr = assert_refused_unchanged(&docs, &path, &["0001", "blocked_by", &list]);
        assert!(
            stderr.contains(&format!("'{value}' would read back")),
            "{value}: {stderr}"
        );
    }
}

#[test]
fn blocked_by_accepts_text_beside_a_number_after_the_early_item_check() {
    let (docs, _) = issue_bundle("adr-and-number");
    let out = set(&docs, &["0001", "blocked_by", "see the design note, 12"]);
    assert!(out.status.success(), "{out:?}");
}

#[test]
fn blocked_by_accepts_items_the_record_quotes_and_keeps_them_as_text() {
    let (docs, path) = issue_bundle("quoted-items");
    let out = set(&docs, &["0001", "blocked_by", "#21, 12"]);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(lines_of(&path, "blocked_by"), ["blocked_by: [\"#21\", 12]"]);
    let out = set(&docs, &["0001", "blocked_by", "12, -0"]);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(lines_of(&path, "blocked_by"), ["blocked_by: [12, \"-0\"]"]);
}

#[test]
fn blocked_by_refuses_null_hex_and_exponent_items_naming_them() {
    let (docs, path) = issue_bundle("other-types");
    for value in ["null", "0x10", "1e3"] {
        let list = format!("12, {value}");
        let stderr = assert_refused_unchanged(&docs, &path, &["0001", "blocked_by", &list]);
        assert!(
            stderr.contains(&format!("'{value}' would read back")),
            "{value}: {stderr}"
        );
    }
}
