//! A type token typed in any ASCII case resolves to its registry row, and
//! everything the CLI writes from it carries the registry's canonical form.

use std::fs;
use std::path::Path;
use std::process::Output;

mod common;
use common::{living_docs, run_check, run_new, temp_bundle, Scratch};

fn run_write(docs: &Path, args: &[&str]) -> Output {
    living_docs()
        .args(["--docs-dir", docs.to_str().unwrap(), "write"])
        .args(args)
        .arg("--plain")
        .output()
        .expect("failed to run living-docs write")
}

fn stable_text(docs: &Path, rel: &str) -> String {
    fs::read_to_string(docs.join(rel))
        .unwrap()
        .lines()
        .filter(|line| !line.starts_with("timestamp:"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn directories(docs: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(docs)
        .unwrap()
        .map(|entry| entry.unwrap())
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect();
    names.sort();
    names
}

/// A fresh scaffold still carries its template placeholders, so `check`
/// reports them; the report must be the same whichever spelling scaffolded it.
fn checked(docs: &Path) -> String {
    let output = run_check(docs);
    let text = String::from_utf8_lossy(&output.stdout).replace(docs.to_str().unwrap(), "<bundle>");
    format!("{:?}\n{text}", output.status.code())
}

fn scaffold(label: &str, adr_token: &str, issue_token: &str) -> Scratch {
    let docs = temp_bundle("type-token-case", label);
    assert!(run_new(&docs, adr_token, "Title").status.success());
    assert!(run_new(&docs, issue_token, "Title").status.success());
    docs
}

#[test]
fn new_with_a_capitalised_token_writes_the_canonical_directory_and_frontmatter() {
    let cased = scaffold("new-cased", "ADR", "Issue");
    let lower = scaffold("new-lower", "adr", "issue");

    assert_eq!(directories(&cased), directories(&lower));
    assert!(!directories(&cased)
        .iter()
        .any(|d| d == "ADR" || d == "Issue"));
    for rel in ["adr/0001-title.md", "issues/0001-title.md"] {
        assert_eq!(stable_text(&cased, rel), stable_text(&lower, rel));
    }
    for rel in ["adr/index.md", "issues/index.md"] {
        assert_eq!(stable_text(&cased, rel), stable_text(&lower, rel));
        assert!(stable_text(&cased, rel).contains("0001-title.md"));
    }
    assert_eq!(checked(&cased), checked(&lower));
    assert!(!checked(&cased).contains("ADR/") && !checked(&cased).contains("Issue/"));
}

#[test]
fn new_with_an_uppercase_directory_alias_resolves() {
    let docs = temp_bundle("type-token-case", "new-upper");
    assert!(run_new(&docs, "ISSUES", "Title").status.success());
    assert!(docs.join("issues/0001-title.md").is_file());
}

#[test]
fn set_with_a_cased_reference_changes_the_same_bytes_as_the_lowercase_form() {
    let cased = scaffold("set-cased", "adr", "issue");
    let lower = scaffold("set-lower", "adr", "issue");

    let adr = run_write(&cased, &["set", "ADR/0001", "status", "Accepted"]);
    assert!(adr.status.success(), "{adr:?}");
    let issue = run_write(&cased, &["set", "Issues/0001", "status", "closed"]);
    assert!(issue.status.success(), "{issue:?}");
    let adr = run_write(&lower, &["set", "adr/0001", "status", "Accepted"]);
    assert!(adr.status.success(), "{adr:?}");
    let issue = run_write(&lower, &["set", "issues/0001", "status", "closed"]);
    assert!(issue.status.success(), "{issue:?}");

    for rel in [
        "adr/0001-title.md",
        "issues/0001-title.md",
        "adr/index.md",
        "issues/index.md",
    ] {
        assert_eq!(stable_text(&cased, rel), stable_text(&lower, rel));
    }
    assert_eq!(directories(&cased), directories(&lower));
}

#[test]
fn read_records_links_to_accepts_a_cased_reference() {
    let docs = scaffold("links-to", "adr", "issue");
    let output = living_docs()
        .args(["--docs-dir", docs.to_str().unwrap()])
        .args(["read", "records", "--links-to", "ADR/0001", "--plain"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
}

#[test]
fn an_unknown_token_in_any_case_is_still_refused() {
    let docs = temp_bundle("type-token-case", "unknown");
    let new = run_new(&docs, "GLOSSARY", "x");
    assert!(!new.status.success());
    assert!(String::from_utf8_lossy(&new.stderr).contains("unsupported doc type 'GLOSSARY'"));

    let set = run_write(&docs, &["set", "Glossary/0001", "status", "x"]);
    assert!(!set.status.success());
    assert!(String::from_utf8_lossy(&set.stderr).contains("'Glossary' is not a valid type"));
}

#[test]
fn a_unicode_look_alike_token_is_refused_and_writes_nothing() {
    let docs = temp_bundle("type-token-case", "look-alike");
    let fullwidth = "\u{FF21}\u{FF24}\u{FF32}";
    for token in [fullwidth, "\u{0131}ssue"] {
        let new = run_new(&docs, token, "x");
        assert!(!new.status.success(), "{token}");
        let stderr = String::from_utf8_lossy(&new.stderr);
        assert!(
            stderr.contains(&format!("unsupported doc type '{token}'")),
            "{stderr}"
        );
    }
    let reference = format!("{fullwidth}/0001");
    let set = run_write(&docs, &["set", &reference, "status", "Accepted"]);
    assert!(!set.status.success());
    let stderr = String::from_utf8_lossy(&set.stderr);
    assert!(stderr.contains("is not a valid type"), "{stderr}");
    assert!(directories(&docs).is_empty());
}
