//! A type token also accepts the type's directory name, and a record
//! reference also accepts a record path inside the bundle.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

mod common;
use common::{living_docs, run_new, run_supersede, temp_bundle};

const TOKENS: [&str; 6] = ["adr", "prd", "issue", "research", "view", "constitution"];

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

fn run_set(docs: &Path, reference: &str) -> Output {
    living_docs()
        .arg("--docs-dir")
        .arg(docs)
        .args(["set", reference, "status", "closed"])
        .output()
        .expect("failed to run living-docs set")
}

fn bundle_with_issue_0049(label: &str) -> (common::Scratch, PathBuf) {
    let docs = temp_bundle("reference-paths", label);
    assert!(run_new(&docs, "issue", "Flaky upload").status.success());
    assert!(run_new(&docs, "adr", "Cache").status.success());
    let issue = docs.join("issues/0049-flaky-upload.md");
    fs::rename(docs.join("issues/0001-flaky-upload.md"), &issue).unwrap();
    (docs, issue)
}

fn assert_closed(issue: &Path) {
    let contents = fs::read_to_string(issue).unwrap();
    assert!(contents.contains("status: closed"), "got:\n{contents}");
}

fn assert_refused_untouched(docs: &Path, issue: &Path, reference: &str, rule: &str) {
    let before = fs::read_to_string(issue).unwrap();
    let output = run_set(docs, reference);
    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(2), "{reference}: {stderr}");
    assert!(
        stderr.contains(rule),
        "{reference}: expected '{rule}' in {stderr}"
    );
    assert_eq!(fs::read_to_string(issue).unwrap(), before, "{reference}");
}

#[test]
fn alias_directory_name_creates_and_indexes_the_record() {
    let docs = temp_bundle("reference-paths", "alias-new");
    let created = run_new(&docs, "issues", "T");
    assert!(created.status.success(), "{}", stderr_of(&created));
    let record = fs::read_to_string(docs.join("issues/0001-t.md")).unwrap();
    assert!(record.contains("type: Issue"), "got:\n{record}");

    fs::remove_file(docs.join("issues/index.md")).unwrap();
    let indexed = living_docs()
        .arg("--docs-dir")
        .arg(&docs)
        .args(["index", "issues"])
        .output()
        .unwrap();
    assert!(indexed.status.success(), "{}", stderr_of(&indexed));
    assert!(fs::read_to_string(docs.join("issues/index.md"))
        .unwrap()
        .contains("0001-t.md"));
}

#[test]
fn alias_directory_name_qualifies_set_and_supersede_references() {
    let (docs, issue) = bundle_with_issue_0049("alias-ref");
    let set = run_set(&docs, "issues/0049");
    assert!(set.status.success(), "{}", stderr_of(&set));
    assert_closed(&issue);

    assert!(run_new(&docs, "issue", "Second").status.success());
    let superseded = run_supersede(&docs, "issues/0049", "issues/0050");
    assert!(superseded.status.success(), "{}", stderr_of(&superseded));
}

#[test]
fn alias_unknown_type_exits_two_naming_every_valid_token() {
    let docs = temp_bundle("reference-paths", "alias-unknown");
    let created = run_new(&docs, "bogus", "T");
    assert_eq!(created.status.code(), Some(2));
    let stderr = stderr_of(&created);
    for token in TOKENS {
        assert!(stderr.contains(token), "missing {token}: {stderr}");
    }

    let (docs, issue) = bundle_with_issue_0049("alias-unknown-ref");
    assert_refused_untouched(&docs, &issue, "bogus/0049", "adr, prd, issue, research");
}

#[test]
fn path_forms_each_close_the_same_record() {
    let (docs, issue) = bundle_with_issue_0049("path");
    let absolute = issue.to_str().unwrap().to_string();
    let docs_name = docs.to_str().unwrap();
    let repo_relative = format!("{docs_name}/issues/0049-flaky-upload.md");
    for reference in [
        "issues/0049-flaky-upload.md",
        repo_relative.as_str(),
        absolute.as_str(),
        "0049",
    ] {
        let reopened = fs::read_to_string(&issue)
            .unwrap()
            .replace("status: closed", "status: open");
        fs::write(&issue, reopened).unwrap();
        let output = run_set(&docs, reference);
        assert!(
            output.status.success(),
            "{reference}: {}",
            stderr_of(&output)
        );
        assert_closed(&issue);
    }
}

#[test]
fn path_relative_to_the_repo_root_resolves() {
    let (docs, issue) = bundle_with_issue_0049("path-repo");
    let bundle_name = docs.file_name().unwrap().to_str().unwrap();
    let reference = format!("{bundle_name}/issues/0049-flaky-upload.md");
    let output = living_docs()
        .current_dir(docs.parent().unwrap())
        .args([
            "--docs-dir",
            bundle_name,
            "set",
            reference.as_str(),
            "status",
            "closed",
        ])
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", stderr_of(&output));
    assert_closed(&issue);
}

#[test]
fn hostile_references_exit_two_naming_the_rule_and_write_nothing() {
    let (docs, issue) = bundle_with_issue_0049("hostile");
    let cases = [
        ("../elsewhere/issues/0049-x.md", ".."),
        ("/tmp/other/bundle/issues/0049-x.md", "outside the bundle"),
        ("notes/0049-x.md", "not a type directory"),
        ("issues/readme.md", "no record number"),
        ("adr/0049-x.md", "no record found"),
        ("issues/sub/0049-x.md", "DIR/NNNN"),
    ];
    for (reference, rule) in cases {
        assert_refused_untouched(&docs, &issue, reference, rule);
    }
}
