//! A record path names exactly that file, and a `TYPE/NNNN` reference that
//! matches two records of one type refuses with every candidate named.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

mod common;
use common::{index_bundle, living_docs, run_supersede, temp_bundle, write};

type Tree = BTreeMap<PathBuf, Vec<u8>>;

fn issue(number: &str, title: &str) -> String {
    format!(
        "---\ntype: Issue\ntitle: {title}\ndescription: d\nstatus: open\ntimestamp: 2026-10-24T10:00:00Z\n---\n\n## {number}. {title}\n\nBody.\n"
    )
}

fn duplicate_bundle(label: &str) -> PathBuf {
    let docs = temp_bundle("reference-duplicates", label);
    write(&docs, "issues/0007-a.md", &issue("0007", "a"));
    write(&docs, "issues/0007-b.md", &issue("0007", "b"));
    write(&docs, "issues/0009-x.md", &issue("0009", "x"));
    write(&docs, "issues/0010-y.md", &issue("0010", "y"));
    index_bundle(&docs);
    docs
}

fn snapshot(root: &Path) -> Tree {
    let mut files = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else if let Ok(bytes) = fs::read(&path) {
                files.insert(path, bytes);
            }
        }
    }
    files
}

fn set(docs: &Path, reference: &str) -> Output {
    living_docs()
        .arg("--docs-dir")
        .arg(docs)
        .args(["set", reference, "description", "x"])
        .output()
        .unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

fn assert_refused_unchanged(before: &Tree, docs: &Path, output: &Output, needle: &str) {
    let text = stderr(output);
    assert_eq!(output.status.code(), Some(2), "{text}");
    assert!(text.contains(needle), "expected '{needle}' in: {text}");
    assert_eq!(&snapshot(docs), before, "the tree must not change");
}

#[test]
fn a_record_path_edits_exactly_that_file_in_both_path_forms() {
    for form in ["relative", "absolute"] {
        let docs = duplicate_bundle(form);
        let sibling = fs::read(docs.join("issues/0007-a.md")).unwrap();
        let reference = match form {
            "relative" => "issues/0007-b.md".to_string(),
            _ => docs.join("issues/0007-b.md").display().to_string(),
        };
        let output = set(&docs, &reference);
        assert!(output.status.success(), "{form}: {}", stderr(&output));
        let edited = fs::read_to_string(docs.join("issues/0007-b.md")).unwrap();
        assert!(edited.contains("description: x"), "{form}: {edited}");
        assert_eq!(fs::read(docs.join("issues/0007-a.md")).unwrap(), sibling);
    }
}

#[test]
fn a_qualified_reference_matching_two_records_refuses_naming_both_and_the_path_form() {
    let docs = duplicate_bundle("ambiguous-set");
    let before = snapshot(&docs);
    let output = set(&docs, "issues/0007");
    assert_refused_unchanged(&before, &docs, &output, "0007-a.md");
    let text = stderr(&output);
    assert!(text.contains("0007-b.md"), "{text}");
    assert!(text.contains("by its path"), "{text}");
}

#[test]
fn supersede_refuses_a_qualified_reference_matching_two_records() {
    let docs = duplicate_bundle("ambiguous-supersede");
    let before = snapshot(&docs);
    let output = run_supersede(&docs, "issues/0007", "issues/0009");
    assert_refused_unchanged(&before, &docs, &output, "0007-b.md");
}

#[test]
fn a_path_to_a_missing_sibling_is_not_found_although_the_number_exists() {
    let docs = duplicate_bundle("missing-sibling");
    let before = snapshot(&docs);
    let output = set(&docs, "issues/0007-c.md");
    assert_refused_unchanged(&before, &docs, &output, "no record found");
}

#[test]
fn a_path_outside_the_bundle_is_refused() {
    let docs = duplicate_bundle("outside");
    let before = snapshot(&docs);
    let outside = docs.parent().unwrap().join("elsewhere/issues/0007-a.md");
    let output = set(&docs, &outside.display().to_string());
    assert_refused_unchanged(&before, &docs, &output, "outside the bundle");
}

fn lone_bundle(label: &str) -> PathBuf {
    let docs = temp_bundle("reference-duplicates", label);
    write(&docs, "issues/0007-a.md", &issue("0007", "a"));
    write(&docs, "issues/0009-x.md", &issue("0009", "x"));
    index_bundle(&docs);
    docs
}

#[test]
fn a_path_naming_no_file_is_refused_on_every_key_and_in_supersede() {
    let docs = lone_bundle("typo-path");
    let before = snapshot(&docs);
    let typo = "issues/0007-typo.md";
    let outputs = [
        set(&docs, typo),
        living_docs()
            .arg("--docs-dir")
            .arg(&docs)
            .args(["set", typo, "number", "next"])
            .output()
            .unwrap(),
        run_supersede(&docs, typo, "issues/0009-x.md"),
        run_supersede(&docs, "issues/0009-x.md", typo),
    ];
    for output in &outputs {
        assert_refused_unchanged(&before, &docs, output, "0007-typo.md");
        assert!(stderr(output).contains("0007-a.md"), "{}", stderr(output));
    }
}

#[test]
fn a_path_naming_no_file_whose_number_no_record_carries_is_not_found() {
    let docs = lone_bundle("typo-unknown-number");
    let before = snapshot(&docs);
    let output = set(&docs, "issues/0042-typo.md");
    assert_refused_unchanged(&before, &docs, &output, "no record found");
}

#[cfg(unix)]
#[test]
fn a_symlink_pointing_outside_the_bundle_is_refused_and_its_target_kept() {
    let docs = duplicate_bundle("symlink");
    let target = docs.parent().unwrap().join("outside.md");
    fs::write(&target, issue("0008", "outside")).unwrap();
    std::os::unix::fs::symlink(&target, docs.join("issues/0008-link.md")).unwrap();
    let before = fs::read(&target).unwrap();
    let output = set(&docs, "issues/0008-link.md");
    assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
    assert_eq!(fs::read(&target).unwrap(), before);
}

#[test]
fn a_single_candidate_resolves_by_number_and_a_path_naming_another_file_is_refused() {
    let docs = duplicate_bundle("single");
    let output = set(&docs, "issues/0009");
    assert!(output.status.success(), "{}", stderr(&output));
    let before = snapshot(&docs);
    let renamed = set(&docs, "issues/0009-old-slug.md");
    assert_refused_unchanged(&before, &docs, &renamed, "0009-x.md");
    let edited = fs::read_to_string(docs.join("issues/0009-x.md")).unwrap();
    assert!(edited.contains("description: x"), "{edited}");
    let superseded = run_supersede(&docs, "issues/0009", "issues/0010");
    assert!(superseded.status.success(), "{}", stderr(&superseded));
}
